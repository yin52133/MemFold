use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType, SourceKind};
use memfold::hooks::{capture_event, HookCaptureInput, HookEvent};
use memfold::init::initialize_root;
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn hook_capture_skips_noop_and_noise_events() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root);
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let skipped = capture_event(
        &config,
        &HookCaptureInput {
            event: HookEvent::TurnEnd,
            scope: scope.clone(),
            session_id: "sess_hook".to_string(),
            source_kind: SourceKind::Decision,
            summary: "no changes".to_string(),
            origin_mode: Mode::Normal,
            state_changed: true,
            promotable: false,
        },
    )
    .unwrap();
    assert!(!skipped.recorded);
    assert_eq!(skipped.reason, "filtered_noise");

    let skipped = capture_event(
        &config,
        &HookCaptureInput {
            event: HookEvent::TurnEnd,
            scope,
            session_id: "sess_hook".to_string(),
            source_kind: SourceKind::Decision,
            summary: "real summary but no state change".to_string(),
            origin_mode: Mode::Normal,
            state_changed: false,
            promotable: false,
        },
    )
    .unwrap();
    assert!(!skipped.recorded);
    assert_eq!(skipped.reason, "no_state_change");
}

#[test]
fn hook_capture_records_meaningful_event_once() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let first = capture_event(
        &config,
        &HookCaptureInput {
            event: HookEvent::TurnEnd,
            scope: scope.clone(),
            session_id: "sess_hook".to_string(),
            source_kind: SourceKind::Decision,
            summary: "完成  Wave 2   CLI 接线".to_string(),
            origin_mode: Mode::Normal,
            state_changed: true,
            promotable: false,
        },
    )
    .unwrap();
    assert!(first.recorded);
    assert_eq!(first.normalized_summary.as_deref(), Some("完成 Wave 2 CLI 接线"));

    let duplicate = capture_event(
        &config,
        &HookCaptureInput {
            event: HookEvent::TurnEnd,
            scope: scope.clone(),
            session_id: "sess_hook".to_string(),
            source_kind: SourceKind::Decision,
            summary: "完成 Wave 2 CLI 接线".to_string(),
            origin_mode: Mode::Normal,
            state_changed: true,
            promotable: false,
        },
    )
    .unwrap();
    assert!(!duplicate.recorded);
    assert_eq!(duplicate.reason, "duplicate_summary");

    let conn = Connection::open(root.join("state").join("memfold.db")).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM evidence_items WHERE session_id = ?1 AND summary = ?2",
            params!["sess_hook", "完成 Wave 2 CLI 接线"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
