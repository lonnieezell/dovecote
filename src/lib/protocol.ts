export type Command = { type: 'ping'; message: string };

export type EngineEvent = { type: 'pong'; message: string; schemaVersion: number };

export interface EngineClient {
  send(command: Command): Promise<void>;
  onEvent(handler: (event: EngineEvent) => void): Promise<() => void>;
}
