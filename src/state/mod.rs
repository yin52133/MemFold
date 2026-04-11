pub mod schema;

use rusqlite::Connection;

use crate::config::MemfoldConfig;

pub struct StateStore {
    conn: Connection,
}

impl StateStore {
    pub fn initialize(config: &MemfoldConfig) -> crate::error::Result<Self> {
        let db_path = config.state_db_path();
        MemfoldConfig::ensure_parent_dir(&db_path)?;

        let conn = Connection::open(db_path)?;
        schema::apply_schema(&conn)?;

        Ok(Self { conn })
    }

    pub fn list_tables(&self) -> crate::error::Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT name FROM sqlite_master \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
             ORDER BY name",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }
}
