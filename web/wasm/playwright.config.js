import { defineConfig } from '@playwright/test';
export default defineConfig({
  testDir: './tests',
  timeout: 240_000,
  expect: { timeout: 15_000 },
  fullyParallel: false,
  workers: 1,
  use: { baseURL: 'http://127.0.0.1:8080', browserName: 'chromium', headless: true, launchOptions: { executablePath: process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE || undefined } },
  webServer: {
    command: 'python3 server.py',
    url: 'http://127.0.0.1:8080',
    reuseExistingServer: !process.env.CI,
  },
});
