# 1. Use async-imap for IMAP

Status: accepted, 16 October 2026. Spike: issue #4, code in `spikes/imap`.

## Decision

The engine uses `async-imap` 0.12 with its `runtime-tokio` feature.

`imap-next` is the second choice. It is not needed unless `async-imap` becomes a problem.

If neither crate can serve all four accounts, the fallback is Electron with the Node mail libraries. Nothing in the spike points that way. The checks that remain open are listed at the end.

## What was tried

A Dovecot 2.4.5 container stands in for the Namecheap accounts. It runs with `docker compose` locally and as the `imap-spike` job in CI. Both crates ran the same scenarios against it. The results below come from `cargo test -p imap-spike`.

| Scenario | async-imap | imap-next |
| --- | --- | --- |
| Connect, log in, fetch an envelope | Works | Works |
| IDLE wakes when mail arrives | Works | Works |
| CONDSTORE: fetch only changes since a mod-sequence | Works | Works |
| Gmail `X-GM-MSGID` and `X-GM-LABELS` in a fetch | Works, with accessors | Does not work (see below) |
| Gmail `X-GM-THRID` in a fetch | Works through the low-level path | Does not work (see below) |

The Gmail rows ran against a small fake server, because Dovecot has no Gmail extensions. The fake server sends the same fetch line Gmail documents.

### async-imap

- `Session::uid_fetch` takes the attribute list as a plain string. `UID FETCH 1 (X-GM-MSGID X-GM-THRID X-GM-LABELS)` goes out as written.
- The parser reads all three Gmail attributes. `Fetch` has accessors for the message ID and the labels only.
- For the thread ID, send the command with `run_command` and read answers with `read_response`. The parsed `AttributeValue::GmailThrId` is there. This needs a direct dependency on `imap-proto`.
- CONDSTORE needs no special support. `select_condstore` returns `highest_modseq`, and `(CHANGEDSINCE n)` goes in the fetch string.
- A fetch stream can include flag updates the server pushed on its own. Those have no UID. Code that reads a fetch stream must skip them or handle them.

### imap-next

- It is sans-IO and fully typed, which is a clean design. But the typed commands have no way to name a Gmail attribute.
- The one escape is the `expose_stream` feature, which the crate describes as "for debug purposes (or experiments)". Writing the command onto the socket by hand does send it.
- The parser then rejects Gmail's answer. The client returns `MalformedMessage` and hands back the bytes it threw away. The values are only in those bytes, so the engine would have to parse them itself.
- The connection recovers after that error. The tagged OK arrives next and later commands work.
- `imap-codec` 2.0.0-alpha.9, which `imap-next` 0.3.4 uses, did not build on its own. It needs `nom` 7 with its `alloc` feature, and nothing turns that on. The spike adds a `nom` line to work around it. The crate is also still on alpha versions.

The spec says Gmail's `X-GM-THRID` and `X-GM-MSGID` are how Gmail threads and de-duplicates mail. `imap-next` would need a fork or a patch to the codec to support that. `async-imap` supports it today.

## Idle memory

One account connected and sitting in IDLE, on macOS, in a release build. Resident memory (RSS) was read with `ps` after 15 seconds. Each row is three runs.

| Mode | RSS after 15 s |
| --- | --- |
| No connection (baseline) | 1.6 to 1.8 MB |
| async-imap, one IDLE connection | 2.0 MB |
| imap-next, one IDLE connection | 1.7 to 2.2 MB |

The cost of one idle account is 0.2 to 0.4 MB for either crate. That is smaller than the run-to-run noise, so these numbers cannot rank the two crates, and memory played no part in the choice. They only show that an idle connection is cheap. Four accounts with two connections each would add a few megabytes. The target in the spec is 250 MB for all processes, and the IMAP crate is not what will use it.

Two limits on this number:

- The connection was plain text, so it did not include TLS. Gmail and Namecheap both need TLS, which adds some memory for the session.
- The measurement does not include parsing real messages or the SQLite database. M1 measures the whole app again.

## Consequences

- The engine's sync code is written against `async-imap`. It sends Gmail attributes as raw fetch strings.
- The engine reads the thread ID through `run_command` and `read_response`. If that gets awkward, wrapping it in one small function keeps the rest of the engine clean.
- The `imap-spike` crate and its CI job stay until the open checks below are done. Then they can be deleted.

## Still open

These need real accounts. The spike could not check them.

- Gmail's own behavior: `X-GM-THRID`, `X-GM-MSGID`, labels, and IDLE reconnects on a real account. The spec asks for a manual check.
- The work Gmail account: whether the Workspace admin allows IMAP and app passwords.
- The Namecheap servers: which hosts and ports, and whether they support IDLE, CONDSTORE and SPECIAL-USE.
- TLS: connecting with `async-imap` over `tokio-rustls` to a real server.
