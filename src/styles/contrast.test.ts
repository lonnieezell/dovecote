import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

const css = readFileSync(new URL('./tokens.css', import.meta.url), 'utf8');

type Mode = 'light' | 'dark';

function token(name: string, mode: Mode): string {
  const match = css.match(new RegExp(`--${name}:\\s*light-dark\\(\\s*(#[0-9a-fA-F]{6})\\s*,\\s*(#[0-9a-fA-F]{6})\\s*\\)`));
  if (!match) throw new Error(`token --${name} is missing or is not light-dark(#hex, #hex)`);
  return match[mode === 'light' ? 1 : 2];
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
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
  for (const color of ['category-gold', 'category-mauve']) {
    for (const surface of surfaces) {
      it(`dark: --${color} on --${surface} is at least 4.5:1`, () => {
        expect(contrast(token(color, 'dark'), token(surface, 'dark'))).toBeGreaterThanOrEqual(4.5);
      });
    }
  }
});

it('defines the other color tokens for light and dark', () => {
  for (const mode of modes) {
    for (const role of ['border', 'accent', 'category-gold', 'category-mauve']) {
      expect(token(role, mode)).toMatch(/^#/);
    }
  }
});
