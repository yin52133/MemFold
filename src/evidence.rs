use rusqlite::{params, Connection, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::config::MemfoldConfig;
use crate::domain::{Mode, ScopeRef, SourceKind};
use crate::error::{Error, Result};
use crate::memory_fs::jsonl::append_jsonl_line;
use crate::memory_fs::paths::session_evidence_path;
use crate::mutations::MutationStore;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone)]
pub struct WriteEvidenceInput {
    pub scope: ScopeRef,
    pub session_id: String,
    pub source_kind: SourceKind,
    pub summary: String,
    pub raw_text: Option<String>,
    pub promotable: bool,
    pub origin_mode: Mode,
    pub claim_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WriteEvidenceResult {
    pub evidence_id: String,
    pub stored: bool,
}

#[derive(Debug, Serialize)]
struct EvidenceJsonlRecord<'a> {
    evidence_id: &'a str,
    scope: EvidenceScope<'a>,
    session_id: &'a str,
    source_kind: &'a str,
    summary: &'a str,
    raw_text: Option<&'a str>,
    promotable: bool,
    origin_mode: &'a str,
    claim_fingerprint: Option<&'a str>,
    created_at: &'a str,
}

#[derive(Debug, Serialize)]
struct EvidenceScope<'a> {
    #[serde(rename = "type")]
    scope_type: &'a str,
    id: &'a str,
}

pub fn write_evidence(config: &MemfoldConfig, input: &WriteEvidenceInput) -> Result<WriteEvidenceResult> {
    validate_summary(&input.summary)?;

    let mutation_store = MutationStore::new(config.clone());
    let idempotency_key = evidence_idempotency_key(input);
    let mutation_id = mutation_store.begin(
        &input.scope,
        "write_evidence",
        &input.session_id,
        &idempotency_key,
    )?;
    let evidence_id = format!("ev_{mutation_id}");

    let mut conn = open_connection(config)?;

    if evidence_exists(&conn, &evidence_id)? {
        mutation_store.mark_applied_to_content(&mutation_id)?;
        mutation_store.mark_fully_applied(&mutation_id)?;
        return Ok(WriteEvidenceResult {
            evidence_id,
            stored: true,
        });
    }

    let created_at = current_timestamp();
    let jsonl_path = session_evidence_path(config, &input.scope, &input.session_id);
    let jsonl_rel_path = relative_session_evidence_path(&input.scope, &input.session_id);
    let raw_text = normalized_raw_text(input);

    let jsonl_record = EvidenceJsonlRecord {
        evidence_id: &evidence_id,
        scope: EvidenceScope {
            scope_type: input.scope.scope_type.as_str(),
            id: &input.scope.scope_id,
        },
        session_id: &input.session_id,
        source_kind: input.source_kind.as_str(),
        summary: &input.summary,
        raw_text: raw_text.as_deref(),
        promotable: input.promotable,
        origin_mode: input.origin_mode.as_str(),
        claim_fingerprint: input.claim_fingerprint.as_deref(),
        created_at: &created_at,
    };
    let jsonl_line = serde_json::to_string(&jsonl_record)?;
    let line_no = append_jsonl_line(&jsonl_path, &jsonl_line)?;

    let tx = conn.transaction()?;
    upsert_session(&tx, input, &created_at)?;
    insert_evidence_item(
        &tx,
        &evidence_id,
        input,
        &jsonl_rel_path,
        line_no as i64,
        raw_text.as_deref(),
        &created_at,
    )?;
    tx.commit()?;

    mutation_store.mark_applied_to_content(&mutation_id)?;
    mutation_store.mark_fully_applied(&mutation_id)?;

    Ok(WriteEvidenceResult {
        evidence_id,
        stored: true,
    })
}

fn validate_summary(summary: &str) -> Result<()> {
    if summary.trim().is_empty()
        || summary.len() > 4 * 1024
        || summary
            .chars()
            .any(|ch| ch == '\0' || (ch.is_control() && !matches!(ch, '\n' | '\r' | '\t')))
    {
        return Err(Error::UnsafeSummary);
    }

    Ok(())
}

fn evidence_exists(conn: &Connection, evidence_id: &str) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM session_log_entries WHERE id = ?1)",
            params![evidence_id],
            |row| row.get::<_, i64>(0),
        )?
        != 0;

    Ok(exists)
}

fn insert_evidence_item(
    tx: &Transaction<'_>,
    evidence_id: &str,
    input: &WriteEvidenceInput,
    jsonl_path: &str,
    line_no: i64,
    raw_text: Option<&str>,
    created_at: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO session_log_entries (
            id,
            session_id,
            scope_type,
            scope_id,
            source_kind,
            summary,
            raw_text,
            jsonl_path,
            line_no,
            promotable,
            origin_mode,
            claim_fingerprint,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            evidence_id,
            input.session_id,
            input.scope.scope_type.as_str(),
            input.scope.scope_id,
            input.source_kind.as_str(),
            input.summary,
            raw_text,
            jsonl_path,
            line_no,
            if input.promotable { 1 } else { 0 },
            input.origin_mode.as_str(),
            input.claim_fingerprint,
            created_at,
        ],
    )?;

    Ok(())
}

fn upsert_session(
    tx: &Transaction<'_>,
    input: &WriteEvidenceInput,
    created_at: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO sessions (
            id,
            scope_type,
            scope_id,
            mode,
            intent,
            host,
            started_at,
            ended_at,
            evidence_count
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, NULL, 1)
        ON CONFLICT(id) DO UPDATE SET
            evidence_count = evidence_count + 1",
        params![
            input.session_id,
            input.scope.scope_type.as_str(),
            input.scope.scope_id,
            input.origin_mode.as_str(),
            "continue",
            created_at,
        ],
    )?;

    Ok(())
}


fn current_timestamp() -> String {
    now_rfc3339()
}

fn evidence_idempotency_key(input: &WriteEvidenceInput) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.scope.scope_key().as_bytes());
    hasher.update(b"\0");
    hasher.update(input.session_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(input.source_kind.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(input.summary.as_bytes());
    hasher.update(b"\0");
    hasher.update(if input.promotable { b"1" } else { b"0" });
    hasher.update(b"\0");
    hasher.update(input.origin_mode.as_str().as_bytes());
    hasher.update(b"\0");
    hasher.update(input.claim_fingerprint.as_deref().unwrap_or("").as_bytes());
    let digest = hasher.finalize();

    format!("evidence:{digest:x}")
}

fn open_connection(config: &MemfoldConfig) -> Result<Connection> {
    let db_path = config.state_db_path();
    MemfoldConfig::ensure_parent_dir(&db_path)?;

    let conn = Connection::open(db_path)?;
    schema::apply_schema(&conn)?;
    Ok(conn)
}

fn relative_session_evidence_path(scope: &ScopeRef, session_id: &str) -> String {
    format!(
        "memory/{}/sessions/{session_id}/session_log.jsonl",
        scope.scope_dir_fragment()
    )
}

fn normalized_raw_text(input: &WriteEvidenceInput) -> Option<String> {
    input.raw_text.clone().or_else(|| {
        if input.source_kind == SourceKind::User {
            Some(input.summary.clone())
        } else {
            None
        }
    })
}
