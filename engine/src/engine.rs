use std::path::Path;

use crate::db::Database;
use crate::{Command, Event, MigrationError};

pub struct Engine {
    schema_version: u32,
}

impl Engine {
    pub fn open(db_path: &Path) -> Result<Self, MigrationError> {
        let db = Database::open(db_path)?;
        Ok(Self { schema_version: db.schema_version()? })
    }

    pub fn handle(&self, command: Command) -> Event {
        match command {
            Command::Ping { message } => Event::Pong {
                message,
                schema_version: self.schema_version,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ping_comes_back_as_pong_with_the_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(&dir.path().join("mail.db")).unwrap();
        let event = engine.handle(Command::Ping { message: "hello".into() });
        assert!(matches!(
            event,
            Event::Pong { ref message, schema_version } if message == "hello" && schema_version >= 1
        ));
    }
}
