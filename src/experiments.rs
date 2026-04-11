use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedAction {
    Promote,
    Hold,
    Discard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentCase {
    pub id: String,
    pub summary: String,
    pub promotable: bool,
    pub tombstoned: bool,
    pub origin_mode: String,
    pub analysis_draft: bool,
    pub expected_action: ExpectedAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentFixture {
    pub cases: Vec<ExperimentCase>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExperimentCaseResult {
    pub id: String,
    pub predicted_action: ExpectedAction,
    pub expected_action: ExpectedAction,
    pub matches_expectation: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentMetrics {
    pub total_cases: usize,
    pub promoted: usize,
    pub held: usize,
    pub discarded: usize,
    pub tombstone_resurrection_rate: f64,
    pub analysis_promotion_rate: f64,
    pub sterile_promotion_rate: f64,
    pub raw_leak_rate: f64,
    pub expectation_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentRunResult {
    pub fixture_path: String,
    pub passed: bool,
    pub metrics: ExperimentMetrics,
    pub results: Vec<ExperimentCaseResult>,
}

pub fn run_fixture(fixture_path: &Path) -> Result<ExperimentRunResult> {
    let fixture = load_fixture(fixture_path)?;
    let results = fixture
        .cases
        .iter()
        .map(|case| {
            let predicted_action = evaluate_case(case);
            ExperimentCaseResult {
                id: case.id.clone(),
                matches_expectation: predicted_action == case.expected_action,
                predicted_action,
                expected_action: case.expected_action.clone(),
            }
        })
        .collect::<Vec<_>>();

    let metrics = build_metrics(&fixture.cases, &results);
    let passed = metrics.tombstone_resurrection_rate == 0.0
        && metrics.analysis_promotion_rate == 0.0
        && metrics.sterile_promotion_rate == 0.0
        && metrics.raw_leak_rate == 0.0
        && metrics.expectation_mismatches == 0;

    Ok(ExperimentRunResult {
        fixture_path: fixture_path.display().to_string(),
        passed,
        metrics,
        results,
    })
}

pub fn default_fixture_path() -> PathBuf {
    PathBuf::from("tests")
        .join("fixtures")
        .join("experiments")
        .join("passing.json")
}

fn load_fixture(path: &Path) -> Result<ExperimentFixture> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn evaluate_case(case: &ExperimentCase) -> ExpectedAction {
    if contains_sensitive_material(&case.summary)
        || case.tombstoned
        || case.analysis_draft
        || case.origin_mode == "sterile"
    {
        return ExpectedAction::Discard;
    }

    if case.promotable {
        ExpectedAction::Promote
    } else {
        ExpectedAction::Hold
    }
}

fn build_metrics(cases: &[ExperimentCase], results: &[ExperimentCaseResult]) -> ExperimentMetrics {
    let promoted = results
        .iter()
        .filter(|result| result.predicted_action == ExpectedAction::Promote)
        .count();
    let held = results
        .iter()
        .filter(|result| result.predicted_action == ExpectedAction::Hold)
        .count();
    let discarded = results
        .iter()
        .filter(|result| result.predicted_action == ExpectedAction::Discard)
        .count();

    let tombstoned_total = cases.iter().filter(|case| case.tombstoned).count();
    let tombstoned_promoted = cases
        .iter()
        .zip(results.iter())
        .filter(|(case, result)| case.tombstoned && result.predicted_action == ExpectedAction::Promote)
        .count();

    let analysis_total = cases.iter().filter(|case| case.analysis_draft).count();
    let analysis_promoted = cases
        .iter()
        .zip(results.iter())
        .filter(|(case, result)| case.analysis_draft && result.predicted_action == ExpectedAction::Promote)
        .count();

    let sterile_total = cases
        .iter()
        .filter(|case| case.origin_mode == "sterile")
        .count();
    let sterile_promoted = cases
        .iter()
        .zip(results.iter())
        .filter(|(case, result)| case.origin_mode == "sterile" && result.predicted_action == ExpectedAction::Promote)
        .count();

    let promoted_sensitive = cases
        .iter()
        .zip(results.iter())
        .filter(|(case, result)| {
            result.predicted_action == ExpectedAction::Promote
                && contains_sensitive_material(&case.summary)
        })
        .count();

    let expectation_mismatches = results
        .iter()
        .filter(|result| !result.matches_expectation)
        .count();

    ExperimentMetrics {
        total_cases: cases.len(),
        promoted,
        held,
        discarded,
        tombstone_resurrection_rate: ratio(tombstoned_promoted, tombstoned_total),
        analysis_promotion_rate: ratio(analysis_promoted, analysis_total),
        sterile_promotion_rate: ratio(sterile_promoted, sterile_total),
        raw_leak_rate: ratio(promoted_sensitive, promoted.max(1)),
        expectation_mismatches,
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn contains_sensitive_material(summary: &str) -> bool {
    let lowered = summary.to_lowercase();
    [
        "sk-",
        "api_key",
        "token=",
        "cookie=",
        "private key",
        "begin private key",
        "secret_access_key",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}
