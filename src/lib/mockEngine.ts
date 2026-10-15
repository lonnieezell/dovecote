import type { EngineClient, EngineEvent } from './protocol';

/** Stands in for the Rust engine in browser tests. */
export function mockEngine(): EngineClient {
  const handlers = new Set<(event: EngineEvent) => void>();
  return {
    async send(command) {
      const event: EngineEvent = { type: 'pong', message: command.message, schemaVersion: 1 };
      handlers.forEach((h) => h(event));
    },
    async onEvent(handler) {
      handlers.add(handler);
      return () => handlers.delete(handler);
    },
  };
}
