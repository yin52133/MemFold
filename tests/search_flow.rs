use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, ScopeRef, ScopeType, SourceKind, Mode};
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::history::{SummarizeHistoryInput, summarize_history};
use memfold::init::initialize_root;
use memfold::qmd_adapter::{load_scope_records, sync_scope};
use memfold::retrieval::{search_memories, SearchResponse};
use memfold::state::StateStore;
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

fn init_config() -> (TempDir, MemfoldConfig) {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(memfold_root(&tmp));
    initialize_root(&config).unwrap();
    (tmp, config)
}

fn insert_memory_item(
    conn: &Connection,
    id: &str,
    scope_type: &str,
    scope_id: &str,
    item_key: &str,
    file_path: &str,
    summary: &str,
    updated_at: &str,
) {
    conn.execute(
        "INSERT INTO memory_items (
            id, scope_type, scope_id, item_key, file_path, title, status, autoload,
            claim_fingerprint, content_hash, revision, supersedes_id, trust_score,
            freshness_score, created_at, updated_at, deleted_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, 'stable', 'boot_project',
            ?7, ?8, 1, NULL, 1.0,
            1.0, ?9, ?10, NULL
        )",
        params![
            id,
            scope_type,
            scope_id,
            item_key,
            file_path,
            summary,
            format!("cfp_{id}"),
            format!("sha256:{id}"),
            updated_at,
            updated_at,
        ],
    )
    .unwrap();
}

fn write_stable_file(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn write_supporting_content(config: &MemfoldConfig, scope: &ScopeRef) {
    let stable_dir = config.root.join("memory").join("repos").join(&scope.scope_id).join("stable");
    write_stable_file(
        &stable_dir.join("card.md"),
        "## item_key: project.rule.alpha\n\
title: Alpha rule\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_alpha\n\
content_hash: sha256:alpha\n\
revision: 1\n\n\
stable alpha memory\n",
    );
    write_stable_file(
        &stable_dir.join("notes.md"),
        "## item_key: project.rule.beta\n\
title: Beta rule\n\
status: candidate\n\
autoload: boot_project\n\
claim_fingerprint: cfp_beta\n\
content_hash: sha256:beta\n\
revision: 1\n\n\
ignored beta memory\n",
    );
}

#[test]
fn sync_scope_builds_sidecar_records_for_stable_evidence_and_archive_sources() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let conn = Connection::open(config.state_db_path()).unwrap();
    StateStore::initialize(&config).unwrap();

    write_supporting_content(&config, &scope);
    insert_memory_item(
        &conn,
        "mem_alpha",
        "project",
        "memfold",
        "project.rule.alpha",
        "memory/repos/memfold/stable/card.md",
        "Alpha rule",
        "2026-04-12T10:00:00Z",
    );

    let evidence = write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_search".to_string(),
            source_kind: SourceKind::User,
            summary: "shared gamma evidence".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_gamma".to_string()),
        },
    )
    .unwrap();
    summarize_history(
        &config,
        &SummarizeHistoryInput {
            scope: scope.clone(),
            session_id: "sess_search".to_string(),
            trigger: "session_end".to_string(),
        },
    )
    .unwrap();

    let synced = sync_scope(&config, &scope).unwrap();
    assert_eq!(synced, 3);

    let records = load_scope_records(&config, &scope).unwrap();
    assert_eq!(records.len(), 3);
    assert!(records.iter().any(|record| record.source_type == "stable"));
    assert!(records.iter().any(|record| record.source_type == "session_log"));
    assert!(records.iter().any(|record| record.source_type == "history"));

    let stable = records.iter().find(|record| record.source_type == "stable").unwrap();
    assert_eq!(stable.doc_id, "mem_alpha");
    assert_eq!(stable.scope_type, "project");
    assert_eq!(stable.scope_id, "memfold");
    assert_eq!(stable.relative_path, "memory/repos/memfold/stable/card.md");
    assert_eq!(stable.pointer, "memory/repos/memfold/stable/card.md#project.rule.alpha");
    assert_eq!(stable.status, "stable");
    assert_eq!(stable.summary, "stable alpha memory");
    assert_eq!(stable.updated_at, "2026-04-12T10:00:00Z");

    let evidence_record = records
        .iter()
        .find(|record| record.source_type == "session_log")
        .unwrap();
    assert_eq!(evidence_record.doc_id, evidence.evidence_id);
    assert_eq!(
        evidence_record.relative_path,
        "memory/repos/memfold/sessions/sess_search/session_log.jsonl"
    );
    assert!(evidence_record.pointer.ends_with("#1"));
    assert_eq!(evidence_record.summary, "shared gamma evidence");
    assert_eq!(evidence_record.status, "promotable");

    let history_record = records
        .iter()
        .find(|record| record.source_type == "history")
        .unwrap();
    assert!(history_record.summary.contains("shared gamma evidence"));
}

