use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use crate::db::{AddAccountError, Database};
use crate::login::{check_imap, check_smtp, Login, Security};
use crate::providers::{domain_of, provider_for, suggest, ServerSettings};
use crate::secrets::SecretStore;
use crate::{AccountSettings, Command, Event, MigrationError, Password};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Engine {
    db: Mutex<Database>,
    secrets: Arc<dyn SecretStore>,
    security: Security,
}

impl Engine {
    pub fn open(db_path: &Path, secrets: Arc<dyn SecretStore>) -> Result<Self, MigrationError> {
        Ok(Self {
            db: Mutex::new(Database::open(db_path)?),
            secrets,
            security: Security::Tls,
        })
    }

    /// Returns the events that answer this command.
    pub async fn handle(&self, command: Command) -> Vec<Event> {
        match command {
            Command::Ping { message } => vec![match self.db().schema_version() {
                Ok(schema_version) => Event::Pong { message, schema_version },
                Err(e) => failed(format!("Could not read the database: {e}")),
            }],
            Command::SuggestSettings { email } => {
                let suggestion = suggest(&email).await;
                vec![Event::SettingsSuggested { email, suggestion }]
            }
            Command::AddAccount { email, password, settings } => self.add_account(email.trim(), &password, &settings).await,
            Command::ListAccounts => vec![self.accounts()],
            Command::SaveAccountSettings { id, settings } => vec![self.save_account_settings(id, &settings)],
        }
    }

    async fn add_account(&self, email: &str, password: &Password, server: &ServerSettings) -> Vec<Event> {
        if domain_of(email).is_none() {
            return vec![failed("That does not look like an email address.".into())];
        }
        if [&server.imap_host, &server.smtp_host].iter().any(|h| h.trim().is_empty())
            || server.imap_port == 0
            || server.smtp_port == 0
        {
            return vec![failed("Fill in both servers and their ports.".into())];
        }
        let login = |host, port| Login { host, port, user: email, password: password.expose() };
        let imap_login = login(&server.imap_host, server.imap_port);
        let smtp_login = login(&server.smtp_host, server.smtp_port);
        let (imap, smtp) = tokio::join!(
            check_imap(&imap_login, self.security, LOGIN_TIMEOUT),
            check_smtp(&smtp_login, self.security, LOGIN_TIMEOUT),
        );
        let both_ok = imap.ok && smtp.ok;
        let mut events = vec![Event::LoginsTested { email: email.into(), imap, smtp }];
        if !both_ok {
            return events;
        }
        let added = self.db().add_account(email, provider_for(server), server);
        let account = match added {
            Ok(account) => account,
            Err(AddAccountError::AlreadyAdded) => {
                events.push(failed(format!("{email} is already added.")));
                return events;
            }
            Err(AddAccountError::Sql(e)) => {
                events.push(failed(format!("Could not save the account: {e}")));
                return events;
            }
        };
        // The Keychain call can block on a system prompt, so it runs off the async threads
        // and without the database lock. If it fails, the account is taken back out.
        let (secrets, id, secret) = (self.secrets.clone(), account.id, password.expose().to_owned());
        let stored = tokio::task::spawn_blocking(move || secrets.set(id, &secret))
            .await
            .unwrap_or_else(|e| Err(e.to_string()));
        match stored {
            Ok(()) => {
                events.push(Event::AccountAdded { account });
                events.push(self.accounts());
            }
            Err(e) => {
                let _ = self.db().delete_account(id);
                events.push(failed(format!("Could not save the password in the Keychain: {e}")));
            }
        }
        events
    }

    fn accounts(&self) -> Event {
        match self.db().accounts() {
            Ok(accounts) => Event::Accounts { accounts },
            Err(e) => failed(format!("Could not read the accounts: {e}")),
        }
    }

    fn save_account_settings(&self, id: i64, settings: &AccountSettings) -> Event {
        if let Some(problem) = settings.problem() {
            return failed(problem.into());
        }
        // Release the lock before matching: the Ok(true) arm takes it again.
        let saved = self.db().save_account_settings(id, settings);
        match saved {
            Ok(true) => self.accounts(),
            Ok(false) => failed("That account no longer exists.".into()),
            Err(e) => failed(format!("Could not save the settings: {e}")),
        }
    }

    fn db(&self) -> MutexGuard<'_, Database> {
        // A panic while holding the lock can't leave the database half-written: every
        // write runs in a transaction. So a poisoned lock is safe to keep using.
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}

