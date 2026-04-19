use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType, SourceKind};
use memfold::dreaming::run_dream;
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::feedback::apply_feedback;
use memfold::history::{SummarizeHistoryInput, summarize_history};
use memfold::init::initialize_root;
use memfold::qmd_adapter::load_scope_records;
use memfold::retrieval::search_memories;
use memfold::domain::Intent;
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn dream_promotes_promotable_evidence_and_feedback_rejects_with_tombstone() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let written = write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_dream".to_string(),
            source_kind: SourceKind::User,
            summary: "用户明确要求默认用中文回答".to_string(),
            raw_text: Some("以后默认用中文回答。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_feedback_language".to_string()),
        },
    )
    .unwrap();
    assert!(written.stored);

    let first_run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(first_run.promoted, 1);
    assert_eq!(first_run.discarded, 0);

    let qmd_records = load_scope_records(&config, &scope).unwrap();
    assert!(
        qmd_records.iter().any(|record| {
            record.source_type == "stable" && record.summary.contains("默认用中文回答")
        }),
        "dream should refresh qmd after promoting stable memory"
    );

    let conn = Connection::open(config.state_db_path()).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM memory_items WHERE claim_fingerprint = ?1",
            params!["cfp_feedback_language"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "stable");

    let stable_file = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("stable")
        .join("rules.md");
    assert!(stable_file.exists());
    assert!(fs::read_to_string(&stable_file)
        .unwrap()
        .contains("默认用中文回答"));

    let feedback = apply_feedback(
        &config,
        &scope,
        "cfp_feedback_language",
        "rejected",
        "这条记忆不对",
        Some("sess_feedback"),
    )
    .unwrap();
    assert!(feedback.updated);
    assert!(feedback.tombstone_written);

    let status: String = conn
        .query_row(
            "SELECT status FROM memory_items WHERE claim_fingerprint = ?1",
            params!["cfp_feedback_language"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "rejected");

    let tombstone_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tombstones WHERE claim_fingerprint = ?1",
            params!["cfp_feedback_language"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tombstone_count, 1);

    let second_run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(second_run.promoted, 0);
    assert!(second_run.discarded >= 1);
}

#[test]
fn dream_discards_sterile_evidence() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_sterile".to_string(),
            source_kind: SourceKind::User,
            summary: "sterile memory should not promote".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Sterile,
            claim_fingerprint: Some("cfp_sterile".to_string()),
        },
    )
    .unwrap();

    let run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(run.promoted, 0);
    assert!(run.discarded >= 1);

    let conn = Connection::open(config.state_db_path()).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_items WHERE claim_fingerprint = ?1",
            params!["cfp_sterile"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn dream_discards_untraceable_user_claim_without_raw_text() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_untraceable".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认用中文回答".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_untraceable".to_string()),
        },
    )
    .unwrap();

    let run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(run.promoted, 0);
    assert!(run.discarded >= 1);

    let conn = Connection::open(config.state_db_path()).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memory_items WHERE claim_fingerprint = ?1",
            params!["cfp_untraceable"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn feedback_rejects_evidence_only_claim_before_promotion() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_pre".to_string(),
            source_kind: SourceKind::User,
            summary: "先记一条会被否定的偏好".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_pre".to_string()),
        },
    )
    .unwrap();

    let feedback = apply_feedback(
        &config,
        &scope,
        "cfp_pre",
        "rejected",
        "这条不对",
        Some("sess_pre"),
    )
    .unwrap();
    assert!(feedback.updated);
    assert!(feedback.tombstone_written);

    let conn = Connection::open(config.state_db_path()).unwrap();
    let tombstone_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tombstones WHERE claim_fingerprint = ?1",
            params!["cfp_pre"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tombstone_count, 1);

    let run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(run.promoted, 0);
    assert!(run.discarded >= 1);
}

#[test]
fn feedback_reject_does_not_duplicate_tombstones_for_same_claim() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_repeat_reject".to_string(),
            source_kind: SourceKind::User,
            summary: "重复拒绝不应生成多个 tombstone".to_string(),
            raw_text: Some("重复拒绝不应生成多个 tombstone".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_repeat_reject".to_string()),
        },
    )
    .unwrap();

    let first = apply_feedback(
        &config,
        &scope,
        "cfp_repeat_reject",
        "rejected",
        "第一次拒绝",
        Some("sess_repeat_reject"),
    )
    .unwrap();
    assert!(first.tombstone_written);

    let second = apply_feedback(
        &config,
        &scope,
        "cfp_repeat_reject",
        "rejected",
        "第二次拒绝",
        Some("sess_repeat_reject"),
    )
    .unwrap();
    assert!(!second.tombstone_written);

    let conn = Connection::open(config.state_db_path()).unwrap();
    let tombstone_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tombstones WHERE claim_fingerprint = 'cfp_repeat_reject'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(tombstone_count, 1);
}

#[test]
fn rejected_claims_do_not_appear_in_search_results() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_rejected_search".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认用中文回答".to_string(),
            raw_text: Some("以后默认用中文回答。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_rejected_search".to_string()),
        },
    )
    .unwrap();

    let run = run_dream(&config, &scope, "manual").unwrap();
    assert_eq!(run.promoted, 1);

    let before = search_memories(&config, &scope, Intent::Continue, "默认用中文", 80).unwrap();
    assert!(!before.results.is_empty());

    let feedback = apply_feedback(
        &config,
        &scope,
        "cfp_rejected_search",
        "rejected",
        "这条记忆不对",
        Some("sess_rejected_search"),
    )
    .unwrap();
    assert!(feedback.updated);

    let after = search_memories(&config, &scope, Intent::Continue, "默认用中文", 80).unwrap();
    assert!(after.results.is_empty());
}

#[test]
fn repair_rebuilds_trace_archives_projection() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_repair".to_string(),
            source_kind: SourceKind::User,
            summary: "repair trace archive case".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_repair".to_string()),
        },
    )
    .unwrap();
    summarize_history(
        &config,
        &SummarizeHistoryInput {
            scope: scope.clone(),
            session_id: "sess_repair".to_string(),
            trigger: "session_end".to_string(),
        },
    )
    .unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    let before: i64 = conn
        .query_row("SELECT COUNT(*) FROM trace_archives", [], |row| row.get(0))
        .unwrap();
    assert_eq!(before, 1);

    let repaired = memfold::repair::run_repair(&config, Some(&scope)).unwrap();
    assert!(repaired.repaired);

    let after: i64 = conn
        .query_row("SELECT COUNT(*) FROM trace_archives", [], |row| row.get(0))
        .unwrap();
    assert_eq!(after, 1);

    let history_dir = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("history")
        .join("daily");
    assert!(history_dir.exists());
}
