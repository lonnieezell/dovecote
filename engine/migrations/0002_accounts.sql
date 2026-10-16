-- Passwords are not stored here. They live in the macOS Keychain, one item per account.
CREATE TABLE accounts (
  id           INTEGER PRIMARY KEY,
  email        TEXT NOT NULL UNIQUE COLLATE NOCASE,
  display_name TEXT NOT NULL,
  color        TEXT NOT NULL,
  provider     TEXT NOT NULL CHECK (provider IN ('gmail', 'imap')),
  imap_host    TEXT NOT NULL,
  imap_port    INTEGER NOT NULL,
  smtp_host    TEXT NOT NULL,
  smtp_port    INTEGER NOT NULL,
  send_from    TEXT NOT NULL,
  signature    TEXT NOT NULL DEFAULT ''
);
