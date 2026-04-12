use memfold::config::MemfoldConfig;
use memfold::domain::{Intent, Mode, ScopeRef, ScopeType};
use std::fs;
use tempfile::TempDir;

#[test]
fn scope_ref_formats_scope_key_and_dir_fragment() {
    let project = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let user = ScopeRef::new(ScopeType::User, "alice").unwrap();

    assert_eq!(project.scope_key(), "project:memfold");
    assert_eq!(project.scope_dir_fragment(), "repos/memfold");
    assert_eq!(user.scope_key(), "user:alice");
    assert_eq!(user.scope_dir_fragment(), "user");
    assert_eq!(Mode::Normal.as_str(), "normal");
    assert_eq!(Mode::Fresh.as_str(), "fresh");
    assert_eq!(Mode::Sterile.as_str(), "sterile");
    assert_eq!(Intent::Startup.as_str(), "startup");
    assert_eq!(Intent::Continue.as_str(), "continue");
    assert_eq!(Intent::KnowledgeLookup.as_str(), "knowledge_lookup");
    assert_eq!(Intent::Reset.as_str(), "reset");
}

#[test]
fn default_config_resolves_expected_paths_and_parent_dirs() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let nested = config
        .root
        .join("state")
        .join("nested")
        .join("memfold.db");

    assert_eq!(config.root, tmp.path().to_path_buf());
    assert_eq!(config.default_mode, Mode::Normal);
    assert_eq!(config.state_db_path(), tmp.path().join("state").join("memfold.db"));
    assert_eq!(config.project_root(&scope), tmp.path().join("memory").join("repos").join("memfold"));
    assert_eq!(config.config_file_path(), tmp.path().join("config").join("config.toml"));

    MemfoldConfig::ensure_parent_dir(&nested).unwrap();
    assert!(tmp.path().join("state").join("nested").exists());
}

#[test]
fn project_root_migrates_legacy_projects_directory() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let legacy = tmp
        .path()
        .join("memory")
        .join("projects")
        .join("memfold");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("sentinel.txt"), "legacy").unwrap();

    let resolved = config.project_root(&scope);

    assert_eq!(resolved, tmp.path().join("memory").join("repos").join("memfold"));
    assert!(resolved.join("sentinel.txt").exists());
    assert!(!legacy.exists());
}
