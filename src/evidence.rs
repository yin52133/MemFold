use std::fs;
use std::io::Write;
use std::path::Path;

use rusqlite::{params, Connection, Transaction};
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

use crate::config::MemfoldConfig;
use crate::domain::{Mode, ScopeRef, SourceKind};
use crate::error::{Error, Result};
use crate::memory_fs::jsonl::append_jsonl_line;
use crate::memory_fs::paths::{archive_daily_path, session_evidence_path};
use crate::mutations::MutationStore;
use crate::state::schema;
use crate::timestamps::now_rfc3339;

#[derive(Debug, Clone)]
pub struct WriteEvidenceInput {
    pub scope: ScopeRef,
    pub session_id: String,
    pub source_kind: SourceKind,
    pub summary: String,
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
    let archive_date = current_archive_date();
    let jsonl_path = session_evidence_path(config, &input.scope, &input.session_id);
    let archive_path = archive_daily_path(config, &input.scope, &archive_date);
    let jsonl_rel_path = relative_session_evidence_path(&input.scope, &input.session_id);
    let archive_rel_path = relative_archive_path(&input.scope, &archive_date);

    let jsonl_record = EvidenceJsonlRecord {
        evidence_id: &evidence_id,
        scope: EvidenceScope {
            scope_type: input.scope.scope_type.as_str(),
            id: &input.scope.scope_id,
        },
        session_id: &input.session_id,
        source_kind: input.source_kind.as_str(),
        summary: &input.summary,
        promotable: input.promotable,
        origin_mode: input.origin_mode.as_str(),
        claim_fingerprint: input.claim_fingerprint.as_deref(),
        created_at: &created_at,
    };
    let jsonl_line = serde_json::to_string(&jsonl_record)?;
    let line_no = append_jsonl_line(&jsonl_path, &jsonl_line)?;

    let archive_entry = render_archive_entry(input, &evidence_id, &jsonl_rel_path, &created_at);
    append_markdown_entry(&archive_path, &archive_entry)?;
    let content_hash = content_hash(&archive_entry);

    let tx = conn.transaction()?;
    upsert_session(&tx, input, &created_at)?;
    insert_evidence_item(
        &tx,
        &evidence_id,
        input,
        &jsonl_rel_path,
        line_no as i64,
        &created_at,
    )?;
    insert_trace_archive(
        &tx,
        &evidence_id,
        input,
        &archive_rel_path,
        &archive_date,
        &content_hash,
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
            jsonl_path,
            line_no,
            promotable,
            origin_mode,
            claim_fingerprint,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            evidence_id,
            input.session_id,
            input.scope.scope_type.as_str(),
            input.scope.scope_id,
            input.source_kind.as_str(),
            input.summary,
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

fn insert_trace_archive(
    tx: &Transaction<'_>,
    evidence_id: &str,
    input: &WriteEvidenceInput,
    file_path: &str,
    archive_date: &str,
    content_hash: &str,
    created_at: &str,
) -> Result<()> {
    tx.execute(
        "INSERT INTO trace_archives (
            id,
            scope_type,
            scope_id,
            archive_date,
            archive_kind,
            file_path,
            line_no,
            content_hash,
            created_at,
            deleted_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, NULL)",
        params![
            evidence_id,
            input.scope.scope_type.as_str(),
            input.scope.scope_id,
            archive_date,
            "daily_log",
            file_path,
            Option::<i64>::None,
            content_hash,
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

fn append_markdown_entry(path: &Path, entry: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;

    if file.metadata()?.len() > 0 && !ends_with_newline(path)? {
        file.write_all(b"\n")?;
    }

    file.write_all(entry.as_bytes())?;
    if !entry.ends_with('\n') {
        file.write_all(b"\n")?;
    }
    file.write_all(b"\n")?;
    file.sync_all()?;

    Ok(())
}

fn ends_with_newline(path: &Path) -> Result<bool> {
    let metadata = fs::metadata(path)?;
    if metadata.len() == 0 {
        return Ok(true);
    }

    use std::io::{Read, Seek, SeekFrom};

    let mut file = fs::File::open(path)?;
    file.seek(SeekFrom::End(-1))?;
    let mut byte = [0u8; 1];
    file.read_exact(&mut byte)?;
    Ok(byte[0] == b'\n')
}

fn render_archive_entry(
    input: &WriteEvidenceInput,
    evidence_id: &str,
    jsonl_rel_path: &str,
    created_at: &str,
) -> String {
    let summary = input.summary.trim_end();
    let mut entry = String::new();
    entry.push_str(&format!(
        "## {} [{}] {}\n",
        created_at,
        input.source_kind.as_str(),
        summary
    ));
    entry.push_str(&format!("source_kind: {}\n", input.source_kind.as_str()));
    entry.push_str(&format!("session: {}\n", input.session_id));
    entry.push_str(&format!("jsonl_path: {}\n", jsonl_rel_path));
    entry.push_str(&format!("scope: {}\n", input.scope.scope_key()));
    entry.push_str(&format!("evidence_id: {}\n", evidence_id));
    entry.push_str(&format!("promotable: {}\n", input.promotable));
    entry.push_str(&format!("origin_mode: {}\n", input.origin_mode.as_str()));
    entry.push_str(&format!(
        "claim_fingerprint: {}\n",
        input
            .claim_fingerprint
            .as_deref()
            .unwrap_or("null")
    ));
    entry.push_str(&format!("created_at: {}\n", created_at));
    entry
}

fn content_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("sha256:{digest:x}")
}

fn current_timestamp() -> String {
    now_rfc3339()
}

fn current_archive_date() -> String {
    OffsetDateTime::now_utc().date().to_string()
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

fn relative_archive_path(scope: &ScopeRef, archive_date: &str) -> String {
    format!(
        "memory/{}/history/daily/{archive_date}.md",
        scope.scope_dir_fragment()
    )
}
