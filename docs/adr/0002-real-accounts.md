# 2. What the real accounts support

Status: accepted in part, 16 October 2026. Issue #5. The probe is in `spikes/imap/examples/probe.rs`.

This records two of the four accounts: one personal Gmail account and one Namecheap account. The work Gmail account, the employer policy and any other accounts are not checked yet. They are listed at the end.

## Decision

The sync code can rely on these on both checked accounts:

- TLS on port 993
- IDLE
- CONDSTORE
- SPECIAL-USE flags on the standard folders
- `MOVE` and `UIDPLUS`

It cannot rely on these:

- QRESYNC. Gmail does not support it. So the engine needs a way to find deleted messages without it.
- Folder names. Both servers have folders whose names look special but carry no flag. The engine finds special folders by their SPECIAL-USE flag only.
- Server thread IDs. Only Gmail has them. Other accounts are threaded from message headers.

## How it was checked

The probe logs in over TLS with `async-imap`, `tokio-rustls` and the `webpki-roots` certificates. It only reads. It prints:

- the capabilities the server lists after login
- every folder with its flags
- on Gmail, the `X-GM-*` values of the last 10 INBOX messages

The owner ran it against each account. Folder names, label names, addresses and IDs from the runs are left out of this record, because the repository is public.

## Results

| | Gmail (personal) | Namecheap |
| --- | --- | --- |
| Host and port | `imap.gmail.com:993` | `mail.privateemail.com:993` |
| Login | App password only | Worked |
| IDLE | Yes | Yes |
| CONDSTORE | Yes | Yes |
| QRESYNC | **No** | Yes |
| SPECIAL-USE | Yes | Yes |
| `MOVE`, `UIDPLUS`, `COMPRESS=DEFLATE` | Yes | Yes |
| Gmail extensions (`X-GM-EXT-1`) | Yes | No |
| Largest message it accepts (`APPENDLIMIT`) | 34 MB | 80 MB |

The Namecheap capability list looks like Dovecot's. So the Dovecot container from ADR 0001 is a fair stand-in for it in tests.

### Gmail

- **Login.** The normal account password is refused with `[ALERT] Application-specific password required`. An app password works. Google only offers app passwords when 2-Step Verification is on.
- **`X-GM-MSGID`.** It came back on every message, and `async-imap` parsed it.
- **`X-GM-THRID`.** It came back on every message. A reply shared its thread ID with the message before it but had its own message ID. A thread's ID equals the message ID of its first message.
- **`X-GM-LABELS`.** A user label and the star came back on a message after the owner added them in Gmail. `\Inbox` never appears while INBOX is the open folder. So the engine adds it from the folder it fetched.
- **Special folders.** All Mail, Drafts, Sent, Spam, Starred and Trash carry the standard flags `\All`, `\Drafts`, `\Sent`, `\Junk`, `\Flagged` and `\Trash`. Important carries `\Important`, which is Gmail's own flag. A user label with the same name as a special folder exists but has no flag.
- **Folders that cannot be opened.** The `[Gmail]` parent is marked `\NoSelect`. Other parent folders can be too.

### Namecheap

- **Special folders.** Archive, Drafts, Sent, Spam and Trash carry the standard flags. Four more folders have special-sounding names but no flag. Other mail apps often create folders like these. They may still hold mail, so the engine syncs them as normal folders.
- **Threads.** There is no thread ID. The server offers `THREAD=REFERENCES`, but the engine threads from the `Message-ID`, `In-Reply-To` and `References` headers itself, so that it works the same way for every account that isn't Gmail.

### A parser detail found on the way

`imap-proto` 0.17 does not undo the escaping inside IMAP quoted strings. Gmail sends the star label as the quoted string `"\\Starred"`, which means `\Starred`. The parsed value still holds both backslashes: `\\Starred`. The cause is `quoted` in `imap-proto`'s `core.rs`. It uses nom's `escaped`, which returns the raw bytes. So this affects every quoted string, not just labels.

## Consequences

- **Deletions.** On Namecheap, the engine uses QRESYNC to learn which messages were deleted. On Gmail, it compares the server's UID list with the local one after a CONDSTORE fetch. Using the UID compare for both is simpler, but slower on big folders. The sync design picks one.
- **Special folders** are found by flag. The engine never guesses from a folder's name.
- **Folders marked `\NoSelect`** are shown in the tree but never opened.
- **Threads.** On Gmail, `X-GM-THRID` is the thread key. On other accounts, the key comes from headers.
- **Unescaping.** The engine undoes the escaping of quoted strings from `imap-proto`: `\\` becomes `\` and `\"` becomes `"`. Labels need this first. A test with Gmail's real label line covers it.
- **Gmail setup.** Adding a Gmail account needs an app password, not the normal one.
- **TLS.** The open TLS check from ADR 0001 is done. `async-imap` over `tokio-rustls` connects to both servers.

## Still open

- **The work Gmail account.** It is not tested yet. The question is whether the Workspace admin allows IMAP and app passwords. If not, v1 ships with three accounts.
- **The employer policy** on syncing work mail to a local app. The owner checks this.
- **Any other accounts.** Only one Namecheap account was probed. Others on the same host are expected to match, but they are not checked.
- **IDLE on a real account.** The probe only checks that IDLE is listed. How it behaves over a long session, and how it reconnects, is checked when the sync code is built.
