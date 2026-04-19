use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::boot::compile_scope_bundle;
use crate::config::MemfoldConfig;
use crate::domain::{ScopeRef, ScopeType};
use crate::error::Result;
use crate::noise::is_memory_noise;
use crate::qmd_adapter::sync_scope;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepairResult {
    pub repaired: bool,
    pub rebuilt_records: usize,
}

pub fn run_repair(config: &MemfoldConfig, scope: Option<&ScopeRef>) -> Result<RepairResult> {
    canonicalize_project_storage(config)?;
    let conn = open_connection(config)?;
    canonicalize_project_scope_rows(&conn)?;
    clean_dirty_memory_content(config, scope)?;
    let scopes = match scope {
        Some(scope) => vec![scope.clone()],
        None => discover_scopes(config)?,
    };

    let mut rebuilt_records = 0usize;
    for scope in scopes {
        clear_scope_projections(&conn, &scope)?;
        rebuilt_records += rebuild_memory_items(config, &conn, &scope)?;
        rebuilt_records += rebuild_evidence_items(config, &conn, &scope)?;
        rebuilt_records += rebuild_trace_archives(config, &conn, &scope)?;
        compile_scope_bundle(config, &scope, 900)?;
        sync_scope(config, &scope)?;
    }

    Ok(RepairResult {
        repaired: true,
        rebuilt_records,
    })
}

