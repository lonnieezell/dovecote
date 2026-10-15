use std::time::Duration;

use hickory_resolver::{proto::rr::RData, TokioResolver};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    pub imap_host: String,
    pub imap_port: u16,
    pub smtp_host: String,
    pub smtp_port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub name: String,
    pub provider: &'static str,
    pub settings: ServerSettings,
}

const GMAIL_IMAP_HOST: &str = "imap.gmail.com";

fn gmail() -> Suggestion {
    Suggestion {
        name: "Gmail".into(),
        provider: "gmail",
        settings: ServerSettings {
            imap_host: GMAIL_IMAP_HOST.into(),
            imap_port: 993,
            smtp_host: "smtp.gmail.com".into(),
            smtp_port: 465,
        },
    }
}

fn namecheap() -> Suggestion {
    Suggestion {
        name: "Namecheap Private Email".into(),
        provider: "imap",
        settings: ServerSettings {
            imap_host: "mail.privateemail.com".into(),
            imap_port: 993,
            smtp_host: "mail.privateemail.com".into(),
            smtp_port: 465,
        },
    }
}

/// The part after the last `@`, lower-cased, or `None` if the address has no domain.
pub fn domain_of(email: &str) -> Option<String> {
    let (local, domain) = email.trim().rsplit_once('@')?;
    (!local.is_empty() && domain.contains('.')).then(|| domain.to_ascii_lowercase())
}

/// Settings for a domain everyone knows, without asking DNS.
pub fn known_domain(domain: &str) -> Option<Suggestion> {
    matches!(domain, "gmail.com" | "googlemail.com").then(gmail)
}

/// Settings for a custom domain, read from the hosts its MX records point at.
pub fn from_mx_hosts<S: AsRef<str>>(hosts: &[S]) -> Option<Suggestion> {
    let under = |host: &str, zone: &str| {
        let host = host.trim_end_matches('.').to_ascii_lowercase();
        host == zone || host.ends_with(&format!(".{zone}"))
    };
    hosts.iter().map(AsRef::as_ref).find_map(|host| {
        if under(host, "google.com") || under(host, "googlemail.com") {
            Some(gmail())
        } else if under(host, "privateemail.com") {
            Some(namecheap())
        } else {
            None
        }
    })
}

/// Settings for an address: from the domain if it is well known, otherwise from its MX records.
pub async fn suggest(email: &str) -> Option<Suggestion> {
    let domain = domain_of(email)?;
    if let Some(known) = known_domain(&domain) {
        return Some(known);
    }
    let lookup = async {
        let resolver = TokioResolver::builder_tokio().ok()?.build().ok()?;
        let answer = resolver.mx_lookup(domain.as_str()).await.ok()?;
        let mut records: Vec<_> = answer
            .answers()
            .iter()
            .filter_map(|record| match &record.data {
                RData::MX(mx) => Some((mx.preference, mx.exchange.to_string())),
                _ => None,
            })
            .collect();
        // Mail goes to the lowest preference first, so that host decides the provider.
        records.sort();
        Some(records.into_iter().map(|(_, host)| host).collect::<Vec<_>>())
    };
    let hosts = tokio::time::timeout(Duration::from_secs(5), lookup).await.ok()??;
    from_mx_hosts(&hosts)
}

/// Gmail gets its own sync path (server thread IDs, labels). Any other server is plain IMAP.
pub fn provider_for(settings: &ServerSettings) -> &'static str {
    if settings.imap_host.eq_ignore_ascii_case(GMAIL_IMAP_HOST) {
        "gmail"
    } else {
        "imap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_domain_from_an_address() {
        assert_eq!(domain_of(" Someone@Example.COM ").as_deref(), Some("example.com"));
        assert_eq!(domain_of("no-at-sign"), None);
        assert_eq!(domain_of("@example.com"), None);
        assert_eq!(domain_of("someone@localhost"), None);
    }

    #[test]
    fn gmail_addresses_get_gmail_settings_without_dns() {
        let s = known_domain("gmail.com").unwrap();
        assert_eq!(s.name, "Gmail");
        assert_eq!((s.settings.imap_host.as_str(), s.settings.imap_port), ("imap.gmail.com", 993));
        assert_eq!((s.settings.smtp_host.as_str(), s.settings.smtp_port), ("smtp.gmail.com", 465));
        assert_eq!(known_domain("googlemail.com"), Some(gmail()));
        assert_eq!(known_domain("example.com"), None);
    }

    #[test]
    fn a_custom_domain_hosted_by_google_gets_gmail_settings() {
        assert_eq!(from_mx_hosts(&["ASPMX.L.GOOGLE.COM."]), Some(gmail()));
        assert_eq!(from_mx_hosts(&["smtp.google.com"]), Some(gmail()));
    }

    #[test]
    fn a_custom_domain_hosted_by_namecheap_gets_namecheap_settings() {
        let s = from_mx_hosts(&["mx1.privateemail.com."]).unwrap();
        assert_eq!(s.name, "Namecheap Private Email");
        assert_eq!((s.settings.imap_host.as_str(), s.settings.imap_port), ("mail.privateemail.com", 993));
        assert_eq!((s.settings.smtp_host.as_str(), s.settings.smtp_port), ("mail.privateemail.com", 465));
    }

    #[test]
    fn an_unknown_host_or_a_lookalike_finds_nothing() {
        assert_eq!(from_mx_hosts(&["mx.example.com"]), None);
        assert_eq!(from_mx_hosts(&["mail.notgoogle.com"]), None);
        assert_eq!(from_mx_hosts::<&str>(&[]), None);
    }

    #[test]
    fn the_provider_is_gmail_only_for_gmails_imap_server() {
        for s in [gmail(), namecheap()] {
            assert_eq!(provider_for(&s.settings), s.provider);
        }
        assert_eq!(gmail().provider, "gmail");
    }
}
