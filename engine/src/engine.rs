use std::path::Path;

use crate::{Command, Database, Event, MigrationError};

pub struct Engine {
    db: Database,
}

impl Engine {
    pub fn open(db_path: &Path) -> Result<Self, MigrationError> {
        Ok(Self { db: Database::open(db_path)? })
    }

    pub fn handle(&self, command: Command) -> Event {
        match command {
            Command::Ping { message } => Event::Pong {
                message,
                schema_version: self.db.schema_version().unwrap_or(0),
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
