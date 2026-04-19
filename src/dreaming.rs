use std::fs;

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::boot::compile_scope_bundle;
use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::{Error, Result};
use crate::qmd_adapter::sync_scope;
use crate::state::schema;
use crate::timestamps::{now_rfc3339, parse_timestamp};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DreamRunResult {
    pub job_id: String,
    pub promoted: u64,
    pub held: u64,
    pub quarantined: u64,
    pub discarded: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaybeDreamResult {
    pub ran: bool,
    pub reason: String,
    pub result: Option<DreamRunResult>,
}

pub fn run_dream(config: &MemfoldConfig, scope: &ScopeRef, trigger: &str) -> Result<DreamRunResult> {
    if trigger == "scheduled" {
        let gate = scheduled_gate(config, scope)?;
        if !gate.eligible {
            return Err(Error::DreamingNotEligible(gate.reason));
        }
    }

    let conn = open_connection(config)?;
    let now = now_rfc3339();
    let job_id = format!("dj_{}", Uuid::new_v4().simple());

    conn.execute(
        "INSERT INTO dream_jobs (id, scope_type, scope_id, trigger, status, promoted, held, quarantined, discarded, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, 'running', 0, 0, 0, 0, ?5, ?5)",
        params![&job_id, scope.scope_type.as_str(), &scope.scope_id, trigger, &now],
    )?;

    let candidates = {
        let mut stmt = conn.prepare(
            "SELECT id, summary, raw_text, claim_fingerprint, source_kind, origin_mode
             FROM session_log_entries
             WHERE scope_type = ?1 AND scope_id = ?2 AND promotable = 1
             ORDER BY created_at",
        )?;
        let rows = stmt.query_map(params![scope.scope_type.as_str(), &scope.scope_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };

    let mut promoted = 0u64;
    let held = 0u64;
    let quarantined = 0u64;
    let mut discarded = 0u64;

    for (_evidence_id, summary, raw_text, claim_fingerprint, source_kind, origin_mode) in candidates {
        let claim_fingerprint = claim_fingerprint.unwrap_or_else(|| fingerprint_for(&summary));

        if origin_mode == "sterile"
            || (source_kind == "user" && raw_text.as_deref().unwrap_or("").trim().is_empty())
            || tombstone_exists(&conn, scope, &claim_fingerprint)?
        {
            discarded += 1;
            continue;
        }

        if memory_exists(&conn, scope, &claim_fingerprint)? {
            discarded += 1;
            continue;
        }

        let item_key = item_key_for(scope, &claim_fingerprint);
        let file_path = stable_file_path(config, scope);
        append_stable_block(&file_path, &item_key, &summary, &claim_fingerprint)?;

        let relative_file_path = file_path
            .strip_prefix(&config.root)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .replace('\\', "/");
        let content_hash = content_hash(&summary);
        let item_id = format!("mem_{}", Uuid::new_v4().simple());

        conn.execute(
            "INSERT INTO memory_items (
                id, scope_type, scope_id, item_key, file_path, title, status, autoload,
                claim_fingerprint, content_hash, revision, supersedes_id, trust_score,
                freshness_score, created_at, updated_at, deleted_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'stable', ?7, ?8, ?9, 1, NULL, 1.0, 1.0, ?10, ?10, NULL)",
            params![
                item_id,
                scope.scope_type.as_str(),
                &scope.scope_id,
                item_key,
                relative_file_path,
                truncate_title(&summary),
                autoload_for(scope),
                claim_fingerprint,
                content_hash,
                &now,
            ],
        )?;
        promoted += 1;
    }

    compile_scope_bundle(config, scope, 900)?;
    sync_scope(config, scope)?;

    conn.execute(
        "UPDATE dream_jobs
         SET status = 'applied', promoted = ?1, held = ?2, quarantined = ?3, discarded = ?4, updated_at = ?5
         WHERE id = ?6",
        params![promoted as i64, held as i64, quarantined as i64, discarded as i64, &now, &job_id],
    )?;

    Ok(DreamRunResult {
        job_id,
        promoted,
        held,
        quarantined,
        discarded,
    })
}

pub fn maybe_run_scheduled_dream(config: &MemfoldConfig, scope: &ScopeRef) -> Result<MaybeDreamResult> {
    let gate = scheduled_gate(config, scope)?;
    if !gate.eligible {
        return Ok(MaybeDreamResult {
            ran: false,
            reason: gate.reason,
            result: None,
        });
    }

    let result = run_dream(config, scope, "scheduled")?;
    Ok(MaybeDreamResult {
        ran: true,
        reason: "eligible".to_string(),
        result: Some(result),
    })
}

fn append_stable_block(
    path: &std::path::Path,
    item_key: &str,
    summary: &str,
    claim_fingerprint: &str,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut existing = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    if !existing.is_empty() {
        existing.push('\n');
    }
    existing.push_str(&format!(
        "## item_key: {item_key}\n\
title: {}\n\
status: stable\n\
autoload: {}\n\
claim_fingerprint: {claim_fingerprint}\n\
content_hash: {}\n\
revision: 1\n\n\
{}\n",
        truncate_title(summary),
        if item_key.starts_with("user.") { "boot_user" } else { "boot_project" },
        content_hash(summary),
        summary
    ));
    fs::write(path, existing)?;
    Ok(())
}

fn item_key_for(scope: &ScopeRef, claim_fingerprint: &str) -> String {
    let suffix = claim_fingerprint
        .trim_start_matches("cfp_")
        .chars()
        .take(12)
        .collect::<String>();
    match scope.scope_type.as_str() {
        "user" => format!("user.memory.{suffix}"),
        _ => format!("project.memory.{suffix}"),
    }
}

fn autoload_for(scope: &ScopeRef) -> &'static str {
    if scope.scope_type.as_str() == "user" {
        "boot_user"
    } else {
        "boot_project"
    }
}

fn stable_file_path(config: &MemfoldConfig, scope: &ScopeRef) -> std::path::PathBuf {
    let name = if scope.scope_type.as_str() == "user" {
        "preferences.md"
    } else {
        "rules.md"
    };
    config.project_root(scope).join("stable").join(name)
}

fn truncate_title(summary: &str) -> String {
    summary.chars().take(48).collect()
}

fn tombstone_exists(conn: &Connection, scope: &ScopeRef, claim_fingerprint: &str) -> Result<bool> {
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

fn memory_exists(conn: &Connection, scope: &ScopeRef, claim_fingerprint: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM memory_items
                WHERE scope_type = ?1 AND scope_id = ?2 AND claim_fingerprint = ?3 AND deleted_at IS NULL
            )",
            params![scope.scope_type.as_str(), &scope.scope_id, claim_fingerprint],
            |row| row.get::<_, i64>(0),
        )?
        != 0;
    Ok(exists)
}

