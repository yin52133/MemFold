use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, ScopeRef, ScopeType, SourceKind, Mode};
use memfold::evidence::{write_evidence, WriteEvidenceInput};
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
    let stable_dir = config.root.join("memory").join("projects").join(&scope.scope_id).join("stable");
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
        "memory/projects/memfold/stable/card.md",
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
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_gamma".to_string()),
        },
    )
    .unwrap();

    let synced = sync_scope(&config, &scope).unwrap();
    assert_eq!(synced, 3);

    let records = load_scope_records(&config, &scope).unwrap();
    assert_eq!(records.len(), 3);
    assert!(records.iter().any(|record| record.source_type == "stable"));
    assert!(records.iter().any(|record| record.source_type == "evidence"));
    assert!(records.iter().any(|record| record.source_type == "archive"));

    let stable = records.iter().find(|record| record.source_type == "stable").unwrap();
    assert_eq!(stable.doc_id, "mem_alpha");
    assert_eq!(stable.scope_type, "project");
    assert_eq!(stable.scope_id, "memfold");
    assert_eq!(stable.relative_path, "memory/projects/memfold/stable/card.md");
    assert_eq!(stable.pointer, "memory/projects/memfold/stable/card.md#project.rule.alpha");
    assert_eq!(stable.status, "stable");
    assert_eq!(stable.summary, "stable alpha memory");
    assert_eq!(stable.updated_at, "2026-04-12T10:00:00Z");

    let evidence_record = records
        .iter()
        .find(|record| record.source_type == "evidence")
        .unwrap();
    assert_eq!(evidence_record.doc_id, evidence.evidence_id);
    assert_eq!(
        evidence_record.relative_path,
        "memory/projects/memfold/sessions/sess_search/evidence.jsonl"
    );
    assert!(evidence_record.pointer.ends_with("#1"));
    assert_eq!(evidence_record.summary, "shared gamma evidence");
    assert_eq!(evidence_record.status, "promotable");
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
        "memory/projects/memfold/stable/card.md",
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
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_gamma".to_string()),
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
    assert_eq!(continue_results.results[0].source_type, "evidence");

    let knowledge_results = search_memories(
        &config,
        &scope,
        Intent::KnowledgeLookup,
        "gamma",
        10,
    )
    .unwrap();
    assert!(!knowledge_results.results.is_empty());
    assert_eq!(knowledge_results.results[0].source_type, "archive");

    let sidecar = config
        .root
        .join("qmd")
        .join("collections")
        .join("projects")
        .join("memfold")
        .join("stable.jsonl");
    assert!(sidecar.exists());
}
