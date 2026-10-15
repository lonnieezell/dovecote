export const THEMES = ['system', 'light', 'dark'] as const;
export type Theme = (typeof THEMES)[number];

const KEY = 'dovecote.theme';

export function readTheme(storage: Pick<Storage, 'getItem'>): Theme {
  const saved = storage.getItem(KEY);
  return THEMES.find((theme) => theme === saved) ?? 'system';
}

export function applyTheme(theme: Theme, root: { dataset: DOMStringMap }) {
  if (theme === 'system') delete root.dataset.theme;
  else root.dataset.theme = theme;
}

export function saveTheme(theme: Theme, storage: Pick<Storage, 'setItem'>) {
  storage.setItem(KEY, theme);
}
