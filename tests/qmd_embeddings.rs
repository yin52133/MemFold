use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, ScopeRef, ScopeType, SourceKind, Mode};
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::init::initialize_root;
use memfold::qmd_adapter::{init_model, load_scope_records, sync_scope};
use memfold::retrieval::search_memories;
use rusqlite::Connection;
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

fn write_stable_file(path: &std::path::Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

#[test]
fn qmd_init_model_writes_config_and_sync_adds_embeddings() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let initialized = init_model(&config, "mock-test").unwrap();
    assert!(initialized.enabled);
    assert_eq!(initialized.model, "mock-test");

    let stable_dir = root.join("memory").join("projects").join("memfold").join("stable");
    write_stable_file(
        &stable_dir.join("rules.md"),
        "## item_key: project.rule.embed\n\
title: Embed rule\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_embed\n\
content_hash: sha256:embed\n\
revision: 1\n\n\
embedding aware memory search\n",
    );

    let conn = Connection::open(config.state_db_path()).unwrap();
    conn.execute(
        "INSERT INTO memory_items (
            id, scope_type, scope_id, item_key, file_path, title, status, autoload,
            claim_fingerprint, content_hash, revision, supersedes_id, trust_score,
            freshness_score, created_at, updated_at, deleted_at
        ) VALUES (
            'mem_embed', 'project', 'memfold', 'project.rule.embed', 'memory/projects/memfold/stable/rules.md',
            'Embed rule', 'stable', 'boot_project', 'cfp_embed', 'sha256:embed', 1, NULL, 1.0, 1.0,
            '2026-04-12T00:00:00Z', '2026-04-12T00:00:00Z', NULL
        )",
        [],
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let records = load_scope_records(&config, &scope).unwrap();
    assert!(records.iter().any(|record| record.embedding.as_ref().is_some()));
}

#[test]
fn embedding_enabled_search_still_returns_results() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    init_model(&config, "mock-test").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_embed".to_string(),
            source_kind: SourceKind::User,
            summary: "embedding aware retrieval note".to_string(),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_embed_evidence".to_string()),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "embedding retrieval", 50).unwrap();
    assert!(!result.results.is_empty());
}
