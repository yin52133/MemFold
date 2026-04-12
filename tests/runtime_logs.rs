use std::fs;

use memfold::config::MemfoldConfig;
use memfold::runtime_log::{StageLogger, runtime_log_path};
use tempfile::TempDir;

#[test]
fn stage_logger_writes_runtime_log_events() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    unsafe {
        std::env::set_var("MEMFOLD_SESSION_ID", "sess_runtime");
    }

    let logger = StageLogger::start(&config, "search", "search");
    logger.finish(Some("search finished"));

    let path = runtime_log_path(&config, "sess_runtime");
    let contents = fs::read_to_string(path).unwrap();
    assert!(contents.contains("\"event\":\"started\""));
    assert!(contents.contains("\"event\":\"finished\""));
}
