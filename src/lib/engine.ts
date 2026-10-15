import type { EngineClient } from './protocol';
import { tauriEngine } from './tauriEngine';

export async function connectEngine(): Promise<EngineClient> {
  if (import.meta.env.VITE_MOCK_ENGINE === '1') {
    const { mockEngine } = await import('./mockEngine');
    return mockEngine();
  }
  const [{ invoke }, { listen }] = await Promise.all([
    import('@tauri-apps/api/core'),
    import('@tauri-apps/api/event'),
  ]);
  return tauriEngine(invoke, listen);
}
