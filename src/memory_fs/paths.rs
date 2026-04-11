use std::path::PathBuf;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;

pub fn scope_root(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    config.project_root(scope)
}

pub fn boot_dir(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    scope_root(config, scope).join("boot")
}

pub fn project_bundle_path(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    boot_dir(config, scope).join("bundle.md")
}

pub fn stable_dir(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    scope_root(config, scope).join("stable")
}

pub fn archive_dir(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    scope_root(config, scope).join("archive")
}

pub fn archive_daily_path(config: &MemfoldConfig, scope: &ScopeRef, date: &str) -> PathBuf {
    archive_dir(config, scope).join(format!("memory-{date}.md"))
}

pub fn sessions_dir(config: &MemfoldConfig, scope: &ScopeRef) -> PathBuf {
    scope_root(config, scope).join("sessions")
}

pub fn session_dir(config: &MemfoldConfig, scope: &ScopeRef, session_id: &str) -> PathBuf {
    sessions_dir(config, scope).join(session_id)
}

pub fn session_evidence_path(
    config: &MemfoldConfig,
    scope: &ScopeRef,
    session_id: &str,
) -> PathBuf {
    session_dir(config, scope, session_id).join("evidence.jsonl")
}
