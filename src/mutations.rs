use rusqlite::{params, Connection, OptionalExtension};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::config::MemfoldConfig;
use crate::domain::ScopeRef;
use crate::error::Result;
use crate::state::schema;

const STATUS_PENDING: &str = "pending";
const STATUS_APPLIED_TO_CONTENT: &str = "applied_to_content";
const STATUS_FULLY_APPLIED: &str = "fully_applied";

#[derive(Debug, Clone)]
pub struct MutationStore {
    config: MemfoldConfig,
}

impl MutationStore {
    pub fn new(config: MemfoldConfig) -> Self {
        Self { config }
    }

    pub fn begin(
        &self,
        scope: &ScopeRef,
        mutation_kind: &str,
        target_ref: &str,
        idempotency_key: &str,
    ) -> Result<String> {
        let conn = self.open_connection()?;
        let encoded_target_ref = encode_target_ref(scope, target_ref);

        if let Some(existing_id) = conn
            .query_row(
                "SELECT id FROM mutations WHERE idempotency_key = ?1 LIMIT 1",
                params![idempotency_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(existing_id);
        }

        let mutation_id = Uuid::new_v4().to_string();
        let now = current_timestamp();

        conn.execute(
            "INSERT INTO mutations (
                id,
                mutation_kind,
                target_ref,
                status,
                idempotency_key,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                mutation_id,
                mutation_kind,
                encoded_target_ref,
                STATUS_PENDING,
                idempotency_key,
                now,
                now
            ],
        )?;

        Ok(mutation_id)
    }

    pub fn mark_applied_to_content(&self, mutation_id: &str) -> Result<()> {
        self.update_status(mutation_id, STATUS_APPLIED_TO_CONTENT)
    }

    pub fn mark_fully_applied(&self, mutation_id: &str) -> Result<()> {
        self.update_status(mutation_id, STATUS_FULLY_APPLIED)
    }

    pub fn has_unstable_scope_mutations(&self, scope: &ScopeRef) -> Result<bool> {
        let conn = self.open_connection()?;
        let scope_prefix = scope_prefix(scope);

        let unstable = conn.query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM mutations
                WHERE substr(target_ref, 1, length(?1)) = ?1
                  AND status <> ?2
            )",
            params![scope_prefix, STATUS_FULLY_APPLIED],
            |row| row.get::<_, i64>(0),
        )?;

        Ok(unstable != 0)
    }

    fn update_status(&self, mutation_id: &str, status: &str) -> Result<()> {
        let conn = self.open_connection()?;
        let updated = conn.execute(
            "UPDATE mutations
             SET status = ?1,
                 updated_at = ?2
             WHERE id = ?3",
            params![status, current_timestamp(), mutation_id],
        )?;

        if updated == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows.into());
        }

        Ok(())
    }

    fn open_connection(&self) -> Result<Connection> {
        let db_path = self.config.state_db_path();
        MemfoldConfig::ensure_parent_dir(&db_path)?;
        let conn = Connection::open(db_path)?;
        schema::apply_schema(&conn)?;
        Ok(conn)
    }
}

fn encode_target_ref(scope: &ScopeRef, target_ref: &str) -> String {
    format!("{}::{}", scope.scope_key(), target_ref)
}

fn scope_prefix(scope: &ScopeRef) -> String {
    format!("{}::", scope.scope_key())
}

fn current_timestamp() -> String {
    OffsetDateTime::now_utc().to_string()
}
