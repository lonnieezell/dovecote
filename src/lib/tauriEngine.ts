import type { Command, EngineClient, EngineEvent } from './protocol';

type Invoke = (name: string, args: { command: Command }) => Promise<EngineEvent[]>;
type Listen = (
  name: string,
  handler: (e: { payload: EngineEvent }) => void,
) => Promise<() => void>;

export function tauriEngine(invoke: Invoke, listen: Listen): EngineClient {
  return {
    send(command) {
      return invoke('send_command', { command });
    },
    onEvent(handler) {
      return listen('engine-event', (e) => handler(e.payload));
    },
  };
}
