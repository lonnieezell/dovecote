import { expect, test, type Page } from '@playwright/test';

const LIGHT_BG = 'rgb(251, 247, 240)';
const DARK_BG = 'rgb(36, 29, 24)';

const background = (page: Page) =>
  page.evaluate(() => getComputedStyle(document.body).backgroundColor);

test('follows the system theme', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await page.goto('/');
  expect(await background(page)).toBe(LIGHT_BG);

  await page.emulateMedia({ colorScheme: 'dark' });
  expect(await background(page)).toBe(DARK_BG);
});

test('a manual choice overrides the system and survives a reload', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.goto('/');
  await page.getByLabel('Theme').selectOption('light');
  expect(await background(page)).toBe(LIGHT_BG);

  await page.reload();
  expect(await background(page)).toBe(LIGHT_BG);

  await page.getByLabel('Theme').selectOption('system');
  expect(await background(page)).toBe(DARK_BG);
});

test('both font families load from the app itself', async ({ page, baseURL }) => {
  const outside: string[] = [];
  page.on('request', (request) => {
    if (request.url().startsWith('http') && new URL(request.url()).origin !== new URL(baseURL!).origin)
      outside.push(request.url());
  });
  await page.goto('/');
  const loaded = await page.evaluate(async () => {
    const faces = await Promise.all([
      document.fonts.load('1em "Newsreader Variable"'),
      document.fonts.load('1em "Source Sans 3 Variable"'),
    ]);
    return faces.every((f) => f.length > 0);
  });
  expect(loaded).toBe(true);
  expect(outside).toEqual([]);
});
