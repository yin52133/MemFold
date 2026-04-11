use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn launcher_script_contains_session_start_and_exit_cleanup() {
    let script = fs::read_to_string("scripts/codex/cdx_memfold.sh").unwrap();

    assert!(script.contains("hooks/session_start.sh"));
    assert!(script.contains("trap cleanup EXIT INT TERM"));
    assert!(script.contains("hooks/session_end.sh"));
}

#[test]
fn launcher_script_is_executable_in_repo() {
    let metadata = fs::metadata("scripts/codex/cdx_memfold.sh").unwrap();
    assert!(metadata.permissions().mode() & 0o111 != 0);
}
