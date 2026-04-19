use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, ScopeRef, ScopeType};
use memfold::init::initialize_root;
use memfold::repair::run_repair;
use memfold::retrieval::search_memories;
use rusqlite::Connection;
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn repair_canonicalizes_project_scope_and_prunes_dirty_memory() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let alias_session_dir = root
        .join("memory")
        .join("repos")
        .join("MemFold")
        .join("sessions")
        .join("sess_upgrade");
    fs::create_dir_all(&alias_session_dir).unwrap();
    fs::write(
        alias_session_dir.join("session_log.jsonl"),
        concat!(
            "{\"evidence_id\":\"ev_noise\",\"scope\":{\"type\":\"project\",\"id\":\"MemFold\"},\"session_id\":\"sess_upgrade\",\"source_kind\":\"decision\",\"summary\":\"session exited via launcher trap\",\"raw_text\":null,\"promotable\":false,\"origin_mode\":\"normal\",\"claim_fingerprint\":null,\"created_at\":\"2026-04-12T18:45:56Z\"}\n",
            "{\"evidence_id\":\"ev_keep\",\"scope\":{\"type\":\"project\",\"id\":\"MemFold\"},\"session_id\":\"sess_upgrade\",\"source_kind\":\"user\",\"summary\":\"用户要求默认中文\",\"raw_text\":\"以后默认用中文回答\",\"promotable\":true,\"origin_mode\":\"normal\",\"claim_fingerprint\":\"cfp_upgrade_keep\",\"created_at\":\"2026-04-12T18:46:00Z\"}\n"
        ),
    )
    .unwrap();

    let alias_history_dir = root
        .join("memory")
        .join("repos")
        .join("MemFold")
        .join("history")
        .join("daily");
    fs::create_dir_all(&alias_history_dir).unwrap();
    fs::write(
        alias_history_dir.join("2026-04-12.md"),
        concat!(
            "<!-- session: sess_upgrade_noise | summary_id: hs_noise -->\n",
            "## 2026-04-12T18:45:56Z [session_end] session exited via launcher trap\n",
            "- [decision] session exited via launcher trap\n\n",
            "<!-- session: sess_upgrade_keep | summary_id: hs_keep -->\n",
            "## 2026-04-12T18:46:00Z [manual] 用户要求默认中文\n",
            "- [user] 用户要求默认中文\n"
        ),
    )
    .unwrap();

    run_repair(&config, None).unwrap();

    let canonical_scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let keep = search_memories(&config, &canonical_scope, Intent::Continue, "默认中文", 80).unwrap();
    assert!(!keep.results.is_empty());

    let noise =
        search_memories(&config, &canonical_scope, Intent::Continue, "launcher trap", 80).unwrap();
    assert!(noise.results.is_empty());

    assert!(root.join("memory").join("repos").join("memfold").exists());
    assert!(!root.join("memory").join("repos").join("MemFold").exists());
}

#[test]
fn repair_rebuilds_session_end_timestamps_from_session_logs() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let session_dir = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("sessions")
        .join("sess_done");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("session_log.jsonl"),
        concat!(
            "{\"evidence_id\":\"ev_1\",\"scope\":{\"type\":\"project\",\"id\":\"memfold\"},\"session_id\":\"sess_done\",\"source_kind\":\"user\",\"summary\":\"first\",\"raw_text\":\"first raw\",\"promotable\":false,\"origin_mode\":\"normal\",\"claim_fingerprint\":\"cfp_1\",\"created_at\":\"2026-04-12T18:00:00Z\"}\n",
            "{\"evidence_id\":\"ev_2\",\"scope\":{\"type\":\"project\",\"id\":\"memfold\"},\"session_id\":\"sess_done\",\"source_kind\":\"decision\",\"summary\":\"second\",\"raw_text\":null,\"promotable\":false,\"origin_mode\":\"normal\",\"claim_fingerprint\":null,\"created_at\":\"2026-04-12T18:05:00Z\"}\n"
        ),
    )
    .unwrap();

    run_repair(&config, None).unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    let session: (String, String) = conn
        .query_row(
            "SELECT started_at, ended_at FROM sessions WHERE id = 'sess_done'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();

    assert_eq!(session.0, "2026-04-12T18:00:00Z");
    assert_eq!(session.1, "2026-04-12T18:05:00Z");
}

#[test]
fn repair_merges_alias_session_logs_without_duplicate_evidence_ids() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let canonical_session_dir = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("sessions")
        .join("sess_dup");
    let alias_session_dir = root
        .join("memory")
        .join("repos")
        .join("MemFold")
        .join("sessions")
        .join("sess_dup");
    fs::create_dir_all(&canonical_session_dir).unwrap();
    fs::create_dir_all(&alias_session_dir).unwrap();

    let canonical_line = "{\"evidence_id\":\"ev_dup\",\"scope\":{\"type\":\"project\",\"id\":\"memfold\"},\"session_id\":\"sess_dup\",\"source_kind\":\"user\",\"summary\":\"用户要求默认中文\",\"raw_text\":\"以后默认用中文回答\",\"promotable\":true,\"origin_mode\":\"normal\",\"claim_fingerprint\":\"cfp_dup\",\"created_at\":\"2026-04-12T18:00:00Z\"}\n";
    let alias_line = "{\"evidence_id\":\"ev_dup\",\"scope\":{\"type\":\"project\",\"id\":\"MemFold\"},\"session_id\":\"sess_dup\",\"source_kind\":\"user\",\"summary\":\"用户要求默认中文\",\"raw_text\":\"以后默认用中文回答\",\"promotable\":true,\"origin_mode\":\"normal\",\"claim_fingerprint\":\"cfp_dup\",\"created_at\":\"2026-04-12T18:00:00Z\"}\n";
    fs::write(canonical_session_dir.join("session_log.jsonl"), canonical_line).unwrap();
    fs::write(alias_session_dir.join("session_log.jsonl"), alias_line).unwrap();

    run_repair(&config, None).unwrap();

    let conn = Connection::open(config.state_db_path()).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM session_log_entries WHERE id = 'ev_dup'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}
