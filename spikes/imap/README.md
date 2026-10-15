# IMAP crate spike

Compares `async-imap` and `imap-next` against a Dovecot container. The result is recorded in
`docs/adr/0001-imap-crate.md`. This crate is throwaway: the engine does not depend on it.

```sh
docker compose -f spikes/imap/docker-compose.yml up -d --wait   # Dovecot on 127.0.0.1:31143
cargo test -p imap-spike                                  # both crates, same scenarios
cargo run --release -p imap-spike --example idle_memory -- async-imap 20
```

Set `IMAP_SPIKE_ADDR` to point the tests at another server. Dovecot accepts any user name with
the password `secret`, and each name gets its own empty mailbox.

Dovecot has no Gmail extensions, so the `X-GM-*` tests run against a small fake server in
`tests/common/mod.rs`.

## Probing a real account

The `probe` example checks a real account over TLS. It prints the server's capabilities, its
folders with their special-use flags, and on Gmail the `X-GM-*` values of the newest INBOX
message. It only reads. Credentials come from the environment, so they stay out of the code
and the shell history if you `read -s` them first.

```sh
read -s PROBE_PASSWORD && export PROBE_PASSWORD
PROBE_HOST=imap.gmail.com PROBE_USER=you@gmail.com cargo run -p imap-spike --example probe
```

`PROBE_PORT` defaults to 993. The output includes folder and label names, so check it before
you share it.
