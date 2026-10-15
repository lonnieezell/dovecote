import { expect, test } from '@playwright/test';

test('shows the three panes and round-trips a command through the engine', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('navigation', { name: 'Sidebar' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Message list' })).toBeVisible();
  await expect(page.getByRole('main', { name: 'Reading pane' })).toBeVisible();

  const ping = page.getByRole('button', { name: 'Ping engine' });
  await expect(ping).toBeEnabled();
  await ping.click();
  await expect(page.getByTestId('engine-reply')).toHaveText('Engine replied: hello (schema 1)');
});