fn canonicalize_project_storage(config: &MemfoldConfig) -> Result<()> {
    let repos_dir = config.root.join("memory").join("repos");
    if !repos_dir.exists() {
        return Ok(());
    }

    let mut repo_names = Vec::new();
    for entry in fs::read_dir(&repos_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            repo_names.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    repo_names.sort();

    for repo_name in repo_names {
        let source = repos_dir.join(&repo_name);
        if !source.exists() {
            continue;
        }

        let canonical_name = repo_name.to_ascii_lowercase();
        if canonical_name == repo_name {
            continue;
        }

        let destination = repos_dir.join(&canonical_name);
        merge_directory_contents(&source, &destination)?;
        if source.exists() {
            fs::remove_dir_all(&source)?;
        }

        let alias_qmd_dir = config
            .root
            .join("qmd")
            .join("collections")
            .join("repos")
            .join(&repo_name);
        if alias_qmd_dir.exists() {
            fs::remove_dir_all(alias_qmd_dir)?;
        }
    }

    Ok(())
}

fn discover_scopes(config: &MemfoldConfig) -> Result<Vec<ScopeRef>> {
    let mut scopes = vec![ScopeRef::new(ScopeType::User, "default")?];
    let repos_dir = config.root.join("memory").join("repos");
    if repos_dir.exists() {
        let mut entries = Vec::new();
        for entry in fs::read_dir(repos_dir)? {
            entries.push(entry?.file_name().to_string_lossy().to_string());
        }
        entries.sort();
        for project in entries {
            scopes.push(ScopeRef::new(ScopeType::Project, project)?);
        }
    }
    Ok(scopes)
}

fn canonicalize_project_scope_rows(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare(
        "SELECT scope_id FROM memory_items WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM session_log_entries WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM trace_archives WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM boot_entries WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM sessions WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM dream_jobs WHERE scope_type = 'project'
         UNION
         SELECT scope_id FROM tombstones WHERE scope_type = 'project'",
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let scope_ids = rows.collect::<rusqlite::Result<Vec<_>>>()?;

    for scope_id in scope_ids {
        let canonical = scope_id.to_ascii_lowercase();
        if canonical == scope_id {
            continue;
        }

        for table in [
            "memory_items",
            "session_log_entries",
            "trace_archives",
            "boot_entries",
            "sessions",
            "dream_jobs",
            "tombstones",
        ] {
            let sql = format!(
                "UPDATE {table} SET scope_id = ?1 WHERE scope_type = 'project' AND scope_id = ?2"
            );
            conn.execute(&sql, params![&canonical, &scope_id])?;
        }

        let old_ref = format!("project:{scope_id}");
        let new_ref = format!("project:{canonical}");
        conn.execute(
            "UPDATE mutations
             SET target_ref = REPLACE(target_ref, ?1, ?2)
             WHERE target_ref LIKE '%' || ?1 || '%'",
            params![old_ref, new_ref],
        )?;
    }

    Ok(())
}

fn clean_dirty_memory_content(config: &MemfoldConfig, scope: Option<&ScopeRef>) -> Result<()> {
    let project_scopes = match scope {
        Some(scope) if scope.scope_type == ScopeType::Project => vec![scope.clone()],
        Some(_) => Vec::new(),
        None => discover_project_scopes(config)?,
    };

    for scope in project_scopes {
        clean_stable_files(config, &scope)?;
        clean_session_logs(config, &scope)?;
        clean_history_files(config, &scope)?;
    }

    Ok(())
}

fn clean_stable_files(config: &MemfoldConfig, scope: &ScopeRef) -> Result<()> {
    let stable_dir = config.project_root(scope).join("stable");
    if !stable_dir.exists() {
        return Ok(());
    }

    let conn = open_connection(config)?;
    let mut files = Vec::new();
    for entry in fs::read_dir(&stable_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for path in files {
        let contents = fs::read_to_string(&path)?;
        let mut kept_blocks = Vec::new();
        let mut current_block = Vec::new();
        for line in contents.lines() {
            if line.starts_with("## item_key:") && !current_block.is_empty() {
                if !stable_block_tombstoned(&conn, scope, &current_block.join("\n"))? {
                    kept_blocks.push(current_block.join("\n"));
                }
                current_block.clear();
            }
            current_block.push(line.to_string());
        }
        if !current_block.is_empty()
            && !stable_block_tombstoned(&conn, scope, &current_block.join("\n"))?
        {
            kept_blocks.push(current_block.join("\n"));
        }

        if kept_blocks.is_empty() {
            fs::remove_file(&path)?;
            continue;
        }

        fs::write(&path, format!("{}\n", kept_blocks.join("\n\n")))?;
    }

    Ok(())
}

fn discover_project_scopes(config: &MemfoldConfig) -> Result<Vec<ScopeRef>> {
    let repos_dir = config.root.join("memory").join("repos");
    if !repos_dir.exists() {
        return Ok(Vec::new());
    }

    let mut scopes = Vec::new();
    let mut entries = Vec::new();
    for entry in fs::read_dir(repos_dir)? {
        let path = entry?.path();
        if path.is_dir() {
            entries.push(path);
        }
    }
    entries.sort();

    for entry in entries {
        let scope_id = entry.file_name().unwrap().to_string_lossy().to_string();
        scopes.push(ScopeRef::new(ScopeType::Project, scope_id)?);
    }

    Ok(scopes)
}

fn clear_scope_projections(conn: &Connection, scope: &ScopeRef) -> Result<()> {
    for table in ["memory_items", "session_log_entries", "trace_archives", "boot_entries"] {
        let sql = format!("DELETE FROM {table} WHERE scope_type = ?1 AND scope_id = ?2");
        conn.execute(&sql, params![scope.scope_type.as_str(), &scope.scope_id])?;
    }
    conn.execute(
        "DELETE FROM sessions WHERE scope_type = ?1 AND scope_id = ?2",
        params![scope.scope_type.as_str(), &scope.scope_id],
    )?;
    Ok(())
}

fn rebuild_memory_items(config: &MemfoldConfig, conn: &Connection, scope: &ScopeRef) -> Result<usize> {
    let stable_dir = config.project_root(scope).join("stable");
    if !stable_dir.exists() {
        return Ok(0);
    }

    let mut count = 0usize;
    let mut files = Vec::new();
    for entry in fs::read_dir(&stable_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for file in files {
        let relative_path = file
            .strip_prefix(&config.root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        for item in parse_stable_items(&file)? {
            conn.execute(
                "INSERT INTO memory_items (
                    id, scope_type, scope_id, item_key, file_path, title, status, autoload,
                    claim_fingerprint, content_hash, revision, supersedes_id, trust_score,
                    freshness_score, created_at, updated_at, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, 1.0, 1.0, ?12, ?12, NULL)",
                params![
                    format!("repair:{}:{}", scope.scope_key(), item.item_key),
                    scope.scope_type.as_str(),
                    &scope.scope_id,
                    item.item_key,
                    &relative_path,
                    item.title,
                    item.status,
                    item.autoload,
                    item.claim_fingerprint,
                    item.content_hash,
                    item.revision,
                    now_rfc3339(),
                ],
            )?;
            count += 1;
        }
    }

    Ok(count)
}

fn rebuild_evidence_items(config: &MemfoldConfig, conn: &Connection, scope: &ScopeRef) -> Result<usize> {
    let sessions_dir = config.project_root(scope).join("sessions");
    if !sessions_dir.exists() {
        return Ok(0);
    }

    let mut count = 0usize;
    let mut session_dirs = Vec::new();
    for entry in fs::read_dir(&sessions_dir)? {
        session_dirs.push(entry?.path());
    }
    session_dirs.sort();

    for session_dir in session_dirs {
        let session_id = session_dir.file_name().unwrap().to_string_lossy().to_string();
        let evidence_path = session_dir.join("session_log.jsonl");
        if !evidence_path.exists() {
            continue;
        }
        let relative_path = evidence_path
            .strip_prefix(&config.root)
            .unwrap_or(&evidence_path)
            .to_string_lossy()
            .replace('\\', "/");
        let contents = fs::read_to_string(&evidence_path)?;
        let mut evidence_count = 0i64;
        let mut started_at = None::<String>;
        let mut ended_at = None::<String>;
        for (idx, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            let created_at = value["created_at"].as_str().unwrap_or_default().to_string();
            if started_at.is_none() {
                started_at = Some(created_at.clone());
            }
            ended_at = Some(created_at.clone());
            conn.execute(
                "INSERT INTO session_log_entries (
                    id, session_id, scope_type, scope_id, source_kind, summary, raw_text, jsonl_path, line_no,
                    promotable, origin_mode, claim_fingerprint, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    value["evidence_id"].as_str().unwrap_or_default(),
                    &session_id,
                    scope.scope_type.as_str(),
                    &scope.scope_id,
                    value["source_kind"].as_str().unwrap_or_default(),
                    value["summary"].as_str().unwrap_or_default(),
                    value["raw_text"].as_str(),
                    &relative_path,
                    (idx + 1) as i64,
                    if value["promotable"].as_bool().unwrap_or(false) { 1 } else { 0 },
                    value["origin_mode"].as_str().unwrap_or("normal"),
                    value["claim_fingerprint"].as_str(),
                    &created_at,
                ],
            )?;
            evidence_count += 1;
            count += 1;
        }

        let started_at = started_at.unwrap_or_else(now_rfc3339);
        let ended_at = ended_at.unwrap_or_else(|| started_at.clone());

        conn.execute(
            "INSERT INTO sessions (id, scope_type, scope_id, mode, intent, host, started_at, ended_at, evidence_count)
             VALUES (?1, ?2, ?3, 'normal', 'continue', NULL, ?4, ?5, ?6)",
            params![
                &session_id,
                scope.scope_type.as_str(),
                &scope.scope_id,
                started_at,
                ended_at,
                evidence_count,
            ],
        )?;
    }

    Ok(count)
}

fn rebuild_trace_archives(config: &MemfoldConfig, conn: &Connection, scope: &ScopeRef) -> Result<usize> {
    let history_dir = config.project_root(scope).join("history").join("daily");
    if !history_dir.exists() {
        return Ok(0);
    }

    let mut count = 0usize;
    let mut files = Vec::new();
    for entry in fs::read_dir(&history_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for file in files {
        let relative_path = file
            .strip_prefix(&config.root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let archive_date = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("unknown")
            .to_string();

        for entry in parse_archive_entries(&file)? {
            conn.execute(
                "INSERT INTO trace_archives (
                    id, scope_type, scope_id, archive_date, archive_kind, file_path, line_no,
                    content_hash, created_at, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, 'daily_log', ?5, NULL, ?6, ?7, NULL)",
                params![
                    entry.entry_id,
                    scope.scope_type.as_str(),
                    &scope.scope_id,
                    &archive_date,
                    &relative_path,
                    content_hash(&entry.raw_block),
                    entry.created_at,
                ],
            )?;
            count += 1;
        }
    }

    Ok(count)
}

fn merge_directory_contents(source: &Path, destination: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    fs::create_dir_all(destination)?;

    let mut entries = Vec::new();
    for entry in fs::read_dir(source)? {
        entries.push(entry?.path());
    }
    entries.sort();

    for path in entries {
        let file_name = path.file_name().unwrap().to_os_string();
        let destination_path = destination.join(file_name);
        if path.is_dir() {
            merge_directory_contents(&path, &destination_path)?;
            if path.exists() {
                fs::remove_dir_all(&path)?;
            }
            continue;
        }

        if !destination_path.exists() {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&path, &destination_path)?;
            continue;
        }

        merge_file_contents(&path, &destination_path)?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
    }

    Ok(())
}

fn merge_file_contents(source: &Path, destination: &Path) -> Result<()> {
    let source_contents = fs::read_to_string(source)?;
    let destination_contents = fs::read_to_string(destination)?;
    if source_contents == destination_contents {
        return Ok(());
    }

    let merged = match source.extension().and_then(|ext| ext.to_str()) {
        Some("md") if destination.components().any(|component| component.as_os_str() == "stable") => {
            merge_stable_markdown_blocks(&destination_contents, &source_contents)
        }
        Some("jsonl") => merge_unique_lines(&destination_contents, &source_contents),
        Some("md") => merge_markdown_blocks(&destination_contents, &source_contents),
        _ => format!("{}\n{}", destination_contents.trim_end(), source_contents.trim_start()),
    };

    fs::write(destination, merged)?;
    Ok(())
}

fn merge_unique_lines(existing: &str, incoming: &str) -> String {
    let mut seen = HashSet::new();
    let mut lines = Vec::new();
    for line in existing.lines().chain(incoming.lines()) {
        let trimmed = line.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        lines.push(trimmed.to_string());
    }

    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn merge_markdown_blocks(existing: &str, incoming: &str) -> String {
    let mut seen = HashSet::new();
    let mut blocks = Vec::new();
    for block in existing
        .split("\n\n")
        .chain(incoming.split("\n\n"))
        .map(str::trim)
        .filter(|block| !block.is_empty())
    {
        if seen.insert(block.to_string()) {
            blocks.push(block.to_string());
        }
    }

    if blocks.is_empty() {
        String::new()
    } else {
        format!("{}\n", blocks.join("\n\n"))
    }
}

fn merge_stable_markdown_blocks(existing: &str, incoming: &str) -> String {
    let mut seen_item_keys = HashSet::new();
    let mut blocks = Vec::new();
    for block in existing
        .split("\n\n")
        .chain(incoming.split("\n\n"))
        .map(str::trim)
        .filter(|block| !block.is_empty())
    {
        let Some(item_key) = stable_item_key(block) else {
            if !blocks.iter().any(|existing_block| existing_block == block) {
                blocks.push(block.to_string());
            }
            continue;
        };
        if seen_item_keys.insert(item_key) {
            blocks.push(block.to_string());
        }
    }

    if blocks.is_empty() {
        String::new()
    } else {
        format!("{}\n", blocks.join("\n\n"))
    }
}

fn stable_item_key(block: &str) -> Option<String> {
    block
        .lines()
        .find_map(|line| line.strip_prefix("## item_key:").map(|value| value.trim().to_string()))
}

fn stable_block_tombstoned(conn: &Connection, scope: &ScopeRef, block: &str) -> Result<bool> {
    let claim_fingerprint = block.lines().find_map(|line| {
        line.strip_prefix("claim_fingerprint:")
            .map(|value| value.trim().to_string())
    });
    let Some(claim_fingerprint) = claim_fingerprint else {
        return Ok(false);
    };

    let exists = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM tombstones
                WHERE scope_type = ?1 AND scope_id = ?2 AND claim_fingerprint = ?3
            )",
            params![scope.scope_type.as_str(), &scope.scope_id, claim_fingerprint],
            |row| row.get::<_, i64>(0),
        )?
        != 0;
    Ok(exists)
}

fn clean_session_logs(config: &MemfoldConfig, scope: &ScopeRef) -> Result<()> {
    let sessions_dir = config.project_root(scope).join("sessions");
    if !sessions_dir.exists() {
        return Ok(());
    }

    let mut session_dirs = Vec::new();
    for entry in fs::read_dir(&sessions_dir)? {
        session_dirs.push(entry?.path());
    }
    session_dirs.sort();

    for session_dir in session_dirs {
        let session_log_path = session_dir.join("session_log.jsonl");
        if !session_log_path.exists() {
            continue;
        }

        let contents = fs::read_to_string(&session_log_path)?;
        let mut kept = Vec::new();
        let mut seen_evidence_ids = HashSet::new();
        for line in contents.lines().filter(|line| !line.trim().is_empty()) {
            let mut value: Value = serde_json::from_str(line)?;
            let summary = value["summary"].as_str().unwrap_or_default().to_string();
            if is_memory_noise(&summary) {
                continue;
            }

            let evidence_id = value["evidence_id"].as_str().unwrap_or_default().to_string();
            if !evidence_id.is_empty() && !seen_evidence_ids.insert(evidence_id) {
                continue;
            }

            if let Some(scope_obj) = value.get_mut("scope").and_then(Value::as_object_mut) {
                scope_obj.insert("id".to_string(), Value::String(scope.scope_id.clone()));
            }
            kept.push(serde_json::to_string(&value)?);
        }

        if kept.is_empty() {
            fs::remove_file(&session_log_path)?;
            if session_dir.read_dir()?.next().is_none() {
                fs::remove_dir(session_dir)?;
            }
            continue;
        }

        fs::write(&session_log_path, format!("{}\n", kept.join("\n")))?;
    }

    Ok(())
}

fn clean_history_files(config: &MemfoldConfig, scope: &ScopeRef) -> Result<()> {
    let history_dir = config.project_root(scope).join("history").join("daily");
    if !history_dir.exists() {
        return Ok(());
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(&history_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();

    for path in files {
        let contents = fs::read_to_string(&path)?;
        let mut cleaned_blocks = Vec::new();
        for block in contents.split("\n\n").map(str::trim).filter(|block| !block.is_empty()) {
            if let Some(cleaned) = clean_history_block(block) {
                cleaned_blocks.push(cleaned);
            }
        }

        if cleaned_blocks.is_empty() {
            fs::remove_file(&path)?;
            continue;
        }

        fs::write(&path, format!("{}\n", cleaned_blocks.join("\n\n")))?;
    }

    Ok(())
}

fn clean_history_block(block: &str) -> Option<String> {
    let lines = block.lines().collect::<Vec<_>>();
    let headline = lines
        .iter()
        .find(|line| line.starts_with("## "))
        .copied()?;
    let summary = headline
        .split("] ")
        .nth(1)
        .unwrap_or_default()
        .trim()
        .to_string();
    if is_memory_noise(&summary) {
        return None;
    }

    let mut cleaned = Vec::new();
    for line in lines {
        if line.starts_with("- ") {
            let bullet_summary = line
                .split("] ")
                .nth(1)
                .unwrap_or_else(|| line.trim_start_matches("- ").trim());
            if is_memory_noise(bullet_summary) {
                continue;
            }
        }
        cleaned.push(line.to_string());
    }

    Some(cleaned.join("\n"))
}

#[derive(Debug)]
struct StableItem {
    item_key: String,
    title: String,
    status: String,
    autoload: String,
    claim_fingerprint: String,
    content_hash: String,
    revision: i64,
}

fn parse_stable_items(path: &std::path::Path) -> Result<Vec<StableItem>> {
    let contents = fs::read_to_string(path)?;
    let mut items = Vec::new();
    let mut current_key = None::<String>;
    let mut metadata = std::collections::BTreeMap::new();

    for line in contents.lines().chain(std::iter::once("## item_key: __END__")) {
        if let Some(rest) = line.strip_prefix("## item_key:") {
            if let Some(item_key) = current_key.take() {
                if item_key != "__END__" {
                    items.push(StableItem {
                        item_key,
                        title: metadata.remove("title").unwrap_or_default(),
                        status: metadata.remove("status").unwrap_or_else(|| "stable".to_string()),
                        autoload: metadata
                            .remove("autoload")
                            .unwrap_or_else(|| "boot_project".to_string()),
                        claim_fingerprint: metadata.remove("claim_fingerprint").unwrap_or_default(),
                        content_hash: metadata.remove("content_hash").unwrap_or_else(|| {
                            let digest = Sha256::digest(b"");
                            format!("sha256:{digest:x}")
                        }),
                        revision: metadata
                            .remove("revision")
                            .and_then(|value| value.parse::<i64>().ok())
                            .unwrap_or(1),
                    });
                }
                metadata.clear();
            }
            current_key = Some(rest.trim().to_string());
            continue;
        }

        if current_key.is_some() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                if matches!(
                    key,
                    "title" | "status" | "autoload" | "claim_fingerprint" | "content_hash" | "revision"
                ) {
                    metadata.insert(key.to_string(), value.trim().to_string());
                }
            }
        }
    }

    Ok(items)
}

#[derive(Debug)]
struct ArchiveEntry {
    entry_id: String,
    created_at: String,
    raw_block: String,
}

fn parse_archive_entries(path: &std::path::Path) -> Result<Vec<ArchiveEntry>> {
    let contents = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    let mut current_prelude = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in contents.lines() {
        if line.starts_with("<!-- ") && current_header.is_none() {
            current_prelude.push(line.to_string());
        } else if line.starts_with("## ") {
            if let Some(header) = current_header.take() {
                if let Some(entry) = finalize_archive_entry(&current_prelude, &header, &current_lines) {
                    entries.push(entry);
                }
                current_prelude.clear();
                current_lines.clear();
            }
            current_header = Some(line.to_string());
        } else if current_header.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(header) = current_header.take() {
        if let Some(entry) = finalize_archive_entry(&current_prelude, &header, &current_lines) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn finalize_archive_entry(prelude: &[String], header: &str, lines: &[String]) -> Option<ArchiveEntry> {
    let created_at = header
        .trim_start_matches("## ")
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_string();
    let entry_id = prelude
        .iter()
        .find_map(|line| {
            line.strip_prefix("<!-- ")
                .and_then(|value| value.strip_suffix(" -->"))
                .and_then(|value| {
                    value.split('|').find_map(|part| {
                        part.trim()
                            .strip_prefix("summary_id: ")
                            .map(|value| value.trim().to_string())
                    })
                })
        })
        .or_else(|| {
            lines
                .iter()
                .find_map(|line| line.strip_prefix("evidence_id:").map(|value| value.trim().to_string()))
        })?;

    let mut raw_block = String::new();
    for line in prelude {
        raw_block.push_str(line);
        raw_block.push('\n');
    }
    raw_block.push_str(header);
    raw_block.push('\n');
    for line in lines {
        raw_block.push_str(line);
        raw_block.push('\n');
    }

    Some(ArchiveEntry {
        entry_id,
        created_at,
        raw_block,
    })
}

fn content_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}
