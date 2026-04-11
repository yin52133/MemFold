# MemFold Wave 1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust foundation for MemFold V1 so later waves can add CLI, retrieval, feedback, and dreaming on top of stable domain types, configuration, SQLite state bootstrap, and filesystem IO primitives.

**Architecture:** Use a single Rust package for now with a library-first layout. Wave 1 produces reusable library modules for config, state, init, and memory filesystem helpers; later waves will wrap them with CLI commands. Keep SQLite as the only state truth source and filesystem content as the only content truth source.

**Tech Stack:** Rust 1.94, Cargo, rusqlite, serde, toml, serde_json, tempfile, sha2, thiserror

---

## File Structure

- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/error.rs`
- Create: `src/domain.rs`
- Create: `src/config.rs`
- Create: `src/init.rs`
- Create: `src/state/mod.rs`
- Create: `src/state/schema.rs`
- Create: `src/memory_fs/mod.rs`
- Create: `src/memory_fs/paths.rs`
- Create: `src/memory_fs/atomic.rs`
- Create: `src/memory_fs/markdown.rs`
- Create: `src/memory_fs/jsonl.rs`
- Create: `tests/config_defaults.rs`
- Create: `tests/state_init.rs`
- Create: `tests/memory_fs_io.rs`
- Create: `tests/init_root.rs`

### Task 1: Bootstrap Rust Package And Shared Types

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `src/error.rs`
- Create: `src/domain.rs`

- [ ] **Step 1: Write the failing shared-types test**

```rust
use memfold::domain::{Intent, Mode, ScopeRef, ScopeType};

