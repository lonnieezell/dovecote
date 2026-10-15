# DoveCote

*A quiet home for your email. Only the birds you invite get to roost.*

DoveCote is a lightweight desktop email client built around two ideas:

- **You decide who gets in.** New senders wait at the gate until you approve them. Nothing lands in your inbox uninvited.

- **Every language is readable.** Translate any email, or any part of one, in place, without leaving the message.

It works with all your accounts in one place, and it's built with Tauri (Rust engine, Svelte UI) to stay small, fast, and easy on memory.

## Status

Early and in active development. Expect rough edges.


## Development

### Requirements

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 24 and [pnpm](https://pnpm.io) 11
- macOS: Xcode Command Line Tools (`xcode-select --install`)

### Run the dev build

```sh
pnpm install
pnpm tauri dev
```

This starts the Vite dev server on port 1420 and opens the app window. The UI reloads as you edit. Rust changes rebuild and restart the app.

The app stores its database in `~/Library/Application Support/dev.dovecote.app/mail.db`.

### Build a debug binary without a dev server

```sh
pnpm tauri build --debug --no-bundle
```

The binary is `target/debug/dovecote`.

### Tests

```sh
cargo test -p dovecote-engine   # engine
pnpm test                       # UI logic (Vitest)
pnpm exec playwright install webkit   # once
pnpm e2e                        # UI in WebKit with a mocked engine
pnpm check                      # type check
```
