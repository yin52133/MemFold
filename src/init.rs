use std::fs;

use crate::config::MemfoldConfig;
use crate::error::Result;
use crate::state::StateStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitSummary {
    pub created_paths: Vec<String>,
}

pub fn initialize_root(config: &MemfoldConfig) -> Result<InitSummary> {
    fs::create_dir_all(config.root.join("config"))?;
    fs::create_dir_all(config.root.join("state"))?;
    fs::create_dir_all(config.root.join("memory").join("user").join("stable"))?;
    fs::create_dir_all(config.root.join("memory").join("projects"))?;

    let config_file_path = config.config_file_path();
    fs::write(&config_file_path, "default_mode = \"normal\"\n")?;

    StateStore::initialize(config)?;

    Ok(InitSummary {
        created_paths: vec![
            config_file_path.display().to_string(),
            config.state_db_path().display().to_string(),
        ],
    })
}
