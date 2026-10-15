import { describe, expect, it, vi } from 'vitest';
import { mockEngine } from './mockEngine';
import type { EngineEvent, ServerSettings } from './protocol';

const servers: ServerSettings = { imapHost: 'i', imapPort: 993, smtpHost: 's', smtpPort: 465 };

async function listening() {
  const engine = mockEngine();
  const events: EngineEvent[] = [];
  await engine.onEvent((e) => events.push(e));
  return { engine, events };
}

describe('mockEngine', () => {
  it('answers a ping with a pong, like the real engine', async () => {
    const engine = mockEngine();
    const handler = vi.fn();
    await engine.onEvent(handler);
    await engine.send({ type: 'ping', message: 'hi' });
    expect(handler).toHaveBeenCalledWith({ type: 'pong', message: 'hi', schemaVersion: 1 });
  });

  it('returns the events that answer a command', async () => {
    const answer = await mockEngine().send({ type: 'listAccounts' });
    expect(answer).toEqual([{ type: 'accounts', accounts: [] }]);
  });

  it('stops delivering events after unlisten', async () => {
    const engine = mockEngine();
    const handler = vi.fn();
    const off = await engine.onEvent(handler);
    off();
    await engine.send({ type: 'ping', message: 'hi' });
    expect(handler).not.toHaveBeenCalled();
  });

  it('suggests Gmail settings for a Gmail address and nothing for an unknown domain', async () => {
    const { engine, events } = await listening();
    await engine.send({ type: 'suggestSettings', email: 'ada@gmail.com' });
    await engine.send({ type: 'suggestSettings', email: 'ada@example.com' });
    expect(events[0]).toMatchObject({ type: 'settingsSuggested', suggestion: { name: 'Gmail' } });
    expect(events[1]).toEqual({ type: 'settingsSuggested', email: 'ada@example.com', suggestion: null });
  });

  it('adds an account with default settings when both logins work', async () => {
    const { engine, events } = await listening();
    await engine.send({ type: 'addAccount', email: 'ada@example.com', password: 'pw', settings: servers });
    expect(events[0]).toMatchObject({ type: 'loginsTested', imap: { ok: true }, smtp: { ok: true } });
    expect(events[2]).toMatchObject({ type: 'accounts', accounts: [{ id: 1 }] });
    expect(events[1]).toEqual({
      type: 'accountAdded',
      account: {
        id: 1,
        email: 'ada@example.com',
        provider: 'imap',
        server: servers,
        settings: { displayName: 'ada@example.com', color: 'account-1', signature: '', sendFrom: 'ada@example.com' },
      },
    });
  });

  it('reports a refused login and adds nothing', async () => {
    const { engine, events } = await listening();
    await engine.send({ type: 'addAccount', email: 'ada@example.com', password: 'wrong', settings: servers });
    await engine.send({ type: 'listAccounts' });
    expect(events[0]).toMatchObject({ type: 'loginsTested', imap: { ok: false }, smtp: { ok: false } });
    expect(events[1]).toEqual({ type: 'accounts', accounts: [] });
  });

  it('saves settings and answers with the account list', async () => {
    const { engine, events } = await listening();
    await engine.send({ type: 'addAccount', email: 'ada@example.com', password: 'pw', settings: servers });
    const settings = { displayName: 'Personal', color: 'account-3', signature: 'Ada', sendFrom: 'ada@example.com' };
    await engine.send({ type: 'saveAccountSettings', id: 1, settings });
    expect(events.at(-1)).toMatchObject({ type: 'accounts', accounts: [{ id: 1, settings }] });
  });
});
