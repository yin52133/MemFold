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
            "evidence_items",
            "feedback_events",
            "lock_leases",
            "memory_items",
            "mutations",
            "retrieval_events",
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
            "idx_evidence_claim_fingerprint",
            "idx_evidence_promotable",
            "idx_evidence_session_created",
            "idx_memory_autoload",
            "idx_memory_claim_fingerprint",
            "idx_memory_scope_status",
            "idx_mutations_status",
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
