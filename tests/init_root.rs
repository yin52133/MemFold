use memfold::config::MemfoldConfig;
use memfold::init::initialize_root;
use tempfile::TempDir;

#[test]
fn initialize_root_creates_required_foundation_layout() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());

    let summary = initialize_root(&config).unwrap();

    assert!(
        summary
            .created_paths
            .iter()
            .any(|path| path.ends_with("config/config.toml"))
    );
    assert!(
        summary
            .created_paths
            .iter()
            .any(|path| path.ends_with("state/memfold.db"))
    );
    assert!(tmp.path().join("memory").join("user").join("stable").exists());
    assert!(tmp.path().join("memory").join("projects").exists());
}