#[test]
fn search_memories_auto_syncs_missing_qmd_and_orders_by_intent() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let conn = Connection::open(config.state_db_path()).unwrap();

    write_supporting_content(&config, &scope);
    insert_memory_item(
        &conn,
        "mem_alpha",
        "project",
        "memfold",
        "project.rule.alpha",
        "memory/repos/memfold/stable/card.md",
        "Alpha rule",
        "2026-04-12T10:00:00Z",
    );

    let _ = write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_search".to_string(),
            source_kind: SourceKind::User,
            summary: "shared gamma evidence".to_string(),
            raw_text: None,
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_gamma".to_string()),
        },
    )
    .unwrap();
    summarize_history(
        &config,
        &SummarizeHistoryInput {
            scope: scope.clone(),
            session_id: "sess_search".to_string(),
            trigger: "session_end".to_string(),
        },
    )
    .unwrap();

    let continue_results: SearchResponse = search_memories(
        &config,
        &scope,
        Intent::Continue,
        "gamma",
        10,
    )
    .unwrap();
    assert!(!continue_results.results.is_empty());
    assert_eq!(continue_results.results[0].source_type, "history");

    let knowledge_results = search_memories(
        &config,
        &scope,
        Intent::KnowledgeLookup,
        "gamma",
        10,
    )
    .unwrap();
    assert!(!knowledge_results.results.is_empty());
    assert_eq!(knowledge_results.results[0].source_type, "history");

    let sidecar = config
        .root
        .join("qmd")
        .join("collections")
        .join("repos")
        .join("memfold")
        .join("stable.jsonl");
    assert!(sidecar.exists());
}

#[test]
fn sync_scope_skips_noisy_session_and_history_records() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let session_dir = config
        .root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("sessions")
        .join("sess_noise");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("session_log.jsonl"),
        concat!(
            "{\"evidence_id\":\"ev_noise\",\"scope\":{\"type\":\"project\",\"id\":\"memfold\"},\"session_id\":\"sess_noise\",\"source_kind\":\"decision\",\"summary\":\"session exited via launcher trap\",\"raw_text\":null,\"promotable\":false,\"origin_mode\":\"normal\",\"claim_fingerprint\":null,\"created_at\":\"2026-04-12T18:45:56Z\"}\n",
            "{\"evidence_id\":\"ev_keep\",\"scope\":{\"type\":\"project\",\"id\":\"memfold\"},\"session_id\":\"sess_noise\",\"source_kind\":\"user\",\"summary\":\"用户要求默认中文\",\"raw_text\":\"以后默认用中文回答\",\"promotable\":true,\"origin_mode\":\"normal\",\"claim_fingerprint\":\"cfp_keep\",\"created_at\":\"2026-04-12T18:46:00Z\"}\n"
        ),
    )
    .unwrap();

    let history_dir = config
        .root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("history")
        .join("daily");
    fs::create_dir_all(&history_dir).unwrap();
    fs::write(
        history_dir.join("2026-04-12.md"),
        concat!(
            "<!-- session: sess_noise_1 | summary_id: hs_noise -->\n",
            "## 2026-04-12T18:45:56Z [session_end] session exited via launcher trap\n",
            "- [decision] session exited via launcher trap\n\n",
            "<!-- session: sess_noise_2 | summary_id: hs_keep -->\n",
            "## 2026-04-12T18:46:00Z [manual] 用户要求默认中文\n",
            "- [user] 用户要求默认中文\n"
        ),
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let records = load_scope_records(&config, &scope).unwrap();

    assert!(records.iter().any(|record| record.summary.contains("用户要求默认中文")));
    assert!(
        !records
            .iter()
            .any(|record| record.summary.contains("session exited via launcher trap"))
    );
}
