use memfold::config::MemfoldConfig;
use memfold::domain::{ScopeRef, ScopeType};
use memfold::memory_fs::MemoryFs;
use tempfile::TempDir;

#[test]
fn memory_fs_appends_jsonl_and_writes_markdown_atomically() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let fs = MemoryFs::new(config);

    let line_no = fs
        .append_evidence_jsonl(&scope, "sess_1", r#"{"hello":"world"}"#)
        .unwrap();
    fs.write_project_bundle(&scope, "## project.rule\nKeep tests first.\n")
        .unwrap();

    assert_eq!(line_no, 1);
    assert!(
        tmp.path()
            .join("memory")
            .join("projects")
            .join("memfold")
            .join("sessions")
            .join("sess_1")
            .join("evidence.jsonl")
            .exists()
    );
    assert!(
        tmp.path()
            .join("memory")
            .join("projects")
            .join("memfold")
            .join("boot")
            .join("bundle.md")
            .exists()
    );
}
