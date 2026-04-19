use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn deploy_script_runs_verify_and_conditional_repair_flow() {
    let script = fs::read_to_string("scripts/deploy_codex_global.sh").unwrap();

    assert!(script.contains("scripts/verify_codex_global.sh"));
    assert!(script.contains("repair"));
    assert!(script.contains("qmd sync"));
    assert!(script.contains(".codex/plugins/cache/home-local/memfold"));
}

#[test]
fn verify_script_checks_launcher_hook_and_search_health() {
    let script = fs::read_to_string("scripts/verify_codex_global.sh").unwrap();

    assert!(script.contains("cdx-memfold"));
    assert!(script.contains("hooks/turn_end.sh"));
    assert!(script.contains(" search "));
    assert!(script.contains("write-evidence"));
    assert!(script.contains("--raw-text"));
    assert!(script.contains("dream run"));
    assert!(script.contains("feedback"));
    assert!(script.contains("MEMFOLD_SESSION_SUMMARY"));
    assert!(script.contains("HOOK_SUMMARY"));
    assert!(script.contains("verify_memory_raw_trace"));
    assert!(script.contains("rm -rf"));
    assert!(script.contains("tr '[:upper:]' '[:lower:]'"));
    assert!(script.contains("verify_launcher_*"));
    assert!(script.contains("42"));
}

#[test]
fn session_start_hook_compensates_with_dream_and_qmd_sync() {
    let script = fs::read_to_string("hooks/codex-global/session_start.sh").unwrap();

    assert!(script.contains("dream maybe-run"));
    assert!(script.contains("qmd sync"));
}

#[test]
fn deploy_and_verify_scripts_are_executable_in_repo() {
    for path in ["scripts/deploy_codex_global.sh", "scripts/verify_codex_global.sh"] {
        let metadata = fs::metadata(path).unwrap();
        assert!(metadata.permissions().mode() & 0o111 != 0, "{path} is not executable");
    }
}
