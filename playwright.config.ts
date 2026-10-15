import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: 'e2e',
  use: { baseURL: 'http://localhost:1421' },
  projects: [{ name: 'webkit', use: { ...devices['Desktop Safari'] } }],
  webServer: {
    command: 'pnpm dev --port 1421',
    url: 'http://localhost:1421',
    env: { VITE_MOCK_ENGINE: '1' },
    reuseExistingServer: false,
  },
});
