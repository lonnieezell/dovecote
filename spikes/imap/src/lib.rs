//! Probes a logged-in IMAP account for the features the sync engine relies on.
//! `examples/probe.rs` runs it against a real account over TLS.

use std::fmt::{Debug, Write as _};

use async_imap::{error::Result, imap_proto::Response, types::Capability, Session};
use futures::TryStreamExt;
use tokio::io::{AsyncRead, AsyncWrite};

const WANTED: [&str; 5] = ["IDLE", "CONDSTORE", "QRESYNC", "SPECIAL-USE", "X-GM-EXT-1"];

/// Returns a plain-text report. It only reads: INBOX is opened with EXAMINE, and the Gmail
/// fetch asks for no body, so no message gets marked as seen.
pub async fn probe<T>(session: &mut Session<T>) -> Result<String>
where
    T: AsyncRead + AsyncWrite + Unpin + Debug + Send,
{
    let mut report = String::new();

    let caps = session.capabilities().await?;
    let mut all: Vec<String> = caps
        .iter()
        .map(|c| match c {
            Capability::Imap4rev1 => "IMAP4rev1".to_string(),
            Capability::Auth(a) => format!("AUTH={a}"),
            Capability::Atom(a) => a.clone(),
        })
        .collect();
    all.sort();
    writeln!(report, "Capabilities: {}", all.join(" ")).unwrap();
    for cap in WANTED {
        let answer = if caps.has_str(cap) { "yes" } else { "no" };
        writeln!(report, "{cap}: {answer}").unwrap();
    }

    writeln!(report, "Folders:").unwrap();
    let names: Vec<_> = session
        .list(Some(""), Some("*"))
        .await?
        .try_collect()
        .await?;
    for name in &names {
        writeln!(report, "  {} {:?}", name.name(), name.attributes()).unwrap();
    }

    if !caps.has_str("X-GM-EXT-1") {
        writeln!(report, "Gmail fetch: skipped").unwrap();
        return Ok(report);
    }
    let exists = session.examine("INBOX").await?.exists;
    if exists == 0 {
        writeln!(report, "Gmail fetch: INBOX is empty").unwrap();
        return Ok(report);
    }
    // The last 10 messages, so a reply in a thread can show a shared X-GM-THRID.
    // `Fetch` has no accessor for X-GM-THRID, so read the parsed attributes directly.
    let first = exists.saturating_sub(9).max(1);
    session
        .run_command(format!(
            "FETCH {first}:* (UID X-GM-MSGID X-GM-THRID X-GM-LABELS)"
        ))
        .await?;
    while let Some(data) = session.read_response().await? {
        match data.parsed() {
            Response::Fetch(_, attrs) => writeln!(report, "Gmail fetch: {attrs:?}").unwrap(),
            Response::Done { .. } => break,
            _ => {}
        }
    }
    Ok(report)
}
