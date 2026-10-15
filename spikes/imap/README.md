# IMAP crate spike

Compares `async-imap` and `imap-next` against a Dovecot container. The result is recorded in
`docs/adr/0001-imap-crate.md`. This crate is throwaway: the engine does not depend on it.

```sh
docker compose -f spikes/imap/docker-compose.yml up -d   # Dovecot on 127.0.0.1:31143
cargo test -p imap-spike                                  # both crates, same scenarios
cargo run --release -p imap-spike --example idle_memory -- async-imap 20
```

Set `IMAP_SPIKE_ADDR` to point the tests at another server. Dovecot accepts any user name with
the password `secret`, and each name gets its own empty mailbox.

Dovecot has no Gmail extensions, so the `X-GM-*` tests run against a small fake server in
`tests/common/mod.rs`. The real check against a Gmail account is still manual.
