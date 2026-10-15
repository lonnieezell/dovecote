//! Connects to a real account over TLS and prints what the sync engine can rely on.
//! It only reads: no flags change and no mail moves.
//!
//! Usage: PROBE_HOST=imap.gmail.com PROBE_USER=you@gmail.com PROBE_PASSWORD='app password' \
//!        cargo run -p imap-spike --example probe
//! PROBE_PORT defaults to 993.

use std::{env, sync::Arc};

use tokio::net::TcpStream;
use tokio_rustls::{
    rustls::{pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};

fn var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("set {name}"))
}

#[tokio::main]
async fn main() {
    let host = var("PROBE_HOST");
    let port = env::var("PROBE_PORT").unwrap_or_else(|_| "993".into());

    let roots = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    let tcp = TcpStream::connect(format!("{host}:{port}")).await.unwrap();
    let tls = TlsConnector::from(Arc::new(config))
        .connect(ServerName::try_from(host).unwrap(), tcp)
        .await
        .unwrap();

    let mut session = async_imap::Client::new(tls)
        .login(var("PROBE_USER"), var("PROBE_PASSWORD"))
        .await
        .map_err(|(e, _)| e)
        .unwrap();
    print!("{}", imap_spike::probe(&mut session).await.unwrap());
    session.logout().await.unwrap();
}
