use std::fmt;

use serde::{Deserialize, Serialize};

use crate::accounts::{Account, AccountSettings};
use crate::login::LoginCheck;
use crate::providers::{ServerSettings, Suggestion};

/// A password on its way from the UI to the Keychain. It never prints.
#[derive(Clone, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Password(String);

impl Password {
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Password {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Password(hidden)")
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Command {
    Ping { message: String },
    SuggestSettings { email: String },
    AddAccount { email: String, password: Password, settings: ServerSettings },
    ListAccounts,
    SaveAccountSettings { id: i64, settings: AccountSettings },
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Event {
    #[serde(rename_all = "camelCase")]
    Pong { message: String, schema_version: u32 },
    SettingsSuggested { email: String, suggestion: Option<Suggestion> },
    LoginsTested { email: String, imap: LoginCheck, smtp: LoginCheck },
    AccountAdded { account: Account },
    Accounts { accounts: Vec<Account> },
    Failed { message: String },
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
    fn add_account_reads_the_wire_shape_the_ui_sends() {
        let cmd: Command = serde_json::from_value(json!({
            "type": "addAccount",
            "email": "ada@example.com",
            "password": "pw",
            "settings": {"imapHost": "i", "imapPort": 993, "smtpHost": "s", "smtpPort": 465}
        }))
        .unwrap();
        assert!(matches!(cmd, Command::AddAccount { ref password, .. } if password.expose() == "pw"));
    }

    #[test]
    fn logins_tested_writes_the_wire_shape_the_ui_reads() {
        let check = |ok| LoginCheck { ok, message: "m".into() };
        let ev = Event::LoginsTested { email: "a@b.c".into(), imap: check(true), smtp: check(false) };
        assert_eq!(
            serde_json::to_value(ev).unwrap(),
            json!({"type": "loginsTested", "email": "a@b.c",
                   "imap": {"ok": true, "message": "m"}, "smtp": {"ok": false, "message": "m"}})
        );
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
