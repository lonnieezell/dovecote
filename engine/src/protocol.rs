use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    Ping { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    Pong { message: String, schema_version: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn command_reads_the_wire_shape_the_ui_sends() {
        let cmd: Command = serde_json::from_value(json!({"type": "ping", "message": "hi"})).unwrap();
        assert_eq!(cmd, Command::Ping { message: "hi".into() });
    }

    #[test]
    fn event_writes_the_wire_shape_the_ui_reads() {
        let ev = Event::Pong { message: "hi".into(), schema_version: 1 };
        assert_eq!(
            serde_json::to_value(ev).unwrap(),
            json!({"type": "pong", "message": "hi", "schemaVersion": 1})
        );
    }
}
