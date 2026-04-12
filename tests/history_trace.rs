use std::fs;

use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType, SourceKind};
use memfold::evidence::{WriteEvidenceInput, write_evidence};
use memfold::history::{SummarizeHistoryInput, summarize_history};
use memfold::init::initialize_root;
use memfold::trace::{TraceQuery, trace_find};
use tempfile::TempDir;

fn memfold_root(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join(".memfold")
}

#[test]
fn summarize_history_writes_daily_summary_block() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root.clone());
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_hist".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求默认中文".to_string(),
            raw_text: Some("以后默认用中文回答".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_hist".to_string()),
        },
    )
    .unwrap();
    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_hist".to_string(),
            source_kind: SourceKind::Decision,
            summary: "确认切换到 session_log 路径".to_string(),
            raw_text: None,
            promotable: false,
            origin_mode: Mode::Normal,
            claim_fingerprint: None,
        },
    )
    .unwrap();

    let result = summarize_history(
        &config,
        &SummarizeHistoryInput {
            scope,
            session_id: "sess_hist".to_string(),
            trigger: "session_end".to_string(),
        },
    )
    .unwrap();
    assert!(result.updated);
    let history_text = fs::read_to_string(root.join(result.daily_path)).unwrap();
    assert!(history_text.contains("用户要求默认中文"));
    assert!(history_text.contains("确认切换到 session_log 路径"));
    assert!(!history_text.contains("jsonl_path:"));
}

#[test]
fn trace_find_returns_raw_user_text() {
    let tmp = TempDir::new().unwrap();
    let root = memfold_root(&tmp);
    let config = MemfoldConfig::default_for_root(root);
    initialize_root(&config).unwrap();
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    write_evidence(
        &config,
        &WriteEvidenceInput {
            scope: scope.clone(),
            session_id: "sess_trace".to_string(),
            source_kind: SourceKind::User,
            summary: "用户要求实事求是".to_string(),
            raw_text: Some("以后不要奉承我，要判断我说得对不对。".to_string()),
            promotable: true,
            origin_mode: Mode::Normal,
            claim_fingerprint: Some("cfp_trace".to_string()),
        },
    )
    .unwrap();

    let trace = trace_find(
        &config,
        &TraceQuery {
            scope: Some(scope),
            query: "不要奉承".to_string(),
        },
    )
    .unwrap();

    assert_eq!(
        trace.raw_text.as_deref(),
        Some("以后不要奉承我，要判断我说得对不对。")
    );
    assert_eq!(trace.summary, "用户要求实事求是");
}
