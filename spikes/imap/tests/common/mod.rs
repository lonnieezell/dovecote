//! Helpers shared by the crate tests. Seeding goes over a plain TCP socket, so it does not
//! depend on the crate under test.

use std::{
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

pub const PASSWORD: &str = "secret";

pub fn addr() -> String {
    std::env::var("IMAP_SPIKE_ADDR").unwrap_or_else(|_| "127.0.0.1:31143".into())
}

/// Dovecot accepts any user name, and each name gets its own empty mailbox.
pub fn unique_user(prefix: &str) -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("{prefix}-{}-{nanos}", std::process::id())
}

pub fn sample_message(subject: &str) -> String {
    format!(
        "From: Ada <ada@example.com>\r\nTo: you@example.com\r\nSubject: {subject}\r\n\
         Message-ID: <{subject}@example.com>\r\nDate: Thu, 15 Oct 2026 10:00:00 +0000\r\n\r\nHello.\r\n"
    )
}

pub struct Seeder {
    reader: BufReader<TcpStream>,
    next_tag: u32,
}

impl Seeder {
    pub async fn login(user: &str) -> Self {
        let tcp = TcpStream::connect(addr())
            .await
            .expect("is Dovecot running? see spikes/imap/README.md");
        let mut seeder = Seeder { reader: BufReader::new(tcp), next_tag: 1 };
        let mut greeting = String::new();
        seeder.reader.read_line(&mut greeting).await.unwrap();
        seeder.run(&format!("LOGIN {user} {PASSWORD}")).await;
        seeder
    }

    /// Sends one command and returns every line up to and including the tagged OK.
    pub async fn run(&mut self, command: &str) -> Vec<String> {
        let tag = format!("S{}", self.next_tag);
        self.next_tag += 1;
        self.reader
            .get_mut()
            .write_all(format!("{tag} {command}\r\n").as_bytes())
            .await
            .unwrap();
        self.read_until_tagged(&tag).await
    }

    pub async fn append(&mut self, message: &str) {
        let command = format!("APPEND INBOX {{{}+}}\r\n{message}", message.len());
        self.run(&command).await;
    }

    pub async fn add_flag(&mut self, uid: u32, flag: &str) {
        self.run("SELECT INBOX").await;
        self.run(&format!("UID STORE {uid} +FLAGS ({flag})")).await;
    }

    async fn read_until_tagged(&mut self, tag: &str) -> Vec<String> {
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            let read = self.reader.read_line(&mut line).await.unwrap();
            assert!(read > 0, "server closed: {lines:?}");
            let done = line.starts_with(tag);
            lines.push(line);
            if done {
                assert!(lines.last().unwrap().contains(" OK"), "command failed: {lines:?}");
                return lines;
            }
        }
    }
}

/// A one-connection IMAP server that answers like Gmail does for `X-GM-*` fetches.
/// Dovecot has no Gmail extensions, so this stands in for them.
pub struct FakeGmail {
    pub addr: String,
    /// Every command line the client sent, in order.
    pub commands: Arc<Mutex<Vec<String>>>,
}

pub const GMAIL_MSG_ID: u64 = 1278455344230334865;
pub const GMAIL_THR_ID: u64 = 1278455344230334866;

impl FakeGmail {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let seen = commands.clone();
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut reader = BufReader::new(tcp);
            let greeting = b"* OK [CAPABILITY IMAP4rev1 X-GM-EXT-1] ready\r\n";
            reader.get_mut().write_all(greeting).await.unwrap();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap() == 0 {
                    return;
                }
                let line = line.trim_end().to_string();
                seen.lock().unwrap().push(line.clone());
                let tag = line.split_whitespace().next().unwrap_or("*").to_string();
                let upper = line.to_uppercase();
                let reply = if upper.contains("UID FETCH") {
                    format!(
                        "* 1 FETCH (UID 1 X-GM-MSGID {GMAIL_MSG_ID} X-GM-THRID {GMAIL_THR_ID} \
                         X-GM-LABELS (\"\\\\Inbox\" Work))\r\n{tag} OK done\r\n"
                    )
                } else if upper.contains("SELECT") {
                    format!(
                        "* 1 EXISTS\r\n* OK [UIDVALIDITY 1] ok\r\n{tag} OK [READ-WRITE] done\r\n"
                    )
                } else if upper.contains("LOGOUT") {
                    format!("* BYE bye\r\n{tag} OK done\r\n")
                } else {
                    format!("{tag} OK done\r\n")
                };
                reader.get_mut().write_all(reply.as_bytes()).await.unwrap();
            }
        });
        FakeGmail { addr, commands }
    }
}