#[test]
fn scope_ref_formats_project_scope_key() {
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    assert_eq!(scope.scope_key(), "project:memfold");
    assert_eq!(scope.scope_dir_fragment(), "projects/memfold");
    assert_eq!(Mode::Normal.as_str(), "normal");
    assert_eq!(Intent::Startup.as_str(), "startup");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test config_defaults scope_ref_formats_project_scope_key`
Expected: FAIL because the crate and exported modules do not exist yet

- [ ] **Step 3: Write minimal package and shared implementation**

```toml
[package]
name = "memfold"
version = "0.1.0"
edition = "2024"

[dependencies]
rusqlite = { version = "0.37", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"
toml = "0.9"

[dev-dependencies]
tempfile = "3"
```

```rust
// src/lib.rs
pub mod config;
pub mod domain;
pub mod error;
pub mod init;
pub mod memory_fs;
pub mod state;
```

```rust
// src/main.rs
fn main() {}
```

```rust
// src/domain.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeType {
    User,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Fresh,
    Sterile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Startup,
    Continue,
    KnowledgeLookup,
    Reset,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test config_defaults scope_ref_formats_project_scope_key`
Expected: PASS

### Task 2: Implement Config Defaults And Root Resolution

**Files:**
- Create: `src/config.rs`
- Modify: `src/domain.rs`
- Modify: `src/error.rs`
- Test: `tests/config_defaults.rs`

- [ ] **Step 1: Write the failing config test**

```rust
use memfold::config::MemfoldConfig;
use memfold::domain::{Mode, ScopeRef, ScopeType};
use tempfile::TempDir;

#[test]
fn default_config_resolves_expected_paths() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();

    assert_eq!(config.default_mode, Mode::Normal);
    assert_eq!(config.state_db_path(), tmp.path().join("state").join("memfold.db"));
    assert_eq!(
        config.project_root(&scope),
        tmp.path().join("memory").join("projects").join("memfold")
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test config_defaults default_config_resolves_expected_paths`
Expected: FAIL because `MemfoldConfig` and path helpers do not exist

- [ ] **Step 3: Write minimal config implementation**

```rust
// src/config.rs
use std::path::{Path, PathBuf};

use crate::domain::{Mode, ScopeRef, ScopeType};

#[derive(Debug, Clone)]
pub struct MemfoldConfig {
    pub root: PathBuf,
    pub default_mode: Mode,
}

impl MemfoldConfig {
    pub fn default_for_root(root: PathBuf) -> Self {
        Self { root, default_mode: Mode::Normal }
    }

    pub fn state_db_path(&self) -> PathBuf {
        self.root.join("state").join("memfold.db")
    }

    pub fn project_root(&self, scope: &ScopeRef) -> PathBuf {
        match scope.scope_type {
            ScopeType::Project => self.root.join("memory").join("projects").join(&scope.scope_id),
            ScopeType::User => self.root.join("memory").join("user"),
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
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test config_defaults default_config_resolves_expected_paths`
Expected: PASS

### Task 3: Implement SQLite Schema Bootstrap

**Files:**
- Create: `src/state/mod.rs`
- Create: `src/state/schema.rs`
- Modify: `src/error.rs`
- Test: `tests/state_init.rs`

- [ ] **Step 1: Write the failing state bootstrap test**

```rust
use memfold::config::MemfoldConfig;
use memfold::state::StateStore;
use tempfile::TempDir;

#[test]
fn state_store_bootstraps_required_tables() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());

    let store = StateStore::initialize(&config).unwrap();
    let table_names = store.list_tables().unwrap();

    assert!(table_names.contains(&"memory_items".to_string()));
    assert!(table_names.contains(&"evidence_items".to_string()));
    assert!(table_names.contains(&"mutations".to_string()));
    assert!(table_names.contains(&"lock_leases".to_string()));
    assert!(table_names.contains(&"sessions".to_string()));
    assert!(table_names.contains(&"tombstones".to_string()));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test state_init state_store_bootstraps_required_tables`
Expected: FAIL because `StateStore` and schema bootstrap do not exist

- [ ] **Step 3: Write minimal SQLite initialization**

```rust
// src/state/schema.rs
pub const REQUIRED_TABLES: &[&str] = &[
    "memory_items",
    "evidence_items",
    "trace_archives",
    "boot_entries",
    "mutations",
    "sessions",
    "lock_leases",
    "dream_jobs",
    "tombstones",
    "feedback_events",
    "retrieval_events",
];

pub fn apply_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS memory_items (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS evidence_items (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS trace_archives (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS boot_entries (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS mutations (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS lock_leases (lock_key TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS dream_jobs (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS tombstones (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS feedback_events (id TEXT PRIMARY KEY);
        CREATE TABLE IF NOT EXISTS retrieval_events (id TEXT PRIMARY KEY);
        ",
    )
}
```

```rust
// src/state/mod.rs
use rusqlite::Connection;

use crate::config::MemfoldConfig;

pub struct StateStore {
    conn: Connection,
}

impl StateStore {
    pub fn initialize(config: &MemfoldConfig) -> crate::error::Result<Self> {
        MemfoldConfig::ensure_parent_dir(&config.state_db_path())?;
        let conn = Connection::open(config.state_db_path())?;
        crate::state::schema::apply_schema(&conn)?;
        Ok(Self { conn })
    }

    pub fn list_tables(&self) -> crate::error::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test state_init state_store_bootstraps_required_tables`
Expected: PASS

### Task 4: Implement Filesystem Content IO Primitives

**Files:**
- Create: `src/memory_fs/mod.rs`
- Create: `src/memory_fs/paths.rs`
- Create: `src/memory_fs/atomic.rs`
- Create: `src/memory_fs/markdown.rs`
- Create: `src/memory_fs/jsonl.rs`
- Test: `tests/memory_fs_io.rs`

- [ ] **Step 1: Write the failing memory filesystem test**

```rust
use memfold::config::MemfoldConfig;
use memfold::domain::{ScopeRef, ScopeType};
use memfold::memory_fs::MemoryFs;
use tempfile::TempDir;

#[test]
fn memory_fs_appends_jsonl_and_writes_markdown_atomically() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());
    let scope = ScopeRef::new(ScopeType::Project, "memfold").unwrap();
    let fs = MemoryFs::new(config.clone());

    let line_no = fs.append_evidence_jsonl(&scope, "sess_1", "{\"hello\":\"world\"}").unwrap();
    fs.write_project_bundle(&scope, "## project.rule\nKeep tests first.\n").unwrap();

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test memory_fs_io memory_fs_appends_jsonl_and_writes_markdown_atomically`
Expected: FAIL because `MemoryFs` and IO helpers do not exist

- [ ] **Step 3: Write minimal IO implementation**

```rust
// src/memory_fs/mod.rs
pub struct MemoryFs {
    config: crate::config::MemfoldConfig,
}

impl MemoryFs {
    pub fn new(config: crate::config::MemfoldConfig) -> Self {
        Self { config }
    }
}
```

```rust
// src/memory_fs/jsonl.rs
pub fn append_jsonl_line(path: &std::path::Path, line: &str) -> std::io::Result<usize> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let line_no = existing.lines().count() + 1;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    use std::io::Write as _;
    writeln!(file, "{line}")?;
    Ok(line_no)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test memory_fs_io memory_fs_appends_jsonl_and_writes_markdown_atomically`
Expected: PASS

### Task 5: Integrate Root Initialization For Wave 1

**Files:**
- Create: `src/init.rs`
- Modify: `src/config.rs`
- Modify: `src/state/mod.rs`
- Modify: `src/memory_fs/mod.rs`
- Test: `tests/init_root.rs`

- [ ] **Step 1: Write the failing root init test**

```rust
use memfold::config::MemfoldConfig;
use memfold::init::initialize_root;
use tempfile::TempDir;

#[test]
fn initialize_root_creates_required_foundation_layout() {
    let tmp = TempDir::new().unwrap();
    let config = MemfoldConfig::default_for_root(tmp.path().to_path_buf());

    let summary = initialize_root(&config).unwrap();

    assert!(summary.created_paths.iter().any(|p| p.ends_with("config/config.toml")));
    assert!(summary.created_paths.iter().any(|p| p.ends_with("state/memfold.db")));
    assert!(tmp.path().join("memory").join("user").join("stable").exists());
    assert!(tmp.path().join("memory").join("projects").exists());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test init_root initialize_root_creates_required_foundation_layout`
Expected: FAIL because `initialize_root` and init summary do not exist

- [ ] **Step 3: Write minimal root init implementation**

```rust
// src/init.rs
pub struct InitSummary {
    pub created_paths: Vec<String>,
}

pub fn initialize_root(config: &crate::config::MemfoldConfig) -> crate::error::Result<InitSummary> {
    let mut created_paths = Vec::new();
    std::fs::create_dir_all(config.root.join("config"))?;
    std::fs::create_dir_all(config.root.join("state"))?;
    std::fs::create_dir_all(config.root.join("memory").join("user").join("stable"))?;
    std::fs::create_dir_all(config.root.join("memory").join("projects"))?;
    std::fs::write(config.config_file_path(), "default_mode = \"normal\"\n")?;
    created_paths.push(config.config_file_path().display().to_string());
    crate::state::StateStore::initialize(config)?;
    created_paths.push(config.state_db_path().display().to_string());
    Ok(InitSummary { created_paths })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test init_root initialize_root_creates_required_foundation_layout`
Expected: PASS

### Task 6: Run Wave 1 Verification Suite

**Files:**
- Modify: `docs/progress/memfold-v1/00-master-checklist.zh-CN.md`
- Modify: `docs/progress/memfold-v1/01-wave-1-foundation.zh-CN.md`

- [ ] **Step 1: Run the full Wave 1 test suite**

Run: `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root`
Expected: all selected tests PASS

- [ ] **Step 2: Run the package-wide test suite**

Run: `cargo test`
Expected: exit code 0

- [ ] **Step 3: Update progress tracking with evidence**

```markdown
- 状态：`done`
- 验证记录：
  - `cargo test --test config_defaults --test state_init --test memory_fs_io --test init_root`
  - `cargo test`
```

- [ ] **Step 4: Mark Wave 1 gate status**

```markdown
- [x] Gate A: `config + state + memory_fs + init bootstrap` 集成通过
```
