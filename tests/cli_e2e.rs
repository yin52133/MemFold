use std::path::PathBuf;
use std::process::Command;

use memfold::config::MemfoldConfig;
use memfold::init::initialize_root;
use serde_json::Value;
use tempfile::TempDir;

fn bin_path() -> String {
    std::env::var("CARGO_BIN_EXE_memfold").expect("binary path should be set by cargo test")
}

fn memfold_root(tmp: &TempDir) -> PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn cli_core_flow_runs_through_search_feedback_dream_and_repair() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let write = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "write-evidence",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--session-id",
            "sess_e2e",
            "--source-kind",
            "user",
            "--summary",
            "用户明确要求默认用中文回答",
            "--promotable",
            "1",
            "--origin-mode",
            "normal",
            "--claim-fingerprint",
            "cfp_e2e_language",
        ])
        .output()
        .unwrap();
    assert!(write.status.success(), "{write:?}");

    let dream = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "dream",
            "run",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--trigger",
            "manual",
        ])
        .output()
        .unwrap();
    assert!(dream.status.success(), "{dream:?}");
    let dream_json: Value = serde_json::from_slice(&dream.stdout).unwrap();
    assert_eq!(dream_json["promoted"], 1);

    let bundle_compile = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "bundle",
            "compile",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--budget",
            "400",
        ])
        .output()
        .unwrap();
    assert!(bundle_compile.status.success(), "{bundle_compile:?}");
    let bundle_json: Value = serde_json::from_slice(&bundle_compile.stdout).unwrap();
    assert_eq!(bundle_json["compiled"], true);
    assert_eq!(bundle_json["items"], 1);

    let qmd_sync = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "qmd",
            "sync",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
        ])
        .output()
        .unwrap();
    assert!(qmd_sync.status.success(), "{qmd_sync:?}");
    let qmd_json: Value = serde_json::from_slice(&qmd_sync.stdout).unwrap();
    assert_eq!(qmd_json["synced"], true);
    assert!(qmd_json["records"].as_u64().unwrap() >= 1);

    let search = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "search",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--intent",
            "continue",
            "--query",
            "中文回答",
            "--budget",
            "400",
        ])
        .output()
        .unwrap();
    assert!(search.status.success(), "{search:?}");
    let search_json: Value = serde_json::from_slice(&search.stdout).unwrap();
    assert!(!search_json["results"].as_array().unwrap().is_empty());

    let feedback = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "feedback",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--claim-fingerprint",
            "cfp_e2e_language",
            "--verdict",
            "rejected",
            "--reason",
            "这条记忆不对",
            "--session-id",
            "sess_feedback",
        ])
        .output()
        .unwrap();
    assert!(feedback.status.success(), "{feedback:?}");
    let feedback_json: Value = serde_json::from_slice(&feedback.stdout).unwrap();
    assert_eq!(feedback_json["updated"], true);
    assert_eq!(feedback_json["tombstone_written"], true);

    let second_dream = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "dream",
            "run",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--trigger",
            "manual",
        ])
        .output()
        .unwrap();
    assert!(second_dream.status.success(), "{second_dream:?}");
    let second_dream_json: Value = serde_json::from_slice(&second_dream.stdout).unwrap();
    assert_eq!(second_dream_json["promoted"], 0);
    assert!(second_dream_json["discarded"].as_u64().unwrap() >= 1);

    let repair = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args(["repair", "--scope-type", "project", "--scope-id", "memfold"])
        .output()
        .unwrap();
    assert!(repair.status.success(), "{repair:?}");
    let repair_json: Value = serde_json::from_slice(&repair.stdout).unwrap();
    assert_eq!(repair_json["repaired"], true);

    let session_log = root
        .join("memory")
        .join("repos")
        .join("memfold")
        .join("sessions")
        .join("sess_e2e")
        .join("session_log.jsonl");
    assert!(session_log.exists());
}
