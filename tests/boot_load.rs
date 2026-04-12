use std::fs;
use std::path::Path;

use memfold::boot::{compile_scope_bundle, load_startup_bundle};
use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType};
use memfold::error::Error;
use memfold::state::StateStore;
use rusqlite::{params, Connection};
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

fn init_config() -> (TempDir, MemfoldConfig) {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(memfold_root(&tmp));
    StateStore::initialize(&config).unwrap();
    (tmp, config)
}

fn write_stable_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

fn insert_memory_item(
    conn: &Connection,
    id: &str,
    scope_type: &str,
    scope_id: &str,
    item_key: &str,
    file_path: &str,
) {
    conn.execute(
        "INSERT INTO memory_items (
            id, scope_type, scope_id, item_key, file_path, title, status, autoload,
            claim_fingerprint, content_hash, revision, supersedes_id, trust_score,
            freshness_score, created_at, updated_at, deleted_at
        ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, 'stable', 'boot_user',
            ?7, ?8, 1, NULL, 1.0,
            1.0, ?9, ?9, NULL
        )",
        params![
            id,
            scope_type,
            scope_id,
            item_key,
            file_path,
            item_key,
            format!("cfp_{id}"),
            format!("sha256:{id}"),
            "2026-04-12T00:00:00Z",
        ],
    )
    .unwrap();
}

#[test]
fn compile_scope_bundle_filters_stable_boot_items_and_refreshes_projection() {
    let (_tmp, config) = init_config();
    let scope = ScopeRef::new(ScopeType::User, "default").unwrap();

    write_stable_file(
        &config.root.join("memory").join("user").join("stable").join("preferences.md"),
        "## item_key: user.preference.language\n\
title: Default language\n\
status: stable\n\
autoload: boot_user\n\
claim_fingerprint: cfp_lang\n\
content_hash: sha256:lang\n\
revision: 1\n\n\
keep tests first\n\n\
## item_key: user.preference.draft\n\
title: Draft\n\
status: candidate\n\
autoload: boot_user\n\
claim_fingerprint: cfp_draft\n\
content_hash: sha256:draft\n\
revision: 1\n\n\
skip this\n\n\
## item_key: user.preference.project-only\n\
title: Project only\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_project\n\
content_hash: sha256:project\n\
revision: 1\n\n\
skip this too\n",
    );

    let conn = Connection::open(config.state_db_path()).unwrap();
    insert_memory_item(
        &conn,
        "mem_lang",
        "user",
        "default",
        "user.preference.language",
        "memory/user/stable/preferences.md",
    );

    let load = compile_scope_bundle(&config, &scope, 10).unwrap();

    assert_eq!(load.items.len(), 1);
    assert_eq!(load.items[0].item_key, "user.preference.language");
    assert_eq!(load.items[0].source_item_id, "mem_lang");
    assert_eq!(load.items[0].token_estimate, 3);
    assert_eq!(load.total_tokens_estimate, 3);
    assert!(!load.degraded);

    let bundle_path = config
        .root
        .join("memory")
        .join("user")
        .join("boot")
        .join("bundle.md");
    let bundle_text = fs::read_to_string(bundle_path).unwrap();
    assert!(bundle_text.contains("compiled:"));
    assert!(bundle_text.contains("## user.preference.language"));
    assert!(bundle_text.contains("keep tests first"));

    let mut stmt = conn
        .prepare(
            "SELECT item_key, source_item_id, text, token_estimate
             FROM boot_entries
             WHERE scope_type = 'user' AND scope_id = 'default'
             ORDER BY item_key",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .unwrap();
    let rows = rows.collect::<rusqlite::Result<Vec<_>>>().unwrap();
    assert_eq!(
        rows,
        vec![(
            "user.preference.language".to_string(),
            "mem_lang".to_string(),
            "keep tests first".to_string(),
            3,
        )]
    );
}

#[test]
fn load_startup_bundle_respects_scope_modes_and_budget() {
    let (_tmp, config) = init_config();
    let user_scope = ScopeRef::new(ScopeType::User, "default").unwrap();
    let project_scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_stable_file(
        &config.root.join("memory").join("user").join("stable").join("preferences.md"),
        "## item_key: user.preference.language\n\
title: Default language\n\
status: stable\n\
autoload: boot_user\n\
claim_fingerprint: cfp_lang\n\
content_hash: sha256:lang\n\
revision: 1\n\n\
always answer english\n",
    );
    write_stable_file(
        &config
            .root
            .join("memory")
            .join("repos")
            .join("memfold")
            .join("stable")
            .join("project-card.md"),
        "## item_key: project.rule.no-stale-path\n\
title: No stale path\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_project\n\
content_hash: sha256:project\n\
revision: 1\n\n\
never carry stale paths\n",
    );

    compile_scope_bundle(&config, &user_scope, 100).unwrap();
    compile_scope_bundle(&config, &project_scope, 100).unwrap();

    let normal_project = load_startup_bundle(&config, &project_scope, Mode::Normal, None).unwrap();
    assert_eq!(normal_project.items.len(), 2);
    assert_eq!(normal_project.items[0].item_key, "user.preference.language");
    assert_eq!(normal_project.items[1].item_key, "project.rule.no-stale-path");
    assert_eq!(normal_project.total_tokens_estimate, 7);
    assert!(!normal_project.degraded);

    let normal_user = load_startup_bundle(&config, &user_scope, Mode::Normal, None).unwrap();
    assert_eq!(normal_user.items.len(), 1);
    assert_eq!(normal_user.items[0].item_key, "user.preference.language");

    let fresh_project = load_startup_bundle(&config, &project_scope, Mode::Fresh, None).unwrap();
    assert_eq!(fresh_project.items.len(), 1);
    assert_eq!(fresh_project.items[0].item_key, "user.preference.language");

    let sterile_project = load_startup_bundle(&config, &project_scope, Mode::Sterile, None).unwrap();
    assert!(sterile_project.items.is_empty());
    assert_eq!(sterile_project.total_tokens_estimate, 0);
    assert!(!sterile_project.degraded);

    let budgeted = load_startup_bundle(&config, &project_scope, Mode::Normal, Some(3)).unwrap();
    assert_eq!(budgeted.items.len(), 1);
    assert_eq!(budgeted.items[0].item_key, "user.preference.language");
    assert_eq!(budgeted.total_tokens_estimate, 3);
    assert!(budgeted.degraded);
}

#[test]
fn load_startup_bundle_rejects_unstable_scope_mutations() {
    let (_tmp, config) = init_config();
    let user_scope = ScopeRef::new(ScopeType::User, "default").unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    conn.execute(
        "INSERT INTO mutations (
            id, mutation_kind, target_ref, status, idempotency_key, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        params![
            "m_pending",
            "write",
            "user:default::boot/bundle.md",
            "pending",
            "idem_pending",
            "2026-04-12T00:00:00Z",
        ],
    )
    .unwrap();

    let err = load_startup_bundle(&config, &user_scope, Mode::Normal, Some(100)).unwrap_err();
    assert!(matches!(err, Error::UnstableMutation(_)));
}
