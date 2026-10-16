export type ServerSettings = { imapHost: string; imapPort: number; smtpHost: string; smtpPort: number };

export type Suggestion = { name: string; provider: 'gmail' | 'imap'; settings: ServerSettings };

export type LoginCheck = { ok: boolean; message: string };

export type AccountSettings = { displayName: string; color: string; signature: string; sendFrom: string };

export type Account = {
  id: number;
  email: string;
  provider: 'gmail' | 'imap';
  server: ServerSettings;
  settings: AccountSettings;
};

export type Command =
  | { type: 'ping'; message: string }
  | { type: 'suggestSettings'; email: string }
  | { type: 'addAccount'; email: string; password: string; settings: ServerSettings }
  | { type: 'listAccounts' }
  | { type: 'saveAccountSettings'; id: number; settings: AccountSettings };

export type EngineEvent =
  | { type: 'pong'; message: string; schemaVersion: number }
  | { type: 'settingsSuggested'; email: string; suggestion: Suggestion | null }
  | { type: 'loginsTested'; email: string; imap: LoginCheck; smtp: LoginCheck }
  | { type: 'accountAdded'; account: Account }
  | { type: 'accounts'; accounts: Account[] }
  | { type: 'failed'; message: string };

export interface EngineClient {
  /** Resolves to the events that answer this command. Every event is also broadcast to onEvent. */
  send(command: Command): Promise<EngineEvent[]>;
  onEvent(handler: (event: EngineEvent) => void): Promise<() => void>;
}

/** The account colors in tokens.css, in the order new accounts take them. */
export const ACCOUNT_COLORS = [1, 2, 3, 4, 5, 6, 7, 8].map((n) => `account-${n}`);
