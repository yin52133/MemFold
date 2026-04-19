use std::fs;
use std::path::PathBuf;

use rusqlite::{params, Connection};
use serde::Serialize;
use uuid::Uuid;

use crate::boot::compile_scope_bundle;
use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::Result;
use crate::qmd_adapter::sync_scope;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeedbackResult {
    pub updated: bool,
    pub tombstone_written: bool,
}

pub fn apply_feedback(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    claim_fingerprint: &str,
    verdict: &str,
    reason: &str,
    session_id: Option<&str>,
) -> Result<FeedbackResult> {
    let mut conn = open_connection(config)?;
    let now = now_rfc3339();

    let targets = {
        let mut stmt = conn.prepare(
            "SELECT id, item_key, file_path
             FROM memory_items
             WHERE scope_type = ?1 AND scope_id = ?2 AND claim_fingerprint = ?3 AND deleted_at IS NULL",
        )?;
        let rows = stmt.query_map(
            params![scope.scope_type.as_str(), &scope.scope_id, claim_fingerprint],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let evidence_targets = {
        let mut stmt = conn.prepare(
            "SELECT id
             FROM session_log_entries
             WHERE scope_type = ?1 AND scope_id = ?2 AND claim_fingerprint = ?3",
        )?;
        let rows = stmt.query_map(
            params![scope.scope_type.as_str(), &scope.scope_id, claim_fingerprint],
            |row| row.get::<_, String>(0),
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let new_status = match verdict {
        "rejected" => "rejected",
        "disputed" => "disputed",
        "confirmed" => "stable",
        _ => "disputed",
    };

    let tx = conn.transaction()?;
    for (target_id, _, _) in &targets {
        tx.execute(
            "UPDATE memory_items SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_status, &now, target_id],
        )?;
        insert_feedback_event_if_missing(
            &tx,
            target_id,
            "memory_item",
            verdict,
            reason,
            session_id,
            &now,
        )?;
    }
    for target_id in &evidence_targets {
        insert_feedback_event_if_missing(
            &tx,
            target_id,
            "evidence_item",
            verdict,
            reason,
            session_id,
            &now,
        )?;
    }

    let mut tombstone_written = false;
    if verdict == "rejected"
        && (!targets.is_empty() || !evidence_targets.is_empty())
        && !tombstone_exists(&tx, scope, claim_fingerprint)?
    {
        tx.execute(
            "INSERT INTO tombstones (id, claim_fingerprint, scope_type, scope_id, reason, source_item_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6)",
            params![
                format!("ts_{}", Uuid::new_v4().simple()),
                claim_fingerprint,
                scope.scope_type.as_str(),
                &scope.scope_id,
                reason,
                &now,
            ],
        )?;
        tombstone_written = true;
    }
    tx.commit()?;

    for (_, item_key, file_path) in &targets {
        update_stable_file_status(
            &config.root.join(file_path),
            item_key,
            new_status,
        )?;
    }

    compile_scope_bundle(config, scope, 900)?;
    sync_scope(config, scope)?;

    Ok(FeedbackResult {
        updated: !targets.is_empty() || !evidence_targets.is_empty(),
        tombstone_written,
    })
}

fn tombstone_exists(
    tx: &rusqlite::Transaction<'_>,
    scope: &ScopeRef,
    claim_fingerprint: &str,
) -> Result<bool> {
    let exists = tx
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

fn insert_feedback_event_if_missing(
    tx: &rusqlite::Transaction<'_>,
    target_id: &str,
    target_type: &str,
    verdict: &str,
    reason: &str,
    session_id: Option<&str>,
    now: &str,
) -> Result<()> {
    let exists = tx
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM feedback_events
                WHERE target_id = ?1 AND target_type = ?2 AND verdict = ?3
            )",
            params![target_id, target_type, verdict],
            |row| row.get::<_, i64>(0),
        )?
        != 0;
    if exists {
        return Ok(());
    }

    tx.execute(
        "INSERT INTO feedback_events (id, target_id, target_type, verdict, reason, session_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            format!("fb_{}", Uuid::new_v4().simple()),
            target_id,
            target_type,
            verdict,
            reason,
            session_id,
            now,
        ],
    )?;
    Ok(())
}

fn update_stable_file_status(path: &PathBuf, item_key: &str, new_status: &str) -> Result<()> {
    let contents = fs::read_to_string(path)?;
    let mut updated = String::new();
    let mut current_item = None::<String>;

    for line in contents.lines() {
        if let Some(rest) = line.strip_prefix("## item_key:") {
            current_item = Some(rest.trim().to_string());
            updated.push_str(line);
            updated.push('\n');
            continue;
        }

        if current_item.as_deref() == Some(item_key) && line.starts_with("status:") {
            updated.push_str(&format!("status: {new_status}\n"));
            continue;
        }

        updated.push_str(line);
        updated.push('\n');
    }

    fs::write(path, updated)?;
    Ok(())
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}
