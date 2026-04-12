pub mod atomic;
pub mod jsonl;
pub mod markdown;
pub mod paths;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct MemoryFs {
    config: MemfoldConfig,
}

impl MemoryFs {
    pub fn new(config: MemfoldConfig) -> Self {
        Self { config }
    }

    pub fn append_evidence_jsonl(
        &self,
        scope: &ScopeRef,
        session_id: &str,
        line: &str,
    ) -> Result<usize> {
        let path = paths::session_evidence_path(&self.config, scope, session_id);
        jsonl::append_jsonl_line(&path, line)
    }

    pub fn append_session_log_jsonl(
        &self,
        scope: &ScopeRef,
        session_id: &str,
        line: &str,
    ) -> Result<usize> {
        self.append_evidence_jsonl(scope, session_id, line)
    }

    pub fn write_project_bundle(&self, scope: &ScopeRef, contents: &str) -> Result<()> {
        let path = paths::project_bundle_path(&self.config, scope);
        markdown::write_markdown_file(&path, contents)
    }
}
