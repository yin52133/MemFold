use std::collections::BTreeSet;
use std::fs;

use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::{Error, Result};
use crate::memory_fs::paths::{archive_daily_path, session_evidence_path};
use crate::runtime_log::runtime_log_path;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone)]
pub struct SummarizeHistoryInput {
    pub scope: ScopeRef,
    pub session_id: String,
    pub trigger: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SummarizeHistoryResult {
    pub summary_id: String,
    pub updated: bool,
    pub daily_path: String,
}

pub fn summarize_history(
    config: &MemfoldConfig,
    input: &SummarizeHistoryInput,
) -> Result<SummarizeHistoryResult> {
    let entries = read_session_entries(config, input)?;
    if entries.is_empty() {
        return Err(Error::HistorySummaryFailed("session log is empty".to_string()));
    }

    let summary_time = entries
        .last()
        .map(|entry| entry.created_at.clone())
        .unwrap_or_else(now_rfc3339);
    let summary_date = summary_time.chars().take(10).collect::<String>();
    let daily_path = archive_daily_path(config, &input.scope, &summary_date);
    let summary_id = format!("hs_{}", Uuid::new_v4().simple());

    let block = render_history_block(config, input, &summary_id, &summary_time, &entries)?;
    upsert_history_block(&daily_path, &input.session_id, &block)?;

    let relative_path = daily_path
        .strip_prefix(&config.root)
        .unwrap_or(&daily_path)
        .to_string_lossy()
        .replace('\\', "/");

    let conn = open_connection(config)?;
    conn.execute(
        "INSERT INTO trace_archives (
            id, scope_type, scope_id, archive_date, archive_kind, file_path, line_no, content_hash, created_at, deleted_at
        ) VALUES (?1, ?2, ?3, ?4, 'daily_summary', ?5, NULL, ?6, ?7, NULL)
        ON CONFLICT(id) DO UPDATE SET
            file_path = excluded.file_path,
            content_hash = excluded.content_hash,
            created_at = excluded.created_at,
            deleted_at = NULL",
        params![
            summary_id,
            input.scope.scope_type.as_str(),
            &input.scope.scope_id,
            &summary_date,
            &relative_path,
            content_hash(&block),
            &summary_time,
        ],
    )?;

    Ok(SummarizeHistoryResult {
        summary_id,
        updated: true,
        daily_path: relative_path,
    })
}

#[derive(Debug)]
struct SessionLogEntry {
    source_kind: String,
    summary: String,
    created_at: String,
}

fn read_session_log(path: &std::path::Path) -> Result<Vec<SessionLogEntry>> {
    let contents = fs::read_to_string(path)?;
    let mut entries = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line)?;
        entries.push(SessionLogEntry {
            source_kind: value["source_kind"].as_str().unwrap_or("").to_string(),
            summary: value["summary"].as_str().unwrap_or("").to_string(),
            created_at: value["created_at"].as_str().unwrap_or_default().to_string(),
        });
    }
    Ok(entries)
}

fn read_session_entries(
    config: &MemfoldConfig,
    input: &SummarizeHistoryInput,
) -> Result<Vec<SessionLogEntry>> {
    let direct_path = session_evidence_path(config, &input.scope, &input.session_id);
    if direct_path.exists() {
        let entries = read_session_log(&direct_path)?;
        if !entries.is_empty() {
            return Ok(entries);
        }
    }

    let conn = open_connection(config)?;
    let mut stmt = conn.prepare(
        "SELECT source_kind, summary, created_at
         FROM session_log_entries
         WHERE session_id = ?1 AND scope_type = ?2 AND scope_id = ?3
         ORDER BY line_no",
    )?;
    let rows = stmt.query_map(
        params![
            &input.session_id,
            input.scope.scope_type.as_str(),
            &input.scope.scope_id
        ],
        |row| {
            Ok(SessionLogEntry {
                source_kind: row.get::<_, String>(0)?,
                summary: row.get::<_, String>(1)?,
                created_at: row.get::<_, String>(2)?,
            })
        },
    )?;
    let entries = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if entries.is_empty() {
        return Err(Error::SessionMissing(input.session_id.clone()));
    }
    Ok(entries)
}

fn render_history_block(
    config: &MemfoldConfig,
    input: &SummarizeHistoryInput,
    summary_id: &str,
    summary_time: &str,
    entries: &[SessionLogEntry],
) -> Result<String> {
    let mut key_points = BTreeSet::new();
    for entry in entries {
        let text = &entry.summary;
        let text = text.trim();
        if !text.is_empty() {
            key_points.insert(format!("[{}] {}", entry.source_kind, text));
        }
    }

    let runtime_failures = read_runtime_failures(config, &input.session_id)?;
    let headline = entries
        .iter()
        .find_map(|entry| {
            let text = entry.summary.trim();
            (!text.is_empty()).then(|| text.to_string())
        })
        .unwrap_or_else(|| "session summary".to_string());

    let mut block = String::new();
    block.push_str(&format!("<!-- session: {} | summary_id: {} -->\n", input.session_id, summary_id));
    block.push_str(&format!("## {} [{}] {}\n", summary_time, input.trigger, headline));
    for point in key_points.iter().take(5) {
        block.push_str(&format!("- {}\n", point));
    }
    for failure in runtime_failures {
        block.push_str(&format!("- [runtime] {}\n", failure));
    }
    Ok(block)
}

fn read_runtime_failures(config: &MemfoldConfig, session_id: &str) -> Result<Vec<String>> {
    let path = runtime_log_path(config, session_id);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    let mut failures = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line)?;
        if value["event"].as_str() == Some("failed") {
            let stage = value["stage"].as_str().unwrap_or("unknown");
            let message = value["message"].as_str().unwrap_or("unknown failure");
            failures.push(format!("{stage}: {message}"));
        }
    }
    Ok(failures)
}

fn upsert_history_block(path: &std::path::Path, session_id: &str, new_block: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    let marker = format!("<!-- session: {session_id} |");
    let mut kept_blocks = Vec::new();
    for block in existing.split("\n\n").map(str::trim).filter(|block| !block.is_empty()) {
        if !block.starts_with(&marker) {
            kept_blocks.push(block.to_string());
        }
    }
    kept_blocks.push(new_block.trim().to_string());
    let contents = format!("{}\n", kept_blocks.join("\n\n"));
    fs::write(path, contents)?;
    Ok(())
}

fn content_hash(value: &str) -> String {
    use sha2::{Digest, Sha256};
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