fn failed(message: String) -> Event {
    Event::Failed { message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::fake;
    use crate::secrets::MemorySecrets;
    use crate::{AccountSettings, LoginCheck, Password, ServerSettings};

    struct Harness {
        engine: Engine,
        secrets: Arc<MemorySecrets>,
        dir: tempfile::TempDir,
    }

    fn harness() -> Harness {
        let dir = tempfile::tempdir().unwrap();
        let secrets = Arc::new(MemorySecrets::default());
        let mut engine = Engine::open(&dir.path().join("mail.db"), secrets.clone()).unwrap();
        engine.security = crate::login::Security::Plain;
        Harness { engine, secrets, dir }
    }

    async fn servers(imap_ok: bool, smtp_ok: bool) -> ServerSettings {
        ServerSettings {
            imap_host: "127.0.0.1".into(),
            imap_port: fake::imap(imap_ok).await,
            smtp_host: "127.0.0.1".into(),
            smtp_port: fake::smtp(smtp_ok).await,
        }
    }

    fn add(email: &str, password: &str, settings: ServerSettings) -> Command {
        Command::AddAccount { email: email.into(), password: Password::from(password), settings }
    }

    fn added(events: &[Event]) -> crate::Account {
        events
            .iter()
            .find_map(|e| match e {
                Event::AccountAdded { account } => Some(account.clone()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no account added: {events:?}"))
    }

    async fn list(engine: &Engine) -> Vec<crate::Account> {
        match engine.handle(Command::ListAccounts).await.as_slice() {
            [Event::Accounts { accounts }] => accounts.clone(),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn ping_comes_back_as_pong_with_the_schema_version() {
        let h = harness();
        let events = h.engine.handle(Command::Ping { message: "hello".into() }).await;
        assert!(matches!(
            events.as_slice(),
            [Event::Pong { message, schema_version }] if message == "hello" && *schema_version >= 1
        ));
    }

    #[tokio::test]
    async fn a_gmail_address_gets_gmail_settings() {
        let h = harness();
        let events = h.engine.handle(Command::SuggestSettings { email: "ada@gmail.com".into() }).await;
        match events.as_slice() {
            [Event::SettingsSuggested { email, suggestion: Some(s) }] => {
                assert_eq!(email, "ada@gmail.com");
                assert_eq!(s.name, "Gmail");
                assert_eq!(s.settings.imap_host, "imap.gmail.com");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[tokio::test]
    async fn something_that_is_not_an_address_gets_no_suggestion() {
        let h = harness();
        let events = h.engine.handle(Command::SuggestSettings { email: "ada".into() }).await;
        assert!(matches!(events.as_slice(), [Event::SettingsSuggested { suggestion: None, .. }]));
    }

    #[tokio::test]
    async fn adding_an_account_tests_both_logins_then_saves_it_and_its_password() {
        let h = harness();
        let events = h.engine.handle(add("ada@example.com", "hunter2", servers(true, true).await)).await;

        let Event::LoginsTested { imap, smtp, .. } = &events[0] else { panic!("{events:?}") };
        assert!(imap.ok && smtp.ok, "{events:?}");
        let account = added(&events);
        assert_eq!(account.email, "ada@example.com");
        assert_eq!(account.provider, "imap");
        assert_eq!(
            account.settings,
            AccountSettings {
                display_name: "ada@example.com".into(),
                color: "account-1".into(),
                signature: String::new(),
                send_from: "ada@example.com".into(),
            }
        );
        assert_eq!(h.secrets.get(account.id).as_deref(), Some("hunter2"));
        assert!(matches!(events.last(), Some(Event::Accounts { accounts }) if *accounts == vec![account.clone()]));
        assert_eq!(list(&h.engine).await, vec![account]);
    }

    #[tokio::test]
    async fn each_new_account_takes_the_next_color() {
        let h = harness();
        h.engine.handle(add("ada@example.com", "pw", servers(true, true).await)).await;
        let events = h.engine.handle(add("bob@example.com", "pw", servers(true, true).await)).await;
        assert_eq!(added(&events).settings.color, "account-2");
    }

    #[tokio::test]
    async fn a_failed_login_reports_each_result_and_saves_nothing() {
        let h = harness();
        let events = h.engine.handle(add("ada@example.com", "wrong", servers(false, true).await)).await;
        match events.as_slice() {
            [Event::LoginsTested { imap: LoginCheck { ok: false, .. }, smtp: LoginCheck { ok: true, .. }, .. }] => {}
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(h.secrets.get(1), None);
        assert!(list(&h.engine).await.is_empty());
    }

    #[tokio::test]
    async fn an_add_with_a_bad_address_or_missing_server_is_refused_before_any_login() {
        let h = harness();
        let good = servers(true, true).await;
        let cases = [
            ("ada", good.clone(), "That does not look like an email address."),
            ("ada@example.com", ServerSettings { imap_host: " ".into(), ..good.clone() }, "Fill in both servers and their ports."),
            ("ada@example.com", ServerSettings { smtp_port: 0, ..good }, "Fill in both servers and their ports."),
        ];
        for (email, settings, expected) in cases {
            let events = h.engine.handle(add(email, "pw", settings)).await;
            assert!(matches!(events.as_slice(), [Event::Failed { message }] if message == expected), "{events:?}");
        }
        assert!(list(&h.engine).await.is_empty());
    }

    #[tokio::test]
    async fn adding_the_same_address_twice_is_refused_and_keeps_the_first_password() {
        let h = harness();
        let id = added(&h.engine.handle(add("ada@example.com", "first", servers(true, true).await)).await).id;
        let events = h.engine.handle(add("ADA@example.com", "second", servers(true, true).await)).await;
        assert!(
            matches!(events.last(), Some(Event::Failed { message }) if message == "ADA@example.com is already added."),
            "{events:?}"
        );
        assert_eq!(h.secrets.get(id).as_deref(), Some("first"));
        assert_eq!(h.secrets.get(id + 1), None);
        assert_eq!(list(&h.engine).await.len(), 1);
    }

    #[tokio::test]
    async fn when_the_keychain_refuses_the_account_is_not_saved() {
        let h = harness();
        h.secrets.fail_next_write();
        let events = h.engine.handle(add("ada@example.com", "pw", servers(true, true).await)).await;
        assert!(
            matches!(events.last(), Some(Event::Failed { message }) if message.starts_with("Could not save the password in the Keychain")),
            "{events:?}"
        );
        assert!(list(&h.engine).await.is_empty());
    }

    #[tokio::test]
    async fn the_password_stays_out_of_the_database_events_and_debug_output() {
        let h = harness();
        let password = "a-very-distinct-password-1234";
        let command = add("ada@example.com", password, servers(true, true).await);
        assert!(!format!("{command:?}").contains(password));

        let mut events = h.engine.handle(command).await;
        events.extend(h.engine.handle(Command::ListAccounts).await);
        for event in &events {
            assert!(!serde_json::to_string(event).unwrap().contains(password));
        }
        for file in std::fs::read_dir(h.dir.path()).unwrap() {
            let bytes = std::fs::read(file.unwrap().path()).unwrap();
            assert!(!bytes.windows(password.len()).any(|w| w == password.as_bytes()));
        }
    }

    #[tokio::test]
    async fn per_account_settings_can_be_saved() {
        let h = harness();
        let id = added(&h.engine.handle(add("ada@example.com", "pw", servers(true, true).await)).await).id;
        let settings = AccountSettings {
            display_name: "Personal".into(),
            color: "account-4".into(),
            signature: "Ada\n-- sent from my desk".into(),
            send_from: "ada@lovelace.example".into(),
        };
        let events = h.engine.handle(Command::SaveAccountSettings { id, settings: settings.clone() }).await;
        match events.as_slice() {
            [Event::Accounts { accounts }] => assert_eq!(accounts[0].settings, settings),
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(list(&h.engine).await[0].settings, settings);
    }

    #[tokio::test]
    async fn settings_that_do_not_make_sense_are_refused_in_plain_words() {
        let h = harness();
        let id = added(&h.engine.handle(add("ada@example.com", "pw", servers(true, true).await)).await).id;
        let good = AccountSettings {
            display_name: "Personal".into(),
            color: "account-2".into(),
            signature: String::new(),
            send_from: "ada@example.com".into(),
        };
        let cases = [
            (AccountSettings { display_name: "  ".into(), ..good.clone() }, "The account needs a name."),
            (AccountSettings { color: "red".into(), ..good.clone() }, "Pick one of the account colors."),
            (AccountSettings { send_from: "ada".into(), ..good.clone() }, "The send-from address needs an @ and a domain."),
        ];
        for (settings, expected) in cases {
            let events = h.engine.handle(Command::SaveAccountSettings { id, settings }).await;
            assert!(matches!(events.as_slice(), [Event::Failed { message }] if message == expected), "{events:?}");
        }
        let events = h.engine.handle(Command::SaveAccountSettings { id: id + 100, settings: good }).await;
        assert!(matches!(events.as_slice(), [Event::Failed { message }] if message == "That account no longer exists."));
    }
}
