mod common;

use common::{addr, login, unique_user, FakeGmail, GMAIL_MSG_ID, GMAIL_THR_ID};

#[tokio::test]
async fn reports_dovecot_capabilities_and_folders() {
    let mut session = login(&addr(), &unique_user("probe")).await;

    let report = imap_spike::probe(&mut session).await.unwrap();

    for line in [
        "IDLE: yes",
        "CONDSTORE: yes",
        "QRESYNC: yes",
        "SPECIAL-USE: yes",
        "X-GM-EXT-1: no",
        "INBOX",
        "Gmail fetch: skipped",
    ] {
        assert!(report.contains(line), "missing {line:?} in:\n{report}");
    }
}

#[tokio::test]
async fn reports_gmail_ids_and_labels() {
    let gmail = FakeGmail::start().await;
    let mut session = login(&gmail.addr, "any").await;

    let report = imap_spike::probe(&mut session).await.unwrap();

    for line in [
        "IDLE: yes".to_string(),
        "X-GM-EXT-1: yes".to_string(),
        "[Gmail]/Sent Mail [Extension(\"\\\\HasNoChildren\"), Sent]".to_string(),
        format!("GmailMsgId({GMAIL_MSG_ID})"),
        format!("GmailThrId({GMAIL_THR_ID})"),
        "Work".to_string(),
    ] {
        assert!(report.contains(&line), "missing {line:?} in:\n{report}");
    }
    let sent = gmail.commands.lock().unwrap().join("\n");
    // The fake INBOX holds one message, so the last-10 range starts at 1.
    assert!(sent.contains(" FETCH 1:* ("), "{sent}");
    assert!(
        sent.contains("EXAMINE") && !sent.contains("SELECT"),
        "must not open INBOX read-write:\n{sent}"
    );
}
