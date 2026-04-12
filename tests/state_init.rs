use memfold::config::MemfoldConfig;
use memfold::state::{schema, StateStore};
use rusqlite::Connection;
use tempfile::TempDir;

fn collect_names(conn: &Connection, sql: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect()
}

fn column_notnull(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info('{table}')"))?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, i64>(3)?))
    })?;

    for row in rows {
        let (name, notnull) = row?;
        if name == column {
            return Ok(notnull != 0);
        }
    }

    Ok(false)
}

#[test]
fn state_store_bootstraps_schema_and_indexes() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());

    let store = StateStore::initialize(&config).unwrap();
    let mut tables = store.list_tables().unwrap();
    tables.sort();

    assert_eq!(
        tables,
        vec![
            "boot_entries",
            "dream_jobs",
            "feedback_events",
            "lock_leases",
            "memory_items",
            "mutations",
            "retrieval_events",
            "session_log_entries",
            "sessions",
            "tombstones",
            "trace_archives",
        ]
    );

    let conn = Connection::open(config.state_db_path()).unwrap();
    let mut indexes = collect_names(
        &conn,
        "SELECT name FROM sqlite_master WHERE type = 'index' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .unwrap();
    indexes.sort();

    assert_eq!(
        indexes,
        vec![
            "idx_dream_jobs_status",
            "idx_memory_autoload",
            "idx_memory_claim_fingerprint",
            "idx_memory_scope_status",
            "idx_mutations_status",
            "idx_session_log_claim_fingerprint",
            "idx_session_log_promotable",
            "idx_session_log_session_created",
            "idx_sessions_scope",
            "idx_tombstones_claim_fingerprint",
            "idx_trace_scope_date",
        ]
    );

    assert!(column_notnull(&conn, "memory_items", "claim_fingerprint").unwrap());
    assert!(column_notnull(&conn, "memory_items", "content_hash").unwrap());
}

#[test]
fn apply_schema_is_idempotent() {
    let conn = Connection::open_in_memory().unwrap();

    schema::apply_schema(&conn).unwrap();
    schema::apply_schema(&conn).unwrap();

    let tables = collect_names(
        &conn,
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .unwrap();

    assert!(tables.contains(&"memory_items".to_string()));
    assert!(tables.contains(&"sessions".to_string()));
    assert!(column_notnull(&conn, "memory_items", "claim_fingerprint").unwrap());
    assert!(column_notnull(&conn, "memory_items", "content_hash").unwrap());
}

#[test]
fn apply_schema_migrates_legacy_evidence_items_into_session_log_entries() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
CREATE TABLE evidence_items (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    jsonl_path TEXT NOT NULL,
    line_no INTEGER NOT NULL,
    promotable INTEGER NOT NULL DEFAULT 0,
    origin_mode TEXT NOT NULL,
    claim_fingerprint TEXT,
    created_at TEXT NOT NULL
);
INSERT INTO evidence_items (
    id, session_id, scope_type, scope_id, source_kind, summary, jsonl_path, line_no,
    promotable, origin_mode, claim_fingerprint, created_at
) VALUES (
    'ev_legacy', 'sess_legacy', 'project', 'memfold', 'user', 'legacy summary',
    'memory/projects/memfold/sessions/sess_legacy/evidence.jsonl', 1, 1, 'normal',
    'cfp_legacy', '2026-04-12T00:00:00Z'
);
"#,
    )
    .unwrap();

    schema::apply_schema(&conn).unwrap();

    let migrated_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_log_entries WHERE id = 'ev_legacy'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(migrated_count, 1);

    let legacy_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM evidence_items WHERE id = 'ev_legacy'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(legacy_count, 1);

    let object_type: String = conn
        .query_row(
            "SELECT type FROM sqlite_master WHERE name = 'evidence_items'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(object_type, "view");
}
