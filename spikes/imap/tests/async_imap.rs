mod common;

use std::time::Duration;

use async_imap::extensions::idle::IdleResponse;
use common::{
    addr, sample_message, unique_user, FakeGmail, Seeder, GMAIL_MSG_ID, GMAIL_THR_ID, PASSWORD,
};
use futures::TryStreamExt;
use imap_proto::{AttributeValue, Response};
use tokio::net::TcpStream;

type Session = async_imap::Session<TcpStream>;

async fn login(addr: &str, user: &str) -> Session {
    let client = async_imap::Client::new(TcpStream::connect(addr).await.unwrap());
    client.login(user, PASSWORD).await.map_err(|(e, _)| e).unwrap()
}

#[tokio::test]
async fn connects_and_fetches_a_message() {
    let user = unique_user("ai-fetch");
    Seeder::login(&user).await.append(&sample_message("fetch-me")).await;

    let mut session = login(&addr(), &user).await;
    let inbox = session.select("INBOX").await.unwrap();
    assert_eq!(inbox.exists, 1);

    let fetched: Vec<_> = session.uid_fetch("1", "(UID ENVELOPE)").await.unwrap().try_collect().await.unwrap();
    let subject = fetched[0].envelope().unwrap().subject.as_ref().unwrap();
    assert_eq!(&subject[..], b"fetch-me");
}

#[tokio::test]
async fn idle_wakes_when_mail_arrives() {
    let user = unique_user("ai-idle");
    let mut session = login(&addr(), &user).await;
    session.select("INBOX").await.unwrap();

    let mut idle = session.idle();
    idle.init().await.unwrap();
    let (wait, _stop) = idle.wait_with_timeout(Duration::from_secs(10));

    Seeder::login(&user).await.append(&sample_message("idle-wake")).await;

    assert!(matches!(wait.await.unwrap(), IdleResponse::NewData(_)));
    idle.done().await.unwrap();
}

#[tokio::test]
async fn condstore_returns_only_changed_flags() {
    let user = unique_user("ai-condstore");
    let mut seeder = Seeder::login(&user).await;
    seeder.append(&sample_message("one")).await;
    seeder.append(&sample_message("two")).await;

    let mut session = login(&addr(), &user).await;
    let modseq = session.select_condstore("INBOX").await.unwrap().highest_modseq.expect("CONDSTORE");

    seeder.add_flag(2, "\\Seen").await;

    let query = format!("(FLAGS) (CHANGEDSINCE {modseq})");
    let changed: Vec<_> = session.uid_fetch("1:*", query).await.unwrap().try_collect().await.unwrap();
    // The server may also push an unsolicited flag update without a UID, so skip those.
    assert_eq!(changed.iter().filter_map(|f| f.uid).collect::<Vec<_>>(), [2]);
}

#[tokio::test]
async fn gmail_attributes_go_out_as_a_raw_fetch_and_come_back_parsed() {
    let gmail = FakeGmail::start().await;
    let mut session = login(&gmail.addr, "any").await;
    session.select("INBOX").await.unwrap();

    let fetched: Vec<_> = session
        .uid_fetch("1", "(X-GM-MSGID X-GM-THRID X-GM-LABELS)")
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();

    let sent = gmail.commands.lock().unwrap().join("\n");
    assert!(sent.contains("UID FETCH 1 (X-GM-MSGID X-GM-THRID X-GM-LABELS)"), "{sent}");
    assert_eq!(fetched[0].gmail_msg_id(), Some(&GMAIL_MSG_ID));
    assert_eq!(fetched[0].gmail_labels().map(|l| l.len()), Some(2));
}

/// `Fetch` has no accessor for `X-GM-THRID`, but the low-level path hands back the parsed attributes.
#[tokio::test]
async fn gmail_thread_id_is_reachable_through_the_low_level_path() {
    let gmail = FakeGmail::start().await;
    let mut session = login(&gmail.addr, "any").await;
    session.select("INBOX").await.unwrap();

    session.run_command("UID FETCH 1 (X-GM-THRID)").await.unwrap();
    let mut thread_id = None;
    while let Some(data) = session.read_response().await.unwrap() {
        match data.parsed() {
            Response::Fetch(_, attrs) => {
                thread_id = attrs.iter().find_map(|a| match a {
                    AttributeValue::GmailThrId(id) => Some(*id),
                    _ => None,
                });
            }
            Response::Done { .. } => break,
            _ => {}
        }
    }
    assert_eq!(thread_id, Some(GMAIL_THR_ID));
}
