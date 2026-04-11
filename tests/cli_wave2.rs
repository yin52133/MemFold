use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use memfold::boot::compile_scope_bundle;
use memfold::config::MemfoldConfig;
use memfold::domain::{ScopeRef, ScopeType};
use memfold::init::initialize_root;
use serde_json::Value;
use tempfile::TempDir;

fn bin_path() -> String {
    std::env::var("CARGO_BIN_EXE_memfold").expect("binary path should be set by cargo test")
}

fn memfold_root(tmp: &TempDir) -> PathBuf {
    tmp.path().join(".memfold")
}

fn write_stable_file(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
}

#[test]
fn cli_load_returns_structured_json_for_startup_bundle() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let user_scope = ScopeRef::new(ScopeType::User, "default").unwrap();
    let project_scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_stable_file(
        &root.join("memory").join("user").join("stable").join("preferences.md"),
        "## item_key: user.preference.language\n\
title: 默认中文\n\
status: stable\n\
autoload: boot_user\n\
claim_fingerprint: cfp_lang\n\
content_hash: sha256:userlang\n\
revision: 1\n\n\
默认用中文回答所有问题。\n",
    );
    write_stable_file(
        &root
            .join("memory")
            .join("projects")
            .join("memfold")
            .join("stable")
            .join("project-card.md"),
        "## item_key: project.rule.no-stale-path\n\
title: 禁止旧错误路径\n\
status: stable\n\
autoload: boot_project\n\
claim_fingerprint: cfp_project\n\
content_hash: sha256:projectrule\n\
revision: 1\n\n\
不要自动带入旧错误路径。\n",
    );

    compile_scope_bundle(&config, &user_scope, 400).unwrap();
    compile_scope_bundle(&config, &project_scope, 400).unwrap();

    let output = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "load",
            "--mode",
            "normal",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--intent",
            "startup",
            "--budget",
            "400",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["mode"], "normal");
    assert_eq!(json["scope"]["type"], "project");
    assert_eq!(json["scope"]["id"], "memfold");
    assert_eq!(json["degraded"], false);
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
}

#[test]
fn cli_write_evidence_returns_structured_json_and_persists_entry() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();

    let output = Command::new(bin_path())
        .env("MEMFOLD_ROOT", &root)
        .args([
            "write-evidence",
            "--scope-type",
            "project",
            "--scope-id",
            "memfold",
            "--session-id",
            "sess_cli",
            "--source-kind",
            "user",
            "--summary",
            "用户明确要求默认用中文回答",
            "--promotable",
            "1",
            "--origin-mode",
            "normal",
            "--claim-fingerprint",
            "cfp_cli_language",
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["stored"], true);
    assert!(json["evidence_id"].as_str().unwrap().starts_with("ev_"));

    let evidence_path = root
        .join("memory")
        .join("projects")
        .join("memfold")
        .join("sessions")
        .join("sess_cli")
        .join("evidence.jsonl");
    assert!(evidence_path.exists());
    assert!(fs::read_to_string(evidence_path)
        .unwrap()
        .contains("用户明确要求默认用中文回答"));
}
