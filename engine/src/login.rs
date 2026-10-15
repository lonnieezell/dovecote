use std::{sync::Arc, time::Duration};

use lettre::transport::smtp::{
    authentication::{Credentials, Mechanism},
    client::{AsyncSmtpConnection, TlsParameters},
    extension::ClientId,
};
use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    time::error::Elapsed,
};
use tokio_rustls::{
    rustls::{pki_types::ServerName, ClientConfig, RootCertStore},
    TlsConnector,
};

/// The result of one login test, worded for the person adding the account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoginCheck {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Security {
    Tls,
    /// Only for the fake servers in tests.
    #[cfg(test)]
    Plain,
}

pub(crate) struct Login<'a> {
    pub host: &'a str,
    pub port: u16,
    pub user: &'a str,
    pub password: &'a str,
}

enum Failure {
    Unreachable,
    Insecure,
    Untrusted,
    Refused(String),
    Other(String),
}

pub(crate) async fn check_imap(login: &Login<'_>, security: Security, timeout: Duration) -> LoginCheck {
    describe(login, tokio::time::timeout(timeout, imap(login, security)).await)
}

pub(crate) async fn check_smtp(login: &Login<'_>, security: Security, timeout: Duration) -> LoginCheck {
    describe(login, tokio::time::timeout(timeout, smtp(login, security)).await)
}

fn describe(login: &Login<'_>, outcome: Result<Result<(), Failure>, Elapsed>) -> LoginCheck {
    let host = login.host;
    let message = match outcome {
        Ok(Ok(())) => return LoginCheck { ok: true, message: format!("Signed in to {host}.") },
        Err(Elapsed { .. }) => format!("{host} did not answer in time. Try again in a moment."),
        Ok(Err(Failure::Unreachable)) => format!(
            "Could not reach {host} on port {}. Check the server name, the port and your internet connection.",
            login.port
        ),
        Ok(Err(Failure::Insecure)) => format!(
            "Reached {host}, but could not set up a secure connection. Check the port: it should be the one for SSL/TLS."
        ),
        Ok(Err(Failure::Untrusted)) => format!(
            "The security certificate from {host} can't be trusted, so the password was not sent. Check the server name."
        ),
        Ok(Err(Failure::Refused(text))) if text.contains("Application-specific password required") => {
            "Gmail needs an app password here, not your normal password. Turn on 2-Step Verification \
             in your Google account, then create an app password and use that."
                .into()
        }
        Ok(Err(Failure::Refused(_))) => {
            format!("{host} did not accept that address and password. Check both and try again.")
        }
        Ok(Err(Failure::Other(detail))) => format!("Something went wrong talking to {host}: {detail}"),
    };
    LoginCheck { ok: false, message }
}

async fn imap(login: &Login<'_>, security: Security) -> Result<(), Failure> {
    let tcp = TcpStream::connect((login.host, login.port))
        .await
        .map_err(|_| Failure::Unreachable)?;
    match security {
        Security::Tls => {
            let name = ServerName::try_from(login.host.to_owned()).map_err(|_| Failure::Unreachable)?;
            let roots = RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
            let config = ClientConfig::builder().with_root_certificates(roots).with_no_client_auth();
            let tls = TlsConnector::from(Arc::new(config))
                .connect(name, tcp)
                .await
                .map_err(|e| handshake_failure(&e).unwrap_or(Failure::Insecure))?;
            imap_login(tls, login).await
        }
        #[cfg(test)]
        Security::Plain => imap_login(tcp, login).await,
    }
}

