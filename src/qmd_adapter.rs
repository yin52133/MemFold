use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::Result;
use crate::state::schema;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QmdRecord {
    pub doc_id: String,
    pub source_type: String,
    pub relative_path: String,
    pub pointer: String,
    pub summary: String,
    pub status: String,
    pub scope_type: String,
    pub scope_id: String,
    pub updated_at: String,
}

pub fn sync_scope(config: &MemfoldConfig, scope: &ScopeRef) -> Result<usize> {
    let stable_records = build_stable_records(config, scope)?;
    let evidence_records = build_evidence_records(config, scope)?;
    let archive_records = build_archive_records(config, scope)?;

    write_collection(config, scope, "stable", &stable_records)?;
    write_collection(config, scope, "evidence", &evidence_records)?;
    write_collection(config, scope, "archive", &archive_records)?;

    Ok(stable_records.len() + evidence_records.len() + archive_records.len())
}

pub fn load_scope_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let mut records = Vec::new();
    for source in ["stable", "evidence", "archive"] {
        let path = collection_file_path(config, scope, source);
        if !path.exists() {
            continue;
        }

        let contents = fs::read_to_string(path)?;
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            records.push(serde_json::from_str::<QmdRecord>(line)?);
        }
    }

    Ok(records)
}

fn build_stable_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let conn = open_connection(config)?;
    let stable_dir = config
        .project_root(scope)
        .join("stable");

    let mut files = Vec::new();
    if stable_dir.exists() {
        for entry in fs::read_dir(stable_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }
    files.sort();

    let mut records = Vec::new();
    for file in files {
        let relative_path = make_relative_path(config, &file);
        for item in parse_markdown_items(&file)? {
            if item.status != "stable" {
                continue;
            }

            let (doc_id, updated_at) = conn
                .query_row(
                    "SELECT id, updated_at
                     FROM memory_items
                     WHERE scope_type = ?1
                       AND scope_id = ?2
                       AND item_key = ?3
                       AND deleted_at IS NULL
                     ORDER BY revision DESC, updated_at DESC
                     LIMIT 1",
                    params![scope.scope_type.as_str(), &scope.scope_id, &item.item_key],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .unwrap_or_else(|| {
                    (
                        format!("stable:{}:{}", scope.scope_key(), item.item_key),
                        now_string(),
                    )
                });

            records.push(QmdRecord {
                doc_id,
                source_type: "stable".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#{}", item.item_key),
                summary: item.summary,
                status: item.status,
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at,
            });
        }
    }

    Ok(records)
}

fn build_evidence_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let sessions_dir = config
        .project_root(scope)
        .join("sessions");
    let mut records = Vec::new();

    if !sessions_dir.exists() {
        return Ok(records);
    }

    let mut session_dirs = Vec::new();
    for entry in fs::read_dir(sessions_dir)? {
        session_dirs.push(entry?.path());
    }
    session_dirs.sort();

    for session_dir in session_dirs {
        let file = session_dir.join("evidence.jsonl");
        if !file.exists() {
            continue;
        }

        let relative_path = make_relative_path(config, &file);
        let contents = fs::read_to_string(&file)?;
        for (idx, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let status = if value["promotable"].as_bool().unwrap_or(false) {
                "promotable"
            } else {
                "recorded"
            };
            records.push(QmdRecord {
                doc_id: value["evidence_id"].as_str().unwrap_or("").to_string(),
                source_type: "evidence".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#{}", idx + 1),
                summary: value["summary"].as_str().unwrap_or("").to_string(),
                status: status.to_string(),
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at: value["created_at"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    Ok(records)
}

fn build_archive_records(config: &MemfoldConfig, scope: &ScopeRef) -> Result<Vec<QmdRecord>> {
    let archive_dir = config
        .project_root(scope)
        .join("archive");
    let mut records = Vec::new();

    if !archive_dir.exists() {
        return Ok(records);
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(archive_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for file in files {
        let relative_path = make_relative_path(config, &file);
        for (idx, entry) in parse_archive_entries(&file)?.into_iter().enumerate() {
            records.push(QmdRecord {
                doc_id: format!("archive:{}:{}", scope.scope_key(), idx + 1),
                source_type: "archive".to_string(),
                relative_path: relative_path.clone(),
                pointer: format!("{relative_path}#entry-{}", idx + 1),
                summary: entry.summary,
                status: "archived".to_string(),
                scope_type: scope.scope_type.as_str().to_string(),
                scope_id: scope.scope_id.clone(),
                updated_at: entry.updated_at,
            });
        }
    }

    Ok(records)
}

fn write_collection(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    source_type: &str,
    records: &[QmdRecord],
) -> Result<()> {
    let path = collection_file_path(config, scope, source_type);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut contents = String::new();
    for record in records {
        contents.push_str(&serde_json::to_string(record)?);
        contents.push('\n');
    }
    fs::write(path, contents)?;
    Ok(())
}

fn collection_file_path(config: &MemfoldConfig, scope: &ScopeRef, source_type: &str) -> PathBuf {
    config
        .root
        .join("qmd")
        .join("collections")
        .join(scope.scope_dir_fragment())
        .join(format!("{source_type}.jsonl"))
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

fn make_relative_path(config: &MemfoldConfig, absolute: &Path) -> String {
    absolute
        .strip_prefix(&config.root)
        .unwrap_or(absolute)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Debug)]
struct ParsedMarkdownItem {
    item_key: String,
    status: String,
    summary: String,
}

fn parse_markdown_items(path: &Path) -> Result<Vec<ParsedMarkdownItem>> {
    let contents = fs::read_to_string(path)?;
    let mut blocks = Vec::new();
    let mut current_key: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("## item_key:") {
            if let Some(item_key) = current_key.take() {
                blocks.push((item_key, std::mem::take(&mut current_lines)));
            }
            current_key = Some(rest.trim().to_string());
        } else if current_key.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(item_key) = current_key.take() {
        blocks.push((item_key, current_lines));
    }

    let mut items = Vec::new();
    for (item_key, lines) in blocks {
        let mut status = None;
        let mut body = Vec::new();
        let mut in_body = false;
        for line in lines {
            if !in_body {
                if line.trim().is_empty() {
                    in_body = true;
                    continue;
                }

                if let Some((key, value)) = line.split_once(':') {
                    if key.trim() == "status" {
                        status = Some(value.trim().to_string());
                    }
                    continue;
                }
                in_body = true;
            }
            if in_body {
                body.push(line);
            }
        }

        items.push(ParsedMarkdownItem {
            item_key,
            status: status.unwrap_or_else(|| "candidate".to_string()),
            summary: body.join("\n").trim().to_string(),
        });
    }

    Ok(items)
}

#[derive(Debug)]
struct ParsedArchiveEntry {
    summary: String,
    updated_at: String,
}

fn parse_archive_entries(path: &Path) -> Result<Vec<ParsedArchiveEntry>> {
    let contents = fs::read_to_string(path)?;
    let mut entries = Vec::new();

    for line in contents.lines() {
        if !line.starts_with("## ") {
            continue;
        }

        let updated_at = line
            .trim_start_matches("## ")
            .split(' ')
            .next()
            .unwrap_or_default()
            .to_string();
        let summary = line
            .split("] ")
            .nth(1)
            .unwrap_or_default()
            .trim()
            .to_string();

        entries.push(ParsedArchiveEntry { summary, updated_at });
    }

    Ok(entries)
}

fn now_string() -> String {
    OffsetDateTime::now_utc().to_string()
}
