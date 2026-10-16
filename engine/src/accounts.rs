use serde::{Deserialize, Serialize};

use crate::providers::{domain_of, ServerSettings};

/// The account colors in `src/styles/tokens.css`, in the order new accounts take them.
pub const COLORS: [&str; 8] = [
    "account-1", "account-2", "account-3", "account-4", "account-5", "account-6", "account-7", "account-8",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub provider: String,
    pub server: ServerSettings,
    pub settings: AccountSettings,
}

/// The parts of an account the owner can change after adding it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSettings {
    pub display_name: String,
    pub color: String,
    pub signature: String,
    pub send_from: String,
}

impl AccountSettings {
    pub fn defaults(email: &str, existing_accounts: usize) -> Self {
        Self {
            display_name: email.into(),
            color: COLORS[existing_accounts % COLORS.len()].into(),
            signature: String::new(),
            send_from: email.into(),
        }
    }

    /// Why these settings cannot be saved, in plain words.
    pub fn problem(&self) -> Option<&'static str> {
        if self.display_name.trim().is_empty() {
            Some("The account needs a name.")
        } else if !COLORS.contains(&self.color.as_str()) {
            Some("Pick one of the account colors.")
        } else if domain_of(&self.send_from).is_none() {
            Some("The send-from address needs an @ and a domain.")
        } else {
            None
        }
    }
}
