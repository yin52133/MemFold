use rusqlite::Connection;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memory_items (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    item_key TEXT NOT NULL,
    file_path TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    autoload TEXT NOT NULL,
    claim_fingerprint TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 0,
    supersedes_id TEXT,
    trust_score REAL NOT NULL DEFAULT 0.0,
    freshness_score REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS evidence_items (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    jsonl_path TEXT NOT NULL,
    line_no INTEGER NOT NULL,
    promotable INTEGER NOT NULL DEFAULT 0,
    origin_mode TEXT NOT NULL,
    claim_fingerprint TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS trace_archives (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    archive_date TEXT NOT NULL,
    archive_kind TEXT NOT NULL,
    file_path TEXT NOT NULL,
    line_no INTEGER,
    content_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS boot_entries (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    item_key TEXT NOT NULL,
    source_item_id TEXT NOT NULL,
    text TEXT NOT NULL,
    token_estimate INTEGER NOT NULL DEFAULT 0,
    compiled_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS mutations (
    id TEXT PRIMARY KEY,
    mutation_kind TEXT NOT NULL,
    target_ref TEXT NOT NULL,
    status TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    intent TEXT NOT NULL,
    host TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    evidence_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS lock_leases (
    lock_key TEXT PRIMARY KEY,
    owner TEXT NOT NULL,
    lease_until TEXT NOT NULL,
    heartbeat_at TEXT NOT NULL,
    idempotency_key TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS dream_jobs (
    id TEXT PRIMARY KEY,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    trigger TEXT NOT NULL,
    status TEXT NOT NULL,
    promoted INTEGER,
    held INTEGER,
    quarantined INTEGER,
    discarded INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tombstones (
    id TEXT PRIMARY KEY,
    claim_fingerprint TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    source_item_id TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS feedback_events (
    id TEXT PRIMARY KEY,
    target_id TEXT NOT NULL,
    target_type TEXT NOT NULL,
    verdict TEXT NOT NULL,
    reason TEXT,
    session_id TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS retrieval_events (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    query TEXT,
    intent TEXT NOT NULL,
    source_type TEXT NOT NULL,
    doc_id TEXT NOT NULL,
    latency_ms INTEGER NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memory_scope_status
    ON memory_items (scope_type, scope_id, status);

CREATE INDEX IF NOT EXISTS idx_memory_claim_fingerprint
    ON memory_items (claim_fingerprint);

CREATE INDEX IF NOT EXISTS idx_memory_autoload
    ON memory_items (autoload, status);

CREATE INDEX IF NOT EXISTS idx_evidence_session_created
    ON evidence_items (session_id, created_at);

CREATE INDEX IF NOT EXISTS idx_evidence_claim_fingerprint
    ON evidence_items (claim_fingerprint);

CREATE INDEX IF NOT EXISTS idx_evidence_promotable
    ON evidence_items (promotable, origin_mode);

CREATE INDEX IF NOT EXISTS idx_trace_scope_date
    ON trace_archives (scope_type, scope_id, archive_date);

CREATE INDEX IF NOT EXISTS idx_mutations_status
    ON mutations (status, created_at);

CREATE INDEX IF NOT EXISTS idx_sessions_scope
    ON sessions (scope_type, scope_id, started_at);

CREATE INDEX IF NOT EXISTS idx_dream_jobs_status
    ON dream_jobs (status, created_at);

CREATE INDEX IF NOT EXISTS idx_tombstones_claim_fingerprint
    ON tombstones (claim_fingerprint, scope_type, scope_id);
"#;

pub fn apply_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA_SQL)
}
