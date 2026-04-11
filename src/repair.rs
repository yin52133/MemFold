use std::fs;

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::boot::compile_scope_bundle;
use crate::config::MemfoldConfig;
use crate::domain::{ScopeRef, ScopeType};
use crate::error::Result;
use crate::qmd_adapter::sync_scope;
use crate::state::schema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RepairResult {
    pub repaired: bool,
    pub rebuilt_records: usize,
}

pub fn run_repair(config: &MemfoldConfig, scope: Option<&ScopeRef>) -> Result<RepairResult> {
    let conn = open_connection(config)?;
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

fn discover_scopes(config: &MemfoldConfig) -> Result<Vec<ScopeRef>> {
    let mut scopes = vec![ScopeRef::new(ScopeType::User, "default")?];
    let projects_dir = config.root.join("memory").join("projects");
    if projects_dir.exists() {
        let mut entries = Vec::new();
        for entry in fs::read_dir(projects_dir)? {
            entries.push(entry?.file_name().to_string_lossy().to_string());
        }
        entries.sort();
        for project in entries {
            scopes.push(ScopeRef::new(ScopeType::Project, project)?);
        }
    }
    Ok(scopes)
}

fn clear_scope_projections(conn: &Connection, scope: &ScopeRef) -> Result<()> {
    for table in ["memory_items", "evidence_items", "trace_archives", "boot_entries"] {
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
                    OffsetDateTime::now_utc().to_string(),
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
        let evidence_path = session_dir.join("evidence.jsonl");
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
        for (idx, line) in contents.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = serde_json::from_str(line)?;
            conn.execute(
                "INSERT INTO evidence_items (
                    id, session_id, scope_type, scope_id, source_kind, summary, jsonl_path, line_no,
                    promotable, origin_mode, claim_fingerprint, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    value["evidence_id"].as_str().unwrap_or_default(),
                    &session_id,
                    scope.scope_type.as_str(),
                    &scope.scope_id,
                    value["source_kind"].as_str().unwrap_or_default(),
                    value["summary"].as_str().unwrap_or_default(),
                    &relative_path,
                    (idx + 1) as i64,
                    if value["promotable"].as_bool().unwrap_or(false) { 1 } else { 0 },
                    value["origin_mode"].as_str().unwrap_or("normal"),
                    value["claim_fingerprint"].as_str(),
                    value["created_at"].as_str().unwrap_or_default(),
                ],
            )?;
            evidence_count += 1;
            count += 1;
        }

        conn.execute(
            "INSERT INTO sessions (id, scope_type, scope_id, mode, intent, host, started_at, ended_at, evidence_count)
             VALUES (?1, ?2, ?3, 'normal', 'continue', NULL, ?4, NULL, ?5)",
            params![
                &session_id,
                scope.scope_type.as_str(),
                &scope.scope_id,
                OffsetDateTime::now_utc().to_string(),
                evidence_count,
            ],
        )?;
    }

    Ok(count)
}

fn rebuild_trace_archives(config: &MemfoldConfig, conn: &Connection, scope: &ScopeRef) -> Result<usize> {
    let archive_dir = config.project_root(scope).join("archive");
    if !archive_dir.exists() {
        return Ok(0);
    }

    let mut count = 0usize;
    let mut files = Vec::new();
    for entry in fs::read_dir(&archive_dir)? {
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
            .unwrap_or("memory-unknown")
            .trim_start_matches("memory-")
            .to_string();

        for entry in parse_archive_entries(&file)? {
            conn.execute(
                "INSERT INTO trace_archives (
                    id, scope_type, scope_id, archive_date, archive_kind, file_path, line_no,
                    content_hash, created_at, deleted_at
                ) VALUES (?1, ?2, ?3, ?4, 'daily_log', ?5, NULL, ?6, ?7, NULL)",
                params![
                    entry.evidence_id,
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
    evidence_id: String,
    created_at: String,
    raw_block: String,
}

fn parse_archive_entries(path: &std::path::Path) -> Result<Vec<ArchiveEntry>> {
    let contents = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    let mut current_header: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in contents.lines() {
        if line.starts_with("## ") {
            if let Some(header) = current_header.take() {
                if let Some(entry) = finalize_archive_entry(&header, &current_lines) {
                    entries.push(entry);
                }
                current_lines.clear();
            }
            current_header = Some(line.to_string());
        } else if current_header.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(header) = current_header.take() {
        if let Some(entry) = finalize_archive_entry(&header, &current_lines) {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn finalize_archive_entry(header: &str, lines: &[String]) -> Option<ArchiveEntry> {
    let created_at = header
        .trim_start_matches("## ")
        .split(' ')
        .next()
        .unwrap_or_default()
        .to_string();
    let evidence_id = lines
        .iter()
        .find_map(|line| line.strip_prefix("evidence_id:").map(|value| value.trim().to_string()))?;

    let mut raw_block = String::new();
    raw_block.push_str(header);
    raw_block.push('\n');
    for line in lines {
        raw_block.push_str(line);
        raw_block.push('\n');
    }

    Some(ArchiveEntry {
        evidence_id,
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
