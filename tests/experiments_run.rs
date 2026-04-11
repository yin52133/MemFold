use std::path::PathBuf;
use std::process::Command;

use memfold::experiments::{default_fixture_path, run_fixture};
use serde_json::Value;

fn bin_path() -> String {
    std::env::var("CARGO_BIN_EXE_memfold").expect("binary path should be set by cargo test")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("experiments")
        .join(name)
}

#[test]
fn run_fixture_passes_for_the_default_regression_set() {
    let result = run_fixture(&fixture_path("passing.json")).unwrap();

    assert!(result.passed);
    assert_eq!(result.metrics.tombstone_resurrection_rate, 0.0);
    assert_eq!(result.metrics.analysis_promotion_rate, 0.0);
    assert_eq!(result.metrics.sterile_promotion_rate, 0.0);
    assert_eq!(result.metrics.raw_leak_rate, 0.0);
    assert_eq!(result.metrics.expectation_mismatches, 0);
    assert_eq!(result.fixture_path, fixture_path("passing.json").display().to_string());
}

#[test]
fn cli_experiment_run_reports_metric_failures_for_bad_fixture() {
    let output = Command::new(bin_path())
        .args([
            "experiment",
            "run",
            "--fixture",
            fixture_path("failing.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");

    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["passed"], false);
    assert_eq!(json["metrics"]["tombstone_resurrection_rate"], 0.0);
    assert_eq!(json["metrics"]["analysis_promotion_rate"], 0.0);
    assert_eq!(json["metrics"]["sterile_promotion_rate"], 0.0);
    assert_eq!(json["metrics"]["raw_leak_rate"], 0.0);
    assert!(json["metrics"]["expectation_mismatches"].as_u64().unwrap() > 0);
}

#[test]
fn default_fixture_path_points_to_passing_regression_set() {
    assert!(default_fixture_path().ends_with("tests/fixtures/experiments/passing.json"));
}
