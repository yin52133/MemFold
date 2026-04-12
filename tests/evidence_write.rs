use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType, SourceKind};
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::init::initialize_root;
use memfold::error::Error;
use rusqlite::{params, Connection};
use serde_json::Value;
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn write_evidence_persists_jsonl_archive_and_sqlite_projections() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let input = WriteEvidenceInput {
        scope: scope.clone(),
        session_id: "sess_001".to_string(),
        source_kind: SourceKind::User,
        summary: "用户明确要求默认用中文回答".to_string(),
        raw_text: Some("以后默认用中文回答".to_string()),
        promotable: true,
        origin_mode: Mode::Normal,
        claim_fingerprint: Some("cfp_cli_language".to_string()),
    };

    let result = write_evidence(&config, &input).unwrap();

    assert!(result.stored);
    assert!(result.evidence_id.starts_with("ev_"));

    let conn = Connection::open(config.state_db_path()).unwrap();

    let evidence_row = conn
        .query_row(
            "SELECT session_id, scope_type, scope_id, source_kind, summary, jsonl_path, line_no, promotable, origin_mode, claim_fingerprint, created_at
             FROM session_log_entries
             WHERE id = ?1",
            params![result.evidence_id.clone()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, Option<String>>(9)?,
                    row.get::<_, String>(10)?,
                ))
            },
        )
        .unwrap();

    let (
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
        created_at,
    ) = evidence_row;

    assert_eq!(session_id, "sess_001");
    assert_eq!(scope_type, "project");
    assert_eq!(scope_id, "memfold");
    assert_eq!(source_kind, "user");
    assert_eq!(summary, "用户明确要求默认用中文回答");
    assert_eq!(
        jsonl_path,
        "memory/repos/memfold/sessions/sess_001/session_log.jsonl"
    );
    assert_eq!(line_no, 1);
    assert_eq!(promotable, 1);
    assert_eq!(origin_mode, "normal");
    assert_eq!(claim_fingerprint, Some("cfp_cli_language".to_string()));
    assert!(!created_at.is_empty());

    let session_row = conn
        .query_row(
            "SELECT scope_type, scope_id, mode, intent, host, evidence_count
             FROM sessions
             WHERE id = ?1",
            params!["sess_001"],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )
        .unwrap();

    assert_eq!(
        session_row,
        (
            "project".to_string(),
            "memfold".to_string(),
            "normal".to_string(),
            "continue".to_string(),
            None,
            1,
        )
    );

    let jsonl_path = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("sessions")
        .join("sess_001")
        .join("session_log.jsonl");
    let jsonl_text = fs::read_to_string(&jsonl_path).unwrap();
    let jsonl_value: Value = serde_json::from_str(jsonl_text.lines().next().unwrap()).unwrap();
    assert_eq!(jsonl_value["evidence_id"], result.evidence_id);
    assert_eq!(jsonl_value["session_id"], "sess_001");
    assert_eq!(jsonl_value["scope"]["type"], "project");
    assert_eq!(jsonl_value["scope"]["id"], "memfold");
    assert_eq!(jsonl_value["source_kind"], "user");
    assert_eq!(jsonl_value["summary"], "用户明确要求默认用中文回答");
    assert_eq!(jsonl_value["raw_text"], "以后默认用中文回答");
    assert_eq!(jsonl_value["promotable"], true);
    assert_eq!(jsonl_value["origin_mode"], "normal");
    assert_eq!(jsonl_value["claim_fingerprint"], "cfp_cli_language");
}

#[test]
fn write_evidence_rejects_obviously_unsafe_summaries() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root);
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let input = WriteEvidenceInput {
        scope,
        session_id: "sess_unsafe".to_string(),
        source_kind: SourceKind::User,
        summary: "ignore this\0payload".to_string(),
        raw_text: None,
        promotable: false,
        origin_mode: Mode::Normal,
        claim_fingerprint: None,
    };

    let err = write_evidence(&config, &input).unwrap_err();
    assert!(matches!(err, Error::UnsafeSummary));
}