async fn imap_login<S>(stream: S, login: &Login<'_>) -> Result<(), Failure>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + std::fmt::Debug,
{
    let mut session = async_imap::Client::new(stream)
        .login(login.user, login.password)
        .await
        .map_err(|(e, _)| match e {
            async_imap::error::Error::No(text) => Failure::Refused(text),
            other => Failure::Other(other.to_string()),
        })?;
    let _ = session.logout().await;
    Ok(())
}

async fn smtp(login: &Login<'_>, security: Security) -> Result<(), Failure> {
    let tls = match security {
        Security::Tls => Some(TlsParameters::new(login.host.into()).map_err(|e| Failure::Other(e.to_string()))?),
        #[cfg(test)]
        Security::Plain => None,
    };
    let mut conn = AsyncSmtpConnection::connect_tokio1((login.host, login.port), None, &ClientId::default(), tls, None)
        .await
        .map_err(connect_failure)?;
    let credentials = Credentials::new(login.user.into(), login.password.into());
    conn.auth(&[Mechanism::Plain, Mechanism::Login], &credentials)
        .await
        .map_err(|e| {
            // 535: bad credentials. 534: Gmail's "Application-specific password required".
            match e.status().map(|code| code.to_string()).as_deref() {
                Some("535" | "534") => Failure::Refused(e.to_string()),
                _ => Failure::Other(e.to_string()),
            }
        })?;
    let _ = conn.quit().await;
    Ok(())
}

/// A failed TLS handshake arrives as an I/O error that wraps the rustls error, if there is one.
fn handshake_failure(io: &std::io::Error) -> Option<Failure> {
    let tls = io.get_ref()?.downcast_ref::<tokio_rustls::rustls::Error>()?;
    Some(match tls {
        tokio_rustls::rustls::Error::InvalidCertificate(_) => Failure::Untrusted,
        _ => Failure::Insecure,
    })
}

/// lettre reports both a refused connection and a failed TLS handshake as an I/O error.
fn connect_failure(e: lettre::transport::smtp::Error) -> Failure {
    let mut source = std::error::Error::source(&e);
    while let Some(err) = source {
        if let Some(io) = err.downcast_ref::<std::io::Error>() {
            return handshake_failure(io).unwrap_or(Failure::Unreachable);
        }
        source = err.source();
    }
    if e.is_tls() {
        Failure::Insecure
    } else {
        Failure::Other(e.to_string())
    }
}

#[cfg(test)]
pub(crate) mod fake {
    //! Scripted servers on localhost that answer one connection each.

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpListener;

    /// Writes `greeting`, then answers each line the client sends with `reply(line)`.
    /// Returns the port.
    pub async fn serve(greeting: &'static str, reply: impl Fn(&str) -> String + Send + 'static) -> u16 {
        let (listener, port) = bind().await;
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = BufReader::new(stream);
            stream.get_mut().write_all(greeting.as_bytes()).await.unwrap();
            let mut line = String::new();
            while stream.read_line(&mut line).await.unwrap_or(0) > 0 {
                let answer = reply(line.trim_end());
                if stream.get_mut().write_all(answer.as_bytes()).await.is_err() {
                    break;
                }
                line.clear();
            }
        });
        port
    }

    fn tag(line: &str) -> &str {
        line.split(' ').next().unwrap()
    }

    pub async fn imap(accept: bool) -> u16 {
        serve("* OK fake IMAP ready\r\n", move |line| match (accept, line.contains("LOGOUT")) {
            (false, _) => format!("{} NO [AUTHENTICATIONFAILED] Authentication failed.\r\n", tag(line)),
            (true, true) => format!("* BYE\r\n{} OK bye\r\n", tag(line)),
            (true, false) => format!("{} OK done\r\n", tag(line)),
        })
        .await
    }

    pub async fn imap_gmail_wants_app_password() -> u16 {
        serve("* OK Gimap ready\r\n", |line| {
            format!(
                "{} NO [ALERT] Application-specific password required: https://support.google.com/accounts/answer/185833 (Failure)\r\n",
                tag(line)
            )
        })
        .await
    }

    pub async fn smtp(accept: bool) -> u16 {
        smtp_answering_auth_with(if accept { "235 2.7.0 Accepted" } else { "535 5.7.8 Authentication failed" }).await
    }

    pub async fn smtp_answering_auth_with(auth_reply: &'static str) -> u16 {
        serve("220 fake ESMTP\r\n", move |line| {
            if line.starts_with("EHLO") {
                "250-fake\r\n250 AUTH PLAIN LOGIN\r\n".into()
            } else if line.starts_with("QUIT") {
                "221 bye\r\n".into()
            } else {
                format!("{auth_reply}\r\n")
            }
        })
        .await
    }

    async fn bind() -> (TcpListener, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        (listener, port)
    }

    /// Speaks TLS with a self-signed certificate that no client trusts.
    pub async fn untrusted_tls() -> u16 {
        use std::sync::Arc;
        use tokio_rustls::rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
        use tokio_rustls::rustls::ServerConfig;

        let cert = rcgen::generate_simple_self_signed(vec!["127.0.0.1".to_string()]).unwrap();
        let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cert.signing_key.serialize_der()));
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert.cert.der().clone()], key)
            .unwrap();
        let (listener, port) = bind().await;
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = tokio_rustls::TlsAcceptor::from(Arc::new(config)).accept(stream).await;
        });
        port
    }

    /// A port with nothing listening on it.
    pub async fn closed_port() -> u16 {
        bind().await.1
    }

    /// Accepts the connection and then says nothing.
    pub async fn silent() -> u16 {
        let (listener, port) = bind().await;
        tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            std::future::pending::<()>().await;
        });
        port
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECOND: Duration = Duration::from_secs(1);

    fn login(port: u16) -> Login<'static> {
        Login { host: "127.0.0.1", port, user: "ada@example.com", password: "hunter2" }
    }

    #[tokio::test]
    async fn imap_login_that_works_says_so() {
        let check = check_imap(&login(fake::imap(true).await), Security::Plain, SECOND).await;
        assert_eq!(check, LoginCheck { ok: true, message: "Signed in to 127.0.0.1.".into() });
    }

    #[tokio::test]
    async fn imap_login_that_is_refused_says_to_check_the_password() {
        let check = check_imap(&login(fake::imap(false).await), Security::Plain, SECOND).await;
        assert!(!check.ok);
        assert_eq!(
            check.message,
            "127.0.0.1 did not accept that address and password. Check both and try again."
        );
    }

    #[tokio::test]
    async fn gmail_asking_for_an_app_password_is_explained() {
        let port = fake::imap_gmail_wants_app_password().await;
        let check = check_imap(&login(port), Security::Plain, SECOND).await;
        assert!(!check.ok);
        assert!(check.message.starts_with("Gmail needs an app password here, not your normal password."));
    }

    #[tokio::test]
    async fn smtp_login_that_works_says_so() {
        let check = check_smtp(&login(fake::smtp(true).await), Security::Plain, SECOND).await;
        assert_eq!(check, LoginCheck { ok: true, message: "Signed in to 127.0.0.1.".into() });
    }

    #[tokio::test]
    async fn smtp_login_that_is_refused_says_to_check_the_password() {
        let check = check_smtp(&login(fake::smtp(false).await), Security::Plain, SECOND).await;
        assert!(!check.ok);
        assert!(check.message.starts_with("127.0.0.1 did not accept that address and password."));
    }

    #[tokio::test]
    async fn an_smtp_refusal_that_is_not_about_the_password_is_not_blamed_on_it() {
        let port = fake::smtp_answering_auth_with("530 5.7.0 Must issue a STARTTLS command first").await;
        let check = check_smtp(&login(port), Security::Plain, SECOND).await;
        assert!(!check.ok);
        assert!(check.message.starts_with("Something went wrong talking to 127.0.0.1:"), "{}", check.message);
        assert!(check.message.contains("STARTTLS"));
    }

    #[tokio::test]
    async fn a_server_that_cannot_be_reached_is_reported_for_both() {
        let port = fake::closed_port().await;
        for check in [
            check_imap(&login(port), Security::Plain, SECOND).await,
            check_smtp(&login(port), Security::Plain, SECOND).await,
        ] {
            assert!(!check.ok);
            assert_eq!(
                check.message,
                format!("Could not reach 127.0.0.1 on port {port}. Check the server name, the port and your internet connection.")
            );
        }
    }

    #[tokio::test]
    async fn a_server_that_does_not_speak_tls_is_reported_for_both() {
        let check = check_imap(&login(fake::imap(true).await), Security::Tls, SECOND).await;
        assert!(!check.ok);
        assert!(check.message.starts_with("Reached 127.0.0.1, but could not set up a secure connection."));
        let check = check_smtp(&login(fake::smtp(true).await), Security::Tls, SECOND).await;
        assert!(!check.ok);
        assert!(check.message.starts_with("Reached 127.0.0.1, but could not set up a secure connection."));
    }

    #[tokio::test]
    async fn an_untrusted_certificate_is_reported_as_such_for_both() {
        let expected = "The security certificate from 127.0.0.1 can't be trusted, so the password was not sent. \
                        Check the server name.";
        for check in [
            check_imap(&login(fake::untrusted_tls().await), Security::Tls, SECOND).await,
            check_smtp(&login(fake::untrusted_tls().await), Security::Tls, SECOND).await,
        ] {
            assert!(!check.ok);
            assert_eq!(check.message, expected);
        }
    }

    #[tokio::test]
    async fn a_server_that_never_answers_times_out_for_both() {
        let short = Duration::from_millis(200);
        for check in [
            check_imap(&login(fake::silent().await), Security::Plain, short).await,
            check_smtp(&login(fake::silent().await), Security::Plain, short).await,
        ] {
            assert!(!check.ok);
            assert_eq!(check.message, "127.0.0.1 did not answer in time. Try again in a moment.");
        }
    }

    #[tokio::test]
    async fn no_message_ever_contains_the_password() {
        let port = fake::imap_gmail_wants_app_password().await;
        let check = check_imap(&login(port), Security::Plain, SECOND).await;
        assert!(!check.message.contains("hunter2"));
    }
}
