mod common;

use std::{num::NonZeroU64, time::Duration};

use common::{addr, sample_message, unique_user, FakeGmail, Seeder, GMAIL_THR_ID, PASSWORD};
use imap_next::{
    client::{self, Client, Event, Options},
    imap_types::{
        command::{Command, CommandBody, FetchModifier, SelectParameter},
        core::Tag,
        fetch::{MessageDataItem, MessageDataItemName},
        response::{Code, Data, Status, StatusKind},
        sequence::SequenceSet,
    },
    stream::{Error, Stream},
};
use tokio::{io::AsyncWriteExt, net::TcpStream, time::timeout};

/// What the server sent while one command ran.
struct Reply {
    data: Vec<Data<'static>>,
    untagged: Vec<Status<'static>>,
}

struct Conn {
    stream: Stream,
    client: Client,
    tags: u32,
}

impl Conn {
    async fn connect(addr: &str) -> Conn {
        let tcp = TcpStream::connect(addr).await.unwrap();
        let mut conn = Conn {
            stream: Stream::insecure(tcp),
            client: Client::new(Options::default()),
            tags: 0,
        };
        loop {
            if let Event::GreetingReceived { .. } =
                conn.stream.next(&mut conn.client).await.unwrap()
            {
                return conn;
            }
        }
    }

    async fn login(addr: &str, user: &str) -> Conn {
        let mut conn = Conn::connect(addr).await;
        conn.run(CommandBody::login(user.to_string(), PASSWORD).unwrap())
            .await;
        conn
    }

    async fn run(&mut self, body: CommandBody<'static>) -> Reply {
        self.tags += 1;
        let tag = Tag::try_from(format!("A{}", self.tags)).unwrap();
        self.client.enqueue_command(Command {
            tag: tag.clone(),
            body,
        });
        let mut reply = Reply {
            data: Vec::new(),
            untagged: Vec::new(),
        };
        loop {
            match self.stream.next(&mut self.client).await.unwrap() {
                Event::DataReceived { data } => reply.data.push(data),
                Event::StatusReceived {
                    status: Status::Tagged(tagged),
                } if tagged.tag == tag => {
                    assert_eq!(tagged.body.kind, StatusKind::Ok, "{:?}", tagged.body);
                    return reply;
                }
                Event::StatusReceived { status } => reply.untagged.push(status),
                _ => {}
            }
        }
    }
}

fn fetch_items(items: Vec<MessageDataItemName<'static>>, uid: bool) -> CommandBody<'static> {
    CommandBody::Fetch {
        sequence_set: SequenceSet::try_from("1:*").unwrap(),
        macro_or_item_names: items.into(),
        uid,
        modifiers: Vec::new(),
    }
}

fn fetched_uids(data: &[Data<'static>]) -> Vec<u32> {
    data.iter()
        .filter_map(|d| match d {
            Data::Fetch { items, .. } => items.as_ref().iter().find_map(|i| match i {
                MessageDataItem::Uid(uid) => Some(uid.get()),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn connects_and_fetches_a_message() {
    let user = unique_user("in-fetch");
    Seeder::login(&user)
        .await
        .append(&sample_message("fetch-me"))
        .await;

    let mut conn = Conn::login(&addr(), &user).await;
    conn.run(CommandBody::select("INBOX").unwrap()).await;
    let reply = conn
        .run(fetch_items(
            vec![MessageDataItemName::Uid, MessageDataItemName::Envelope],
            true,
        ))
        .await;

    assert_eq!(fetched_uids(&reply.data), [1]);
    assert!(
        format!("{:?}", reply.data).contains("fetch-me"),
        "{:?}",
        reply.data
    );
}

#[tokio::test]
async fn idle_wakes_when_mail_arrives() {
    let user = unique_user("in-idle");
    let mut conn = Conn::login(&addr(), &user).await;
    conn.run(CommandBody::select("INBOX").unwrap()).await;

    conn.client.enqueue_command(Command {
        tag: Tag::try_from("IDLE1").unwrap(),
        body: CommandBody::Idle,
    });
    loop {
        if let Event::IdleAccepted { .. } = conn.stream.next(&mut conn.client).await.unwrap() {
            break;
        }
    }

    Seeder::login(&user)
        .await
        .append(&sample_message("idle-wake"))
        .await;

    let pushed = timeout(Duration::from_secs(10), async {
        loop {
            if let Event::DataReceived {
                data: Data::Exists(1),
            } = conn.stream.next(&mut conn.client).await.unwrap()
            {
                return;
            }
        }
    })
    .await;
    assert!(pushed.is_ok(), "no EXISTS push during IDLE");

    conn.client.set_idle_done().unwrap();
    loop {
        if let Event::StatusReceived {
            status: Status::Tagged(_),
        } = conn.stream.next(&mut conn.client).await.unwrap()
        {
            break;
        }
    }
}

#[tokio::test]
async fn condstore_returns_only_changed_flags() {
    let user = unique_user("in-condstore");
    let mut seeder = Seeder::login(&user).await;
    seeder.append(&sample_message("one")).await;
    seeder.append(&sample_message("two")).await;

    let mut conn = Conn::login(&addr(), &user).await;
    let select = conn
        .run(CommandBody::Select {
            mailbox: "INBOX".try_into().unwrap(),
            parameters: vec![SelectParameter::CondStore],
        })
        .await;
    let modseq = select
        .untagged
        .iter()
        .find_map(|s| match s {
            Status::Untagged(body) => match body.code {
                Some(Code::HighestModSeq(n)) => Some(n),
                _ => None,
            },
            _ => None,
        })
        .expect("HIGHESTMODSEQ");

    seeder.add_flag(2, "\\Seen").await;

    let mut fetch = fetch_items(
        vec![MessageDataItemName::Uid, MessageDataItemName::Flags],
        true,
    );
    if let CommandBody::Fetch { modifiers, .. } = &mut fetch {
        modifiers.push(FetchModifier::ChangedSince(
            NonZeroU64::new(modseq.get()).unwrap(),
        ));
    }
    let reply = conn.run(fetch).await;
    assert_eq!(fetched_uids(&reply.data), [2]);
}

/// The typed API has no way to name `X-GM-THRID`, so the only route is writing the command
/// onto the socket by hand. The parser then rejects Gmail's answer as malformed. The values
/// survive only in the discarded bytes, and the connection keeps working afterward.
#[tokio::test]
async fn gmail_attributes_are_rejected_but_recoverable_from_discarded_bytes() {
    let gmail = FakeGmail::start().await;
    let mut conn = Conn::login(&gmail.addr, "any").await;
    conn.run(CommandBody::select("INBOX").unwrap()).await;

    let raw = b"RAW1 UID FETCH 1 (X-GM-MSGID X-GM-THRID X-GM-LABELS)\r\n";
    conn.stream.stream_mut().write_all(raw).await.unwrap();

    let error = conn.stream.next(&mut conn.client).await.unwrap_err();
    let Error::State(client::Error::MalformedMessage { discarded_bytes }) = error else {
        panic!("expected a malformed message, got {error:?}");
    };
    let discarded = String::from_utf8_lossy(discarded_bytes.declassify()).into_owned();
    assert!(
        discarded.contains(&format!("X-GM-THRID {GMAIL_THR_ID}")),
        "{discarded}"
    );

    let next = conn.stream.next(&mut conn.client).await.unwrap();
    assert!(
        matches!(
            next,
            Event::StatusReceived {
                status: Status::Tagged(_)
            }
        ),
        "{next:?}"
    );
    conn.run(CommandBody::Noop).await;
}
