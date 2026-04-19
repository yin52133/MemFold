use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, ScopeRef, ScopeType, SourceKind, Mode};
use memfold::evidence::{write_evidence, WriteEvidenceInput};
use memfold::history::{SummarizeHistoryInput, summarize_history};
use memfold::init::initialize_root;
use memfold::qmd_adapter::{embed_query, init_model, load_scope_records, sync_scope, QmdRecord};
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
fn sync_scope_history_doc_ids_are_unique_across_days() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let history_dir = config
        .root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("history")
        .join("daily");
    fs::create_dir_all(&history_dir).unwrap();
    fs::write(
        history_dir.join("2026-04-18.md"),
        "<!-- session: s1 | summary_id: hs_1 -->\n## 2026-04-18T12:00:00Z [manual] first day summary\n- [decision] first day summary\n",
    )
    .unwrap();
    fs::write(
        history_dir.join("2026-04-19.md"),
        "<!-- session: s2 | summary_id: hs_2 -->\n## 2026-04-19T12:00:00Z [manual] second day summary\n- [decision] second day summary\n",
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let records = load_scope_records(&config, &scope).unwrap();
    let history_ids = records
        .iter()
        .filter(|record| record.source_type == "history")
        .map(|record| record.doc_id.clone())
        .collect::<Vec<_>>();

    assert_eq!(history_ids.len(), 2);
    assert_ne!(history_ids[0], history_ids[1]);
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

#[test]
fn search_memories_deduplicates_equivalent_summaries_across_layers() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let conn = Connection::open(config.state_db_path()).unwrap();

    write_supporting_content(&config, &scope);
    insert_memory_item(
        &conn,
        "mem_clean_truth",
        "project",
        "memfold",
        "project.memory.clean_truth",
        "memory/repos/memfold/stable/card.md",
        "用户要求默认中文",
        "2026-04-12T10:00:00Z",
    );
    write_stable_file(
        &config
            .root
            .join("memory")
            .join("repos")
            .join("memfold")
            .join("stable")
            .join("card.md"),
        "## item_key: project.memory.clean_truth\n\
title: Clean truth\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_clean_truth\n\
content_hash: sha256:cleantruth\n\
revision: 1\n\n\
用户要求默认中文\n",
    );

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_dedupe".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认中文".to_string(),
            raw_text: Some("以后默认用中文回答".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_clean_truth".to_string()),
        },
    )
    .unwrap();
    summarize_history(
        &config,
        &SummarizeHistoryInput {
            scope: scope.clone(),
            session_id: "sess_dedupe".to_string(),
            trigger: "session_end".to_string(),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "中文 默认", 100).unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].source_type, "stable");
    assert_eq!(result.results[0].summary, "用户要求默认中文");
}

#[test]
fn search_memories_rejects_moderate_semantic_match_without_lexical_overlap() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    init_model(&config, "mock-test").unwrap();

    let query = "launcher trap";
    let query_embedding = embed_query(&config, query).unwrap().unwrap();
    let doc_embedding = embedding_with_target_cosine(&query_embedding, 0.71);
    write_qmd_records(
        &config,
        &scope,
        "stable",
        &[QmdRecord {
            doc_id: "mem_false_positive".to_string(),
            source_type: "stable".to_string(),
            relative_path: "memory/repos/memfold/stable/rules.md".to_string(),
            pointer: "memory/repos/memfold/stable/rules.md#project.memory.clean_truth".to_string(),
            summary: "用户要求默认中文".to_string(),
            raw_text: None,
            claim_fingerprint: None,
            history_has_user_signal: None,
            status: "stable".to_string(),
            scope_type: "project".to_string(),
            scope_id: "memfold".to_string(),
            updated_at: "2026-04-19T12:00:00Z".to_string(),
            embedding: Some(doc_embedding),
        }],
    );

    let result = search_memories(&config, &scope, Intent::Continue, query, 50).unwrap();
    assert!(result.results.is_empty());
}

#[test]
fn search_memories_keeps_high_confidence_semantic_match_without_lexical_overlap() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    init_model(&config, "mock-test").unwrap();

    let query = "launcher trap";
    let query_embedding = embed_query(&config, query).unwrap().unwrap();
    let doc_embedding = embedding_with_target_cosine(&query_embedding, 0.97);
    write_qmd_records(
        &config,
        &scope,
        "stable",
        &[QmdRecord {
            doc_id: "mem_true_positive".to_string(),
            source_type: "stable".to_string(),
            relative_path: "memory/repos/memfold/stable/rules.md".to_string(),
            pointer: "memory/repos/memfold/stable/rules.md#project.memory.semantic".to_string(),
            summary: "launch lifecycle shutdown signal handling".to_string(),
            raw_text: None,
            claim_fingerprint: None,
            history_has_user_signal: None,
            status: "stable".to_string(),
            scope_type: "project".to_string(),
            scope_id: "memfold".to_string(),
            updated_at: "2026-04-19T12:00:00Z".to_string(),
            embedding: Some(doc_embedding),
        }],
    );

    let result = search_memories(&config, &scope, Intent::Continue, query, 50).unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].doc_id, "mem_true_positive");
}

#[test]
fn search_memories_matches_session_raw_text_when_summary_is_paraphrased() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_raw_text".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认中文".to_string(),
            raw_text: Some("以后不要英文，直接用中文回答我。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_raw_search".to_string()),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "不要英文 直接用中文", 80).unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].source_type, "session_log");
    assert_eq!(result.results[0].summary, "用户要求默认中文");
}

