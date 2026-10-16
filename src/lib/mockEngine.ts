import { ACCOUNT_COLORS, type Account, type Command, type EngineClient, type EngineEvent } from './protocol';

const GMAIL = {
  name: 'Gmail',
  provider: 'gmail' as const,
  settings: { imapHost: 'imap.gmail.com', imapPort: 993, smtpHost: 'smtp.gmail.com', smtpPort: 465 },
};

/** Stands in for the Rust engine in browser tests. The password "wrong" fails both logins. */
export function mockEngine(): EngineClient {
  const handlers = new Set<(event: EngineEvent) => void>();
  const accounts: Account[] = [];

  function answer(command: Command): EngineEvent[] {
    switch (command.type) {
      case 'ping':
        return [{ type: 'pong', message: command.message, schemaVersion: 1 }];
      case 'suggestSettings':
        return [
          {
            type: 'settingsSuggested',
            email: command.email,
            suggestion: command.email.trim().toLowerCase().endsWith('@gmail.com') ? GMAIL : null,
          },
        ];
      case 'addAccount': {
        const { email, settings } = command;
        const ok = command.password !== 'wrong';
        const check = (host: string) =>
          ok
            ? { ok, message: `Signed in to ${host}.` }
            : { ok, message: `${host} did not accept that address and password. Check both and try again.` };
        const tested: EngineEvent = {
          type: 'loginsTested',
          email,
          imap: check(settings.imapHost),
          smtp: check(settings.smtpHost),
        };
        if (!ok) return [tested];
        const account: Account = {
          id: accounts.length + 1,
          email,
          provider: settings.imapHost === 'imap.gmail.com' ? 'gmail' : 'imap',
          server: settings,
          settings: {
            displayName: email,
            color: ACCOUNT_COLORS[accounts.length % ACCOUNT_COLORS.length],
            signature: '',
            sendFrom: email,
          },
        };
        accounts.push(account);
        return [tested, { type: 'accountAdded', account }, { type: 'accounts', accounts: structuredClone(accounts) }];
      }
      case 'listAccounts':
        return [{ type: 'accounts', accounts: structuredClone(accounts) }];
      case 'saveAccountSettings': {
        const account = accounts.find((a) => a.id === command.id);
        if (!account) return [{ type: 'failed', message: 'That account no longer exists.' }];
        account.settings = { ...command.settings };
        return [{ type: 'accounts', accounts: structuredClone(accounts) }];
      }
    }
  }

  return {
    async send(command) {
      const events = answer(command);
      for (const event of events) handlers.forEach((h) => h(event));
      return events;
    },
    async onEvent(handler) {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  };
}
