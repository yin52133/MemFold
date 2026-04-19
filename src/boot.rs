use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::config::MemfoldConfig;
use crate::domain::{Mode, ScopeRef, ScopeType};
use crate::error::{Error, Result};
use crate::memory_fs::markdown;
use crate::memory_fs::paths::{project_bundle_path, stable_dir};
use crate::mutations::MutationStore;
use crate::state::schema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleItem {
    pub item_key: String,
    pub text: String,
    pub source_item_id: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleLoad {
    pub items: Vec<BundleItem>,
    pub total_tokens_estimate: usize,
    pub degraded: bool,
}

#[derive(Debug, Clone)]
struct ParsedStableItem {
    item_key: String,
    text: String,
    file_path: String,
    status: String,
    autoload: String,
}

pub fn compile_scope_bundle(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    budget: usize,
) -> Result<BundleLoad> {
    let mut conn = open_state_conn(config)?;
    let compiled_at = now_rfc3339()?;
    let stable_items = read_stable_items(config, scope)?;

    let mut items = Vec::new();
    let mut seen_texts = HashSet::new();
    let mut total_tokens_estimate = 0usize;
    let mut degraded = false;

    for item in stable_items {
        if item.status != "stable" || !autoload_matches(scope.scope_type, &item.autoload) {
            continue;
        }
        let text_key = canonical_text_key(&item.text);
        if !seen_texts.insert(text_key) {
            continue;
        }

        let token_estimate = estimate_tokens(&item.text);
        if total_tokens_estimate.saturating_add(token_estimate) > budget {
            degraded = true;
            break;
        }

        let source_item_id = resolve_source_item_id(&conn, scope, &item)?;
        items.push(BundleItem {
            item_key: item.item_key,
            text: item.text,
            source_item_id,
            token_estimate,
        });
        total_tokens_estimate += token_estimate;
    }

    write_bundle_file(config, scope, &items, &compiled_at, total_tokens_estimate)?;
    refresh_boot_entries(&mut conn, scope, &items, &compiled_at)?;

    Ok(BundleLoad {
        items,
        total_tokens_estimate,
        degraded,
    })
}

pub fn load_startup_bundle(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    mode: Mode,
    budget: Option<usize>,
) -> Result<BundleLoad> {
    if mode == Mode::Sterile {
        return Ok(BundleLoad {
            items: Vec::new(),
            total_tokens_estimate: 0,
            degraded: false,
        });
    }

    let limit = budget.unwrap_or(usize::MAX);
    let mut items = Vec::new();
    let mut total_tokens_estimate = 0usize;
    let mut degraded = false;

    let mut bundle_scopes = Vec::new();
    match mode {
        Mode::Sterile => {}
        Mode::Fresh => bundle_scopes.push(user_bundle_scope(scope)),
        Mode::Normal => {
            bundle_scopes.push(user_bundle_scope(scope));
            if scope.scope_type == ScopeType::Project {
                bundle_scopes.push(scope.clone());
            }
        }
    }

    let mutation_store = MutationStore::new(config.clone());
    for bundle_scope in &bundle_scopes {
        if mutation_store.has_unstable_scope_mutations(bundle_scope)? {
            return Err(Error::UnstableMutation(bundle_scope.scope_key()));
        }
    }

    for bundle_scope in bundle_scopes {
        let compiled = compile_scope_bundle(config, &bundle_scope, usize::MAX)?;
        for item in compiled.items {
            if total_tokens_estimate.saturating_add(item.token_estimate) > limit {
                degraded = true;
                return Ok(BundleLoad {
                    items,
                    total_tokens_estimate,
                    degraded,
                });
            }

            total_tokens_estimate += item.token_estimate;
            items.push(item);
        }
    }

    Ok(BundleLoad {
        items,
        total_tokens_estimate,
        degraded,
    })
}

fn open_state_conn(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

fn now_rfc3339() -> Result<String> {
    Ok(
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .expect("RFC3339 formatting should not fail"),
    )
}

fn read_stable_items(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<ParsedStableItem>> {
    let stable_dir = stable_dir(config, scope);
    let mut files = Vec::new();

    if stable_dir.exists() {
        for entry in fs::read_dir(&stable_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    files.sort();

    let mut items = Vec::new();
    for file_path in files {
        items.extend(parse_stable_file(&file_path)?);
    }

    Ok(items)
}

fn parse_stable_file(file_path: &Path) -> Result<Vec<ParsedStableItem>> {
    let contents = fs::read_to_string(file_path)?;
    let mut blocks: Vec<(String, Vec<String>)> = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("## item_key:") {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            current = Some((rest.trim().to_string(), Vec::new()));
            continue;
        }

        if let Some((_, ref mut lines)) = current {
            lines.push(line.to_string());
        }
    }

    if let Some(block) = current.take() {
        blocks.push(block);
    }

    let mut items = Vec::new();
    for (item_key, lines) in blocks {
        if let Some(item) = parse_item_block(file_path, &item_key, &lines) {
            items.push(item);
        }
    }

    Ok(items)
}

fn parse_item_block(file_path: &Path, item_key: &str, lines: &[String]) -> Option<ParsedStableItem> {
    let mut status = None::<String>;
    let mut autoload = None::<String>;
    let mut body_lines = Vec::new();
    let mut in_body = false;

    for line in lines {
        if !in_body {
            if line.trim().is_empty() {
                in_body = true;
                continue;
            }

            if let Some((key, value)) = line.split_once(':') {
                match key.trim() {
                    "status" => status = Some(value.trim().to_string()),
                    "autoload" => autoload = Some(value.trim().to_string()),
                    _ => {}
                }
                continue;
            }

            in_body = true;
        }

        if in_body {
            body_lines.push(line.clone());
        }
    }

    while body_lines.first().is_some_and(|line| line.trim().is_empty()) {
        body_lines.remove(0);
    }
    while body_lines.last().is_some_and(|line| line.trim().is_empty()) {
        body_lines.pop();
    }

    Some(ParsedStableItem {
        item_key: item_key.trim().to_string(),
        text: body_lines.join("\n"),
        file_path: file_path.display().to_string(),
        status: status?,
        autoload: autoload?,
    })
}

fn autoload_matches(scope_type: ScopeType, autoload: &str) -> bool {
    match scope_type {
        ScopeType::User => autoload == "boot_user",
        ScopeType::Project => autoload == "boot_project",
    }
}

fn estimate_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

fn canonical_text_key(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_lowercase()
}

fn resolve_source_item_id(
    conn: &Connection,
    scope: &ScopeRef,
    item: &ParsedStableItem,
) -> Result<String> {
    let source_item_id = conn
        .query_row(
            "SELECT id
             FROM memory_items
             WHERE scope_type = ?1
               AND scope_id = ?2
               AND item_key = ?3
               AND deleted_at IS NULL
             ORDER BY revision DESC, updated_at DESC
             LIMIT 1",
            params![scope.scope_type.as_str(), &scope.scope_id, item.item_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .unwrap_or_else(|| {
            format!(
                "stable:{}:{}:{}",
                scope.scope_key(),
                item.file_path,
                item.item_key
            )
        });

    Ok(source_item_id)
}

fn write_bundle_file(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    items: &[BundleItem],
    compiled_at: &str,
    total_tokens_estimate: usize,
) -> Result<()> {
    let path = project_bundle_path(config, scope);
    MemfoldConfig::ensure_parent_dir(&path)?;

    let mut contents = format!(
        "<!-- compiled: {} | scope: {}/{} | tokens: {} -->\n\n",
        compiled_at,
        scope.scope_type.as_str(),
        &scope.scope_id,
        total_tokens_estimate
    );

    for item in items {
        contents.push_str(&format!("## {}\n", item.item_key));
        if item.text.is_empty() {
            contents.push('\n');
        } else {
            contents.push_str(&item.text);
            contents.push_str("\n\n");
        }
    }

    markdown::write_markdown_file(&path, &contents)?;
    Ok(())
}

fn refresh_boot_entries(
    conn: &mut Connection,
    scope: &ScopeRef,
    items: &[BundleItem],
    compiled_at: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "DELETE FROM boot_entries WHERE scope_type = ?1 AND scope_id = ?2",
        params![scope.scope_type.as_str(), &scope.scope_id],
    )?;

    for item in items {
        let entry_id = format!("boot:{}:{}", scope.scope_key(), item.item_key);
        tx.execute(
            "INSERT INTO boot_entries (
                id, scope_type, scope_id, item_key, source_item_id, text,
                token_estimate, compiled_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                entry_id,
                scope.scope_type.as_str(),
                &scope.scope_id,
                item.item_key,
                item.source_item_id,
                item.text,
                item.token_estimate as i64,
                compiled_at,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn user_bundle_scope(scope: &ScopeRef) -> ScopeRef {
    if scope.scope_type == ScopeType::User {
        scope.clone()
    } else {
        ScopeRef::new(ScopeType::User, "default")
            .expect("default user scope id should be valid")
    }
}
