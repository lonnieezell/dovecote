use std::path::Path;

use rusqlite::{ffi, Connection, Row};

use crate::accounts::{Account, AccountSettings};
use crate::migrate::{migrate, Migration, MigrationError};
use crate::providers::ServerSettings;

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "accounts",
        sql: include_str!("../migrations/0002_accounts.sql"),
    },
];

fn account_from_row(r: &Row) -> rusqlite::Result<Account> {
    Ok(Account {
        id: r.get(0)?,
        email: r.get(1)?,
        provider: r.get(2)?,
        server: ServerSettings {
            imap_host: r.get(3)?,
            imap_port: r.get(4)?,
            smtp_host: r.get(5)?,
            smtp_port: r.get(6)?,
        },
        settings: AccountSettings {
            display_name: r.get(7)?,
            color: r.get(8)?,
            signature: r.get(9)?,
            send_from: r.get(10)?,
        },
    })
}

#[derive(Debug)]
pub enum AddAccountError {
    AlreadyAdded,
    Sql(rusqlite::Error),
}

impl From<rusqlite::Error> for AddAccountError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Sql(e)
    }
}

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

    /// Inserts the account with default settings.
    pub fn add_account(&self, email: &str, provider: &str, server: &ServerSettings) -> Result<Account, AddAccountError> {
        let existing: i64 = self.conn.query_row("SELECT count(*) FROM accounts", [], |r| r.get(0))?;
        let settings = AccountSettings::defaults(email, existing as usize);
        self.conn.execute(
            "INSERT INTO accounts (email, provider, imap_host, imap_port, smtp_host, smtp_port,
                                   display_name, color, signature, send_from)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            (
                email,
                provider,
                &server.imap_host,
                server.imap_port,
                &server.smtp_host,
                server.smtp_port,
                &settings.display_name,
                &settings.color,
                &settings.signature,
                &settings.send_from,
            ),
        )
        .map_err(|e| match e {
            rusqlite::Error::SqliteFailure(ref f, _) if f.extended_code == ffi::SQLITE_CONSTRAINT_UNIQUE => {
                AddAccountError::AlreadyAdded
            }
            e => AddAccountError::Sql(e),
        })?;
        Ok(Account {
            id: self.conn.last_insert_rowid(),
            email: email.into(),
            provider: provider.into(),
            server: server.clone(),
            settings,
        })
    }

    pub fn accounts(&self) -> rusqlite::Result<Vec<Account>> {
        self.conn
            .prepare(
                "SELECT id, email, provider, imap_host, imap_port, smtp_host, smtp_port,
                        display_name, color, signature, send_from
                 FROM accounts ORDER BY id",
            )?
            .query_map([], account_from_row)?
            .collect()
    }

    pub fn delete_account(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM accounts WHERE id = ?1", [id]).map(|_| ())
    }

    /// Returns false if no account has that id.
    pub fn save_account_settings(&self, id: i64, s: &AccountSettings) -> rusqlite::Result<bool> {
        let changed = self.conn.execute(
            "UPDATE accounts SET display_name = ?2, color = ?3, signature = ?4, send_from = ?5
             WHERE id = ?1",
            (id, s.display_name.trim(), &s.color, &s.signature, s.send_from.trim()),
        )?;
        Ok(changed == 1)
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
    fn every_migration_file_is_registered_with_a_matching_version_and_name() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        let registered: Vec<String> = MIGRATIONS
            .iter()
            .map(|m| format!("{:04}_{}.sql", m.version, m.name))
            .collect();
        assert_eq!(files, registered);
        assert!(MIGRATIONS.iter().zip(1..).all(|(m, v)| m.version == v));
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
