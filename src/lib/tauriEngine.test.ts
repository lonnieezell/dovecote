import { describe, expect, it, vi } from 'vitest';
import { tauriEngine } from './tauriEngine';
import type { EngineEvent } from './protocol';

describe('tauriEngine', () => {
  it('sends a command through the send_command Tauri command and returns its answer', async () => {
    const pong: EngineEvent = { type: 'pong', message: 'hi', schemaVersion: 1 };
    const invoke = vi.fn().mockResolvedValue([pong]);
    const engine = tauriEngine(invoke, vi.fn());
    expect(await engine.send({ type: 'ping', message: 'hi' })).toEqual([pong]);
    expect(invoke).toHaveBeenCalledWith('send_command', { command: { type: 'ping', message: 'hi' } });
  });

  it('hands the payload of engine-event Tauri events to the handler', async () => {
    let emit: (e: { payload: EngineEvent }) => void = () => {};
    const listen = vi.fn(async (_name: string, cb: typeof emit) => {
      emit = cb;
      return () => {};
    });
    const handler = vi.fn();
    await tauriEngine(vi.fn(), listen).onEvent(handler);
    expect(listen).toHaveBeenCalledWith('engine-event', expect.any(Function));
    emit({ payload: { type: 'pong', message: 'hi', schemaVersion: 1 } });
    expect(handler).toHaveBeenCalledWith({ type: 'pong', message: 'hi', schemaVersion: 1 });
  });

  it('returns the unlisten function so the caller can stop listening', async () => {
    const unlisten = vi.fn();
    const off = await tauriEngine(vi.fn(), async () => unlisten).onEvent(vi.fn());
    off();
    expect(unlisten).toHaveBeenCalled();
  });
});
