use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType, SourceKind};
use memfold::dreaming::maybe_run_scheduled_dream;
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::init::initialize_root;
use rusqlite::{params, Connection};
use tempfile::TempDir;
use time::Duration;
use time::OffsetDateTime;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

fn mark_sessions_ended(conn: &Connection, scope: &ScopeRef, count: usize, ended_at: &str) {
    for idx in 0..count {
        conn.execute(
            "INSERT INTO sessions (id, scope_type, scope_id, mode, intent, host, started_at, ended_at, evidence_count)
             VALUES (?1, ?2, ?3, 'normal', 'continue', NULL, ?4, ?4, 1)",
            params![
                format!("sched_{idx}"),
                scope.scope_type.as_str(),
                &scope.scope_id,
                ended_at,
            ],
        )
        .unwrap();
    }
}

#[test]
fn scheduled_dream_runs_only_after_five_ended_sessions() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sched_seed".to_string(),
            source_kind: SourceKind::User,
            summary: "scheduled dream candidate".to_string(),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_sched".to_string()),
        },
    )
    .unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    mark_sessions_ended(&conn, &scope, 4, "2026-04-10T00:00:00Z");

    let not_yet = maybe_run_scheduled_dream(&config, &scope).unwrap();
    assert!(!not_yet.ran);
    assert!(not_yet.reason.starts_with("insufficient_sessions"));

    conn.execute(
        "INSERT INTO sessions (id, scope_type, scope_id, mode, intent, host, started_at, ended_at, evidence_count)
         VALUES ('sched_4', ?1, ?2, 'normal', 'continue', NULL, '2026-04-10T00:00:00Z', '2026-04-10T00:00:00Z', 1)",
        params![scope.scope_type.as_str(), &scope.scope_id],
    )
    .unwrap();

    let ran = maybe_run_scheduled_dream(&config, &scope).unwrap();
    assert!(ran.ran);
    assert_eq!(ran.result.unwrap().promoted, 1);
}

#[test]
fn scheduled_dream_respects_twenty_four_hour_cooldown() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    let last_applied = (OffsetDateTime::now_utc() - Duration::hours(1)).to_string();
    let ended_at = OffsetDateTime::now_utc().to_string();
    mark_sessions_ended(&conn, &scope, 5, &ended_at);
    conn.execute(
        "INSERT INTO dream_jobs (id, scope_type, scope_id, trigger, status, promoted, held, quarantined, discarded, created_at, updated_at)
         VALUES ('dj_recent', ?1, ?2, 'scheduled', 'applied', 1, 0, 0, 0, ?3, ?3)",
        params![scope.scope_type.as_str(), &scope.scope_id, last_applied],
    )
    .unwrap();

    let result = maybe_run_scheduled_dream(&config, &scope).unwrap();
    assert!(!result.ran);
    assert!(result.reason.starts_with("cooldown_hours"));
}
