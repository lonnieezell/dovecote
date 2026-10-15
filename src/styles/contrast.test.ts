import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('./tokens.css', import.meta.url), 'utf8');

type Mode = 'light' | 'dark';

function token(name: string, mode: Mode): string {
  const pair = css.match(new RegExp(`--${name}:\\s*light-dark\\(\\s*(#[0-9a-fA-F]{6})\\s*,\\s*(#[0-9a-fA-F]{6})\\s*\\)`));
  if (pair) return pair[mode === 'light' ? 1 : 2];
  const plain = css.match(new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6})\\s*;`));
  if (plain) return plain[1];
  throw new Error(`token --${name} is missing or is not light-dark(#hex, #hex) or #hex`);
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function oklab(hex: string): number[] {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b);
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b);
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b);
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ];
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const textRoles = ['text', 'text-secondary', 'text-muted', 'accent-text', 'teal'];
const surfaces = ['bg', 'bg-sunken', 'bg-raised'];
const modes: Mode[] = ['light', 'dark'];

describe('token contrast', () => {
  for (const mode of modes) {
    for (const text of textRoles) {
      for (const surface of surfaces) {
        it(`${mode}: --${text} on --${surface} is at least 4.5:1`, () => {
          expect(contrast(token(text, mode), token(surface, mode))).toBeGreaterThanOrEqual(4.5);
        });
      }
    }
  }
});

describe('token pairs from the design', () => {
  const pairs = [
    ['on-inverse', 'text'],
    ['accent-soft-text', 'accent-soft-bg'],
  ];
  for (const mode of modes) {
    for (const [fg, bg] of pairs) {
      it(`${mode}: --${fg} on --${bg} is at least 4.5:1`, () => {
        expect(contrast(token(fg, mode), token(bg, mode))).toBeGreaterThanOrEqual(4.5);
      });
    }
  }

  // The design draws these as small text only in dark mode (outlined avatars).
  for (const color of ['category-gold', 'category-mauve', 'category-sage', 'category-neutral']) {
    for (const surface of surfaces) {
      it(`dark: --${color} on --${surface} is at least 4.5:1`, () => {
        expect(contrast(token(color, 'dark'), token(surface, 'dark'))).toBeGreaterThanOrEqual(4.5);
      });
    }
  }
});

it('defines the other color tokens for light and dark', () => {
  for (const mode of modes) {
    for (const role of ['border', 'accent', 'category-gold', 'category-mauve', 'category-sage-deep']) {
      expect(token(role, mode)).toMatch(/^#/);
    }
  }
});

describe('account colors', () => {
  const accounts = Array.from({ length: 8 }, (_, i) => `account-${i + 1}`);

  // 1 to 3 come from the design and are lighter than this, so they are left out.
  for (const account of accounts.slice(3)) {
    for (const mode of modes) {
      for (const surface of surfaces) {
        it(`${mode}: --${account} on --${surface} is at least 3:1`, () => {
          expect(contrast(token(account, mode), token(surface, mode))).toBeGreaterThanOrEqual(3);
        });
      }
    }
  }

  it('keeps every pair of accounts easy to tell apart', () => {
    for (const [i, a] of accounts.entries()) {
      for (const b of accounts.slice(i + 1)) {
        const [x, y] = [oklab(token(a, 'light')), oklab(token(b, 'light'))];
        expect(Math.hypot(...x.map((v, k) => v - y[k])), `${a} vs ${b}`).toBeGreaterThanOrEqual(0.1);
      }
    }
  });
});