#[test]
fn search_memories_prefers_stable_memory_when_raw_text_matches_same_claim() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let conn = Connection::open(config.state_db_path()).unwrap();

    write_stable_file(
        &config
            .root
            .join("memory")
            .join("repos")
            .join("memfold")
            .join("stable")
            .join("rules.md"),
        "## item_key: project.memory.clean_truth\n\
title: Clean truth\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_clean_truth\n\
content_hash: sha256:cleantruth\n\
revision: 1\n\n\
用户要求默认中文\n",
    );
    insert_memory_item(
        &conn,
        "mem_clean_truth",
        "project",
        "memfold",
        "project.memory.clean_truth",
        "memory/repos/memfold/stable/rules.md",
        "Clean truth",
        "2026-04-19T12:00:00Z",
    );

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_raw_text_stable".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认中文".to_string(),
            raw_text: Some("以后不要英文，直接用中文回答我。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_clean_truth".to_string()),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "不要英文 直接用中文", 80).unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].source_type, "stable");
    assert_eq!(result.results[0].summary, "用户要求默认中文");
}

#[test]
fn search_memories_ignores_cjk_punctuation_during_lexical_match() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_cjk_punct".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认中文".to_string(),
            raw_text: Some("以后不要英文，直接用中文回答我。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_cjk_punct".to_string()),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result =
        search_memories(&config, &scope, Intent::Continue, "不要英文直接用中文", 80).unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].source_type, "session_log");
}

#[test]
fn search_memories_prefers_exact_hyphenated_token_match_over_generic_history_hits() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let history_dir = config
        .root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("history")
        .join("daily");
    fs::create_dir_all(&history_dir).unwrap();
    fs::write(
        history_dir.join("2026-04-19.md"),
        concat!(
            "<!-- session: hist_1 | summary_id: hs_1 -->\n",
            "## 2026-04-19T12:00:00Z [manual] MemFold launcher verify 1776603189\n",
            "- [decision] MemFold launcher verify 1776603189\n\n",
            "<!-- session: hist_2 | summary_id: hs_2 -->\n",
            "## 2026-04-19T12:01:00Z [manual] MemFold launcher verify 1776603210\n",
            "- [decision] MemFold launcher verify 1776603210\n"
        ),
    )
    .unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_exact_token".to_string(),
            source_kind: SourceKind::Decision,
            summary: "MemFold hook verify verify-token-abc123".to_string(),
            raw_text: None,
            promotable: false,
            origin_mode: Mode::Normal,
            claim_fingerprint: None,
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "verify-token-abc123", 80).unwrap();

    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].source_type, "session_log");
    assert_eq!(result.results[0].summary, "MemFold hook verify verify-token-abc123");
}

#[test]
fn search_memories_keeps_related_non_exact_results_for_natural_language_queries() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    let history_dir = config
        .root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("history")
        .join("daily");
    fs::create_dir_all(&history_dir).unwrap();
    fs::write(
        history_dir.join("2026-04-19.md"),
        concat!(
            "<!-- session: hist_1 | summary_id: hs_1 -->\n",
            "## 2026-04-19T12:00:00Z [manual] respond in chinese by default\n",
            "- [decision] respond in chinese by default\n"
        ),
    )
    .unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_natural_query".to_string(),
            source_kind: SourceKind::User,
            summary: "answer in chinese".to_string(),
            raw_text: Some("answer in chinese".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_natural_query".to_string()),
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "answer in chinese", 80).unwrap();

    assert!(result.results.len() >= 2);
    assert_eq!(result.results[0].summary, "answer in chinese");
    assert!(result.results.iter().any(|item| item.summary.contains("respond in chinese by default")));
}

#[test]
fn search_memories_does_not_treat_summary_raw_text_boundary_as_exact_match() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_boundary_match".to_string(),
            source_kind: SourceKind::User,
            summary: "alpha".to_string(),
            raw_text: Some("beta".to_string()),
            promotable: false,
            origin_mode: Mode::Normal,
            claim_fingerprint: None,
        },
    )
    .unwrap();

    sync_scope(&config, &scope).unwrap();
    let result = search_memories(&config, &scope, Intent::Continue, "alphabeta", 80).unwrap();
    assert!(result.results.is_empty());
}

fn write_qmd_records(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    source_type: &str,
    records: &[QmdRecord],
) {
    let collection_dir = config
        .root
        .join("qmd")
        .join("collections")
        .join("repos")
        .join(&scope.scope_id);
    fs::create_dir_all(&collection_dir).unwrap();
    let path = collection_dir.join(format!("{source_type}.jsonl"));
    let mut lines = Vec::new();
    for record in records {
        lines.push(serde_json::to_string(record).unwrap());
    }
    fs::write(path, format!("{}\n", lines.join("\n"))).unwrap();
}

fn embedding_with_target_cosine(query: &[f32], cosine: f32) -> Vec<f32> {
    let query_norm = normalize(query);
    let mut basis = vec![0.0f32; query.len()];
    basis[0] = 1.0;
    let projection = dot(&basis, &query_norm);
    let orthogonal = basis
        .iter()
        .zip(query_norm.iter())
        .map(|(base, query_value)| base - projection * query_value)
        .collect::<Vec<_>>();
    let orthogonal_norm = normalize(&orthogonal);
    let other_weight = (1.0 - cosine * cosine).sqrt();
    query_norm
        .iter()
        .zip(orthogonal_norm.iter())
        .map(|(q, o)| cosine * q + other_weight * o)
        .collect()
}

fn normalize(values: &[f32]) -> Vec<f32> {
    let norm = values.iter().map(|value| value * value).sum::<f32>().sqrt();
    values.iter().map(|value| value / norm).collect()
}

fn dot(left: &[f32], right: &[f32]) -> f32 {
    left.iter().zip(right.iter()).map(|(l, r)| l * r).sum()
}
