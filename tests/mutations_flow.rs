use memfold::config::MemfoldConfig;
use memfold::domain::{ScopeRef, ScopeType};
use memfold::mutations::MutationStore;
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn read_mutation(conn: &Connection, mutation_id: &str) -> rusqlite::Result<(String, String, String)> {
    conn.query_row(
        "SELECT status, target_ref, idempotency_key FROM mutations WHERE id = ?1",
        params![mutation_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
}

#[test]
fn mutation_lifecycle_tracks_scope_affinity_and_stability() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let store = MutationStore::new(config.clone());

    let mutation_id = store
        .begin(&scope, "write", "notes/alpha.md", "idem-001")
        .unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    let (status, target_ref, idempotency_key) = read_mutation(&conn, &mutation_id).unwrap();

    assert_eq!(status, "pending");
    assert_eq!(target_ref, format!("{}::{}", scope.scope_key(), "notes/alpha.md"));
    assert_eq!(idempotency_key, "idem-001");
    assert!(store.has_unstable_scope_mutations(&scope).unwrap());

    store.mark_applied_to_content(&mutation_id).unwrap();
    let (status, _, _) = read_mutation(&conn, &mutation_id).unwrap();
    assert_eq!(status, "applied_to_content");
    assert!(store.has_unstable_scope_mutations(&scope).unwrap());

    store.mark_fully_applied(&mutation_id).unwrap();
    let (status, _, _) = read_mutation(&conn, &mutation_id).unwrap();
    assert_eq!(status, "fully_applied");
    assert!(!store.has_unstable_scope_mutations(&scope).unwrap());
}
