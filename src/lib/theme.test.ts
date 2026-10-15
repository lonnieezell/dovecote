import { describe, expect, it } from 'vitest';
import { applyTheme, readTheme, saveTheme } from './theme';

function fakeStorage(initial: Record<string, string> = {}) {
  const data = { ...initial };
  return {
    data,
    getItem: (key: string) => data[key] ?? null,
    setItem: (key: string, value: string) => void (data[key] = value),
  };
}

describe('theme', () => {
  it('defaults to following the system', () => {
    expect(readTheme(fakeStorage())).toBe('system');
  });

  it('ignores a saved value it does not know', () => {
    expect(readTheme(fakeStorage({ 'dovecote.theme': 'sepia' }))).toBe('system');
  });

  it('remembers a saved choice', () => {
    const storage = fakeStorage();
    saveTheme('dark', storage);
    expect(readTheme(storage)).toBe('dark');
  });

  it('forces a theme on the root element', () => {
    const root = { dataset: {} as DOMStringMap };
    applyTheme('dark', root);
    expect(root.dataset.theme).toBe('dark');
  });

  it('goes back to the system by removing the override', () => {
    const root = { dataset: {} as DOMStringMap };
    applyTheme('light', root);
    applyTheme('system', root);
    expect(root.dataset.theme).toBeUndefined();
  });
});
