use std::path::{Path, PathBuf};

use crate::domain::{Mode, ScopeRef, ScopeType};

#[derive(Debug, Clone)]
pub struct MemfoldConfig {
    pub root: PathBuf,
    pub default_mode: Mode,
}

impl MemfoldConfig {
    pub fn default_for_root(root: PathBuf) -> Self {
        Self {
            root,
            default_mode: Mode::Normal,
        }
    }

    pub fn state_db_path(&self) -> PathBuf {
        self.root.join("state").join("memfold.db")
    }

    pub fn project_root(&self, scope: &ScopeRef) -> PathBuf {
        match scope.scope_type {
            ScopeType::User => self.root.join("memory").join("user"),
            ScopeType::Project => self.root.join("memory").join("projects").join(&scope.scope_id),
        }
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.root.join("config").join("config.toml")
    }

    pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }

    pub fn resolve_root(root_override: Option<PathBuf>) -> PathBuf {
        if let Some(root) = root_override {
            return root;
        }

        if let Ok(root) = std::env::var("MEMFOLD_ROOT") {
            return PathBuf::from(root);
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".memfold")
    }
}
