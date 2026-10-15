use std::path::Path;

use rusqlite::Connection;

use crate::migrate::{migrate, Migration, MigrationError};

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../migrations/0001_init.sql"),
}];

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self, MigrationError> {
        let mut conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Self { conn })
    }

    pub fn schema_version(&self) -> Result<u32, rusqlite::Error> {
        self.conn
            .query_row("SELECT COALESCE(MAX(version), 0) FROM schema_migrations", [], |r| r.get(0))
    }

    #[cfg(test)]
    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_in_wal_mode_and_applies_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("mail.db")).unwrap();
        let mode: String = db.conn().query_row("PRAGMA journal_mode", [], |r| r.get(0)).unwrap();
        assert_eq!(mode, "wal");
        assert!(db.schema_version().unwrap() >= 1);
    }

    #[test]
    fn reopening_does_not_reapply_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.db");
        let first = Database::open(&path).unwrap().schema_version().unwrap();
        let second = Database::open(&path).unwrap().schema_version().unwrap();
        assert_eq!(first, second);
    }
}
