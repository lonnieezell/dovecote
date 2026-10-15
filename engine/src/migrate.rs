use rusqlite::{Connection, OptionalExtension};

pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("migration {0} was edited after it was applied")]
    Edited(u32),
    #[error(transparent)]
    Sql(#[from] rusqlite::Error),
}

/// Applies each migration not yet recorded, in the order given. Each one runs in its own
/// transaction. A recorded migration whose SQL no longer matches is an error.
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version    INTEGER PRIMARY KEY,
           name       TEXT NOT NULL,
           sql        TEXT NOT NULL,
           applied_at INTEGER NOT NULL
         )",
    )?;
    for m in migrations {
        let applied: Option<String> = conn
            .query_row("SELECT sql FROM schema_migrations WHERE version = ?1", [m.version], |r| r.get(0))
            .optional()?;
        match applied {
            Some(sql) if sql == m.sql => {}
            Some(_) => return Err(MigrationError::Edited(m.version)),
            None => {
                let tx = conn.transaction()?;
                tx.execute_batch(m.sql)?;
                tx.execute(
                    "INSERT INTO schema_migrations (version, name, sql, applied_at)
                     VALUES (?1, ?2, ?3, unixepoch())",
                    (m.version, m.name, m.sql),
                )?;
                tx.commit()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CREATE_A: Migration = Migration {
        version: 1,
        name: "create_a",
        sql: "CREATE TABLE a (id INTEGER PRIMARY KEY);",
    };
    const CREATE_B: Migration = Migration {
        version: 2,
        name: "create_b",
        sql: "CREATE TABLE b (id INTEGER PRIMARY KEY);",
    };

    fn table_exists(conn: &Connection, name: &str) -> bool {
        conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
            == 1
    }

    fn applied_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn applies_each_migration_in_order() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &[CREATE_A, CREATE_B]).unwrap();
        assert!(table_exists(&conn, "a"));
        assert!(table_exists(&conn, "b"));
        assert_eq!(applied_count(&conn), 2);
    }

    #[test]
    fn applies_a_migration_only_once() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &[CREATE_A]).unwrap();
        // A second run would fail on CREATE TABLE if it re-applied.
        migrate(&mut conn, &[CREATE_A]).unwrap();
        assert_eq!(applied_count(&conn), 1);
    }

    #[test]
    fn applies_only_the_new_migrations_on_a_later_run() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &[CREATE_A]).unwrap();
        migrate(&mut conn, &[CREATE_A, CREATE_B]).unwrap();
        assert!(table_exists(&conn, "b"));
        assert_eq!(applied_count(&conn), 2);
    }

    #[test]
    fn refuses_an_applied_migration_whose_sql_changed() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &[CREATE_A]).unwrap();
        let edited = Migration {
            sql: "CREATE TABLE a (id INTEGER PRIMARY KEY, extra TEXT);",
            ..CREATE_A
        };
        let err = migrate(&mut conn, &[edited]).unwrap_err();
        assert!(matches!(err, MigrationError::Edited(1)));
    }

    #[test]
    fn a_failing_migration_leaves_no_trace() {
        let mut conn = Connection::open_in_memory().unwrap();
        let broken = Migration {
            version: 1,
            name: "broken",
            sql: "CREATE TABLE c (id INTEGER); THIS IS NOT SQL;",
        };
        assert!(migrate(&mut conn, &[broken]).is_err());
        assert!(!table_exists(&conn, "c"));
        assert_eq!(applied_count(&conn), 0);
    }
}
