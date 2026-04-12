use std::fs;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::{Error, Result};
use crate::state::schema;

#[derive(Debug, Clone)]
pub struct TraceQuery {
    pub scope: Option<ScopeRef>,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TraceResult {
    pub entry_id: String,
    pub session_id: String,
    pub line_no: i64,
    pub raw_text: Option<String>,
    pub summary: String,
}

pub fn trace_find(config: &MemfoldConfig, query: &TraceQuery) -> Result<TraceResult> {
    if query.query.trim().is_empty() {
        return Err(Error::EmptyQuery);
    }

    let conn = open_connection(config)?;
    let escaped = format!("%{}%", query.query.trim());
    let row = match &query.scope {
        Some(scope) => conn
            .query_row(
                "SELECT id, session_id, line_no, jsonl_path, raw_text, summary
                 FROM session_log_entries
                 WHERE scope_type = ?1
                   AND scope_id = ?2
                   AND (COALESCE(raw_text, summary) LIKE ?3 OR summary LIKE ?3)
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![scope.scope_type.as_str(), &scope.scope_id, escaped],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?,
        None => conn
            .query_row(
                "SELECT id, session_id, line_no, jsonl_path, raw_text, summary
                 FROM session_log_entries
                 WHERE COALESCE(raw_text, summary) LIKE ?1 OR summary LIKE ?1
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![escaped],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
            .optional()?,
    };

    let (entry_id, session_id, line_no, file_path, raw_text, summary) =
        row.ok_or_else(|| Error::TraceNotFound(query.query.clone()))?;

    let resolved_raw_text = if raw_text.is_some() {
        raw_text
    } else {
        raw_text_from_file(&config.root, &file_path, line_no)?
    };

    Ok(TraceResult {
        entry_id,
        session_id,
        line_no,
        raw_text: resolved_raw_text,
        summary,
    })
}

fn raw_text_from_file(root: &std::path::Path, file_path: &str, line_no: i64) -> Result<Option<String>> {
    let full_path = root.join(file_path);
    let contents = fs::read_to_string(&full_path)?;
    let line = contents
        .lines()
        .nth((line_no - 1) as usize)
        .ok_or_else(|| Error::SessionLogCorrupted(file_path.to_string()))?;
    let value: serde_json::Value = serde_json::from_str(line)?;
    Ok(value["raw_text"].as_str().map(|value| value.to_string()))
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}
