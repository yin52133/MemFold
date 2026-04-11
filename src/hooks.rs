use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use time::OffsetDateTime;

use crate::config::MemfoldConfig;
use crate::domain::{Mode, ScopeRef, SourceKind};
use crate::error::Result;
use crate::evidence::{write_evidence, WriteEvidenceInput};
use crate::state::schema;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEvent {
    TurnEnd,
    SessionEnd,
}

impl HookEvent {
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "turn_end" => Some(Self::TurnEnd),
            "session_end" => Some(Self::SessionEnd),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HookCaptureInput {
    pub event: HookEvent,
    pub scope: ScopeRef,
    pub session_id: String,
    pub source_kind: SourceKind,
    pub summary: String,
    pub origin_mode: Mode,
    pub state_changed: bool,
    pub promotable: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HookCaptureResult {
    pub recorded: bool,
    pub reason: String,
    pub evidence_id: Option<String>,
    pub normalized_summary: Option<String>,
}

pub fn capture_event(config: &MemfoldConfig, input: &HookCaptureInput) -> Result<HookCaptureResult> {
    let normalized_summary = normalize_summary(&input.summary);

    if !input.state_changed {
        return Ok(skip("no_state_change"));
    }
    if normalized_summary.is_empty() {
        return Ok(skip("empty_summary"));
    }
    if is_noise(&normalized_summary, &input.event, input.source_kind) {
        return Ok(skip("filtered_noise"));
    }
    if is_duplicate(config, input, &normalized_summary)? {
        return Ok(skip("duplicate_summary"));
    }

    let write_input = WriteEvidenceInput {
        scope: input.scope.clone(),
        session_id: input.session_id.clone(),
        source_kind: input.source_kind,
        summary: normalized_summary.clone(),
        promotable: input.promotable,
        origin_mode: input.origin_mode,
        claim_fingerprint: None,
    };
    let written = write_evidence(config, &write_input)?;
    if matches!(input.event, HookEvent::SessionEnd) {
        mark_session_ended(config, input)?;
    }

    Ok(HookCaptureResult {
        recorded: true,
        reason: "recorded".to_string(),
        evidence_id: Some(written.evidence_id),
        normalized_summary: Some(normalized_summary),
    })
}

fn normalize_summary(summary: &str) -> String {
    summary.split_whitespace().collect::<Vec<_>>().join(" ").trim().to_string()
}

fn is_noise(summary: &str, event: &HookEvent, source_kind: SourceKind) -> bool {
    let lowered = summary.to_lowercase();
    if lowered.len() < 8 {
        return true;
    }

    let exact_noise = [
        "no changes",
        "nothing changed",
        "continue working",
        "still working",
        "tool call completed",
        "done",
        "ok",
    ];
    if exact_noise.contains(&lowered.as_str()) {
        return true;
    }

    if matches!(event, HookEvent::TurnEnd)
        && source_kind != SourceKind::User
        && (lowered.starts_with("search ")
            || lowered.starts_with("opened ")
            || lowered.starts_with("listed ")
            || lowered.starts_with("thinking "))
    {
        return true;
    }

    false
}

fn is_duplicate(config: &MemfoldConfig, input: &HookCaptureInput, normalized_summary: &str) -> Result<bool> {
    let conn = open_connection(config)?;
    let existing = conn
        .query_row(
            "SELECT summary
             FROM evidence_items
             WHERE session_id = ?1 AND scope_type = ?2 AND scope_id = ?3 AND source_kind = ?4
             ORDER BY created_at DESC LIMIT 1",
            params![
                &input.session_id,
                input.scope.scope_type.as_str(),
                &input.scope.scope_id,
                input.source_kind.as_str()
            ],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(existing.as_deref() == Some(normalized_summary))
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;
    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

fn mark_session_ended(config: &MemfoldConfig, input: &HookCaptureInput) -> Result<()> {
    let conn = open_connection(config)?;
    conn.execute(
        "UPDATE sessions
         SET ended_at = ?1
         WHERE id = ?2 AND scope_type = ?3 AND scope_id = ?4",
        params![
            OffsetDateTime::now_utc().to_string(),
            &input.session_id,
            input.scope.scope_type.as_str(),
            &input.scope.scope_id,
        ],
    )?;
    Ok(())
}

fn skip(reason: &str) -> HookCaptureResult {
    HookCaptureResult {
        recorded: false,
        reason: reason.to_string(),
        evidence_id: None,
        normalized_summary: None,
    }
}
