import { describe, expect, it, vi } from 'vitest';
import { mockEngine } from './mockEngine';

describe('mockEngine', () => {
  it('answers a ping with a pong, like the real engine', async () => {
    const engine = mockEngine();
    const handler = vi.fn();
    await engine.onEvent(handler);
    await engine.send({ type: 'ping', message: 'hi' });
    expect(handler).toHaveBeenCalledWith({ type: 'pong', message: 'hi', schemaVersion: 1 });
  });

  it('stops delivering events after unlisten', async () => {
    const engine = mockEngine();
    const handler = vi.fn();
    const off = await engine.onEvent(handler);
    off();
    await engine.send({ type: 'ping', message: 'hi' });
    expect(handler).not.toHaveBeenCalled();
  });
});
