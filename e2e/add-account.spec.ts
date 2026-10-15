import { expect, test } from '@playwright/test';

test('adds a Gmail account, then saves its settings', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add account' }).click();

  await page.getByLabel('Email address').fill('ada@gmail.com');
  await page.getByRole('button', { name: 'Continue' }).click();
  await expect(page.getByText('Gmail settings found')).toBeVisible();

  const password = page.getByLabel('Password or app password');
  await password.fill('app-password');
  await page.getByRole('button', { name: 'Test and add' }).click();

  await expect(page.getByTestId('imap-check')).toHaveText('Reading mail (IMAP): Signed in to imap.gmail.com.');
  await expect(page.getByTestId('smtp-check')).toHaveText('Sending mail (SMTP): Signed in to smtp.gmail.com.');
  await expect(password).toHaveCount(0);

  await page.getByLabel('Name').fill('Personal');
  await page.getByLabel('Color 3').check();
  await page.getByLabel('Signature').fill('Ada');
  await page.getByLabel('Send from').fill('ada@lovelace.example');
  await page.getByRole('button', { name: 'Save settings' }).click();

  await expect(page.getByText('Saved.')).toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Sidebar' }).getByRole('button', { name: 'Personal' })).toBeVisible();
});

test('reports each login result in plain words and adds nothing when they fail', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add account' }).click();
  await page.getByLabel('Email address').fill('ada@gmail.com');
  await page.getByRole('button', { name: 'Continue' }).click();
  await page.getByLabel('Password or app password').fill('wrong');
  await page.getByRole('button', { name: 'Test and add' }).click();

  await expect(page.getByTestId('imap-check')).toContainText('imap.gmail.com did not accept that address and password');
  await expect(page.getByTestId('smtp-check')).toContainText('smtp.gmail.com did not accept that address and password');
  await expect(page.getByLabel('Name')).toHaveCount(0);
});

test('the same form adds a second account, and changing the address starts over', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add account' }).click();
  const email = page.getByLabel('Email address');
  const password = page.getByLabel('Password or app password');

  await email.fill('ada@gmail.com');
  await page.getByRole('button', { name: 'Continue' }).click();
  await password.fill('pw');
  await page.getByRole('button', { name: 'Test and add' }).click();
  await expect(page.getByLabel('Name')).toBeVisible();

  await email.fill('bob@example.com');
  await expect(password).toHaveCount(0);
  await page.getByRole('button', { name: 'Continue' }).click();
  await expect(page.getByLabel('Name')).toHaveCount(0);
  await expect(page.getByTestId('imap-check')).toHaveCount(0);
  await expect(page.getByLabel('IMAP server')).toHaveValue('');
});

test('an unknown domain falls back to entering the servers by hand', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Add account' }).click();
  await page.getByLabel('Email address').fill('ada@example.com');
  await page.getByRole('button', { name: 'Continue' }).click();

  await expect(page.getByText('Enter the server settings for this account.')).toBeVisible();
  await page.getByLabel('IMAP server').fill('mail.example.com');
  await page.getByLabel('SMTP server').fill('mail.example.com');
  await page.getByLabel('Password or app password').fill('pw');
  await page.getByRole('button', { name: 'Test and add' }).click();

  await expect(page.getByTestId('imap-check')).toHaveText('Reading mail (IMAP): Signed in to mail.example.com.');
  await expect(page.getByLabel('Name')).toHaveValue('ada@example.com');
});
