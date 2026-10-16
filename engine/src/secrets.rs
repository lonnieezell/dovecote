/// Where account passwords live. Each account has one item, named by its account id.
pub trait SecretStore: Send + Sync {
    fn set(&self, account_id: i64, secret: &str) -> Result<(), String>;
}

/// The macOS Keychain, through the `keyring` crate.
pub struct Keychain {
    service: String,
}

impl Keychain {
    pub fn new(service: impl Into<String>) -> Self {
        Self { service: service.into() }
    }

    fn entry(&self, account_id: i64) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service, &format!("account-{account_id}")).map_err(|e| e.to_string())
    }
}

impl SecretStore for Keychain {
    fn set(&self, account_id: i64, secret: &str) -> Result<(), String> {
        self.entry(account_id)?.set_password(secret).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct MemorySecrets {
    items: std::sync::Mutex<std::collections::HashMap<i64, String>>,
    fail_next_write: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl MemorySecrets {
    pub fn get(&self, account_id: i64) -> Option<String> {
        self.items.lock().unwrap().get(&account_id).cloned()
    }

    pub fn fail_next_write(&self) {
        self.fail_next_write.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
impl SecretStore for MemorySecrets {
    fn set(&self, account_id: i64, secret: &str) -> Result<(), String> {
        if self.fail_next_write.swap(false, std::sync::atomic::Ordering::SeqCst) {
            return Err("User canceled the operation.".into());
        }
        self.items.lock().unwrap().insert(account_id, secret.into());
        Ok(())
    }
}
