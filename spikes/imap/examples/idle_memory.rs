//! Holds one IDLE connection open and prints the process's resident memory (RSS).
//!
//! Usage: cargo run --release -p imap-spike --example idle_memory -- <baseline|async-imap|imap-next> [seconds]

use std::{process, time::Duration};

use imap_next::{
    client::{Client, Event, Options},
    imap_types::{
        command::{Command, CommandBody},
        core::Tag,
    },
    stream::Stream,
};
use tokio::{net::TcpStream, time::sleep};

const USER: &str = "memory-probe";
const PASSWORD: &str = "secret";

fn rss_kb() -> u64 {
    let out = process::Command::new("ps").args(["-o", "rss=", "-p", &process::id().to_string()]).output().unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

async fn hold_with_async_imap(addr: &str, hold: Duration) {
    let client = async_imap::Client::new(TcpStream::connect(addr).await.unwrap());
    let mut session = client.login(USER, PASSWORD).await.map_err(|(e, _)| e).unwrap();
    session.select("INBOX").await.unwrap();
    let mut idle = session.idle();
    idle.init().await.unwrap();
    let (wait, _stop) = idle.wait_with_timeout(hold + Duration::from_secs(5));
    let _ = tokio::time::timeout(hold, wait).await;
}

async fn hold_with_imap_next(addr: &str, hold: Duration) {
    let mut stream = Stream::insecure(TcpStream::connect(addr).await.unwrap());
    let mut client = Client::new(Options::default());
    loop {
        if let Event::GreetingReceived { .. } = stream.next(&mut client).await.unwrap() {
            break;
        }
    }
    client.enqueue_command(Command {
        tag: Tag::try_from("A1").unwrap(),
        body: CommandBody::login(USER, PASSWORD).unwrap(),
    });
    client.enqueue_command(Command { tag: Tag::try_from("A2").unwrap(), body: CommandBody::select("INBOX").unwrap() });
    client.enqueue_command(Command { tag: Tag::try_from("A3").unwrap(), body: CommandBody::Idle });
    let hold_for = async {
        loop {
            if let Event::IdleAccepted { .. } = stream.next(&mut client).await.unwrap() {
                break;
            }
        }
        // Keep driving the connection while idle, as a real client would.
        loop {
            stream.next(&mut client).await.unwrap();
        }
    };
    let _ = tokio::time::timeout(hold, hold_for).await;
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let which = args.next().expect("baseline | async-imap | imap-next");
    let hold = Duration::from_secs(args.next().map_or(20, |s| s.parse().unwrap()));
    let addr = std::env::var("IMAP_SPIKE_ADDR").unwrap_or_else(|_| "127.0.0.1:31143".into());

    println!("rss_kb_at_start={}", rss_kb());
    match which.as_str() {
        "baseline" => {
            sleep(hold).await;
                }
        "async-imap" => hold_with_async_imap(&addr, hold).await,
        "imap-next" => hold_with_imap_next(&addr, hold).await,
        other => panic!("unknown mode {other}"),
    }
    println!("rss_kb_after_hold={}", rss_kb());
}