fn fingerprint_for(summary: &str) -> String {
    let digest = Sha256::digest(summary.as_bytes());
    format!("cfp_{digest:x}")
}

fn content_hash(summary: &str) -> String {
    let digest = Sha256::digest(summary.as_bytes());
    format!("sha256:{digest:x}")
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

#[derive(Debug)]
struct ScheduledGate {
    eligible: bool,
    reason: String,
}

fn scheduled_gate(config: &MemfoldConfig, scope: &ScopeRef) -> Result<ScheduledGate> {
    let conn = open_connection(config)?;
    let last_applied = latest_applied_timestamp(&conn, scope)?;
    let session_count = ended_session_count_since(&conn, scope, last_applied)?;

    if session_count < 5 {
        return Ok(ScheduledGate {
            eligible: false,
            reason: format!("insufficient_sessions:{session_count}"),
        });
    }

    if let Some(last_applied) = last_applied {
        let hours_elapsed = ((OffsetDateTime::now_utc() - last_applied).whole_hours()) as i64;
        if hours_elapsed < 24 {
            return Ok(ScheduledGate {
                eligible: false,
                reason: format!("cooldown_hours:{hours_elapsed}"),
            });
        }
    }

    Ok(ScheduledGate {
        eligible: true,
        reason: "eligible".to_string(),
    })
}

fn latest_applied_timestamp(conn: &Connection, scope: &ScopeRef) -> Result<Option<OffsetDateTime>> {
    let mut stmt = conn.prepare(
        "SELECT updated_at
         FROM dream_jobs
         WHERE scope_type = ?1 AND scope_id = ?2 AND status = 'applied'",
    )?;
    let rows = stmt.query_map(params![scope.scope_type.as_str(), &scope.scope_id], |row| {
        row.get::<_, String>(0)
    })?;

    let mut latest = None;
    for row in rows {
        let value = row?;
        let parsed = parse_timestamp(&value)
            .ok_or_else(|| Error::DreamingNotEligible("invalid_last_applied_timestamp".to_string()))?;
        latest = Some(match latest {
            Some(current) if current >= parsed => current,
            _ => parsed,
        });
    }

    Ok(latest)
}

fn ended_session_count_since(
    conn: &Connection,
    scope: &ScopeRef,
    last_applied: Option<OffsetDateTime>,
) -> Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT ended_at
         FROM sessions
         WHERE scope_type = ?1 AND scope_id = ?2 AND ended_at IS NOT NULL",
    )?;
    let rows = stmt.query_map(params![scope.scope_type.as_str(), &scope.scope_id], |row| {
        row.get::<_, String>(0)
    })?;

    let mut count = 0i64;
    for row in rows {
        let value = row?;
        let ended_at = parse_timestamp(&value).ok_or_else(|| {
            Error::DreamingNotEligible("invalid_session_ended_timestamp".to_string())
        })?;
        if last_applied.map(|applied| ended_at > applied).unwrap_or(true) {
            count += 1;
        }
    }

    Ok(count)
}
