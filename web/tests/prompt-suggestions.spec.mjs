import { test, expect } from '../wasm/node_modules/@playwright/test/index.mjs';

test('native suggestion endpoint preserves max-u64 and rejects invalid inputs', async ({ request }) => {
  for (const prompt_profile of ['default', 'necromancer', 'shaman', 'lunatic', 'boss', 'ocean']) {
    const data = { seed: '18446744073709551615', difficulty: 2, prompt_profile };
    const first = await request.post('/api/suggest', { data });
    const second = await request.post('/api/suggest', { data });
    expect(first.status()).toBe(200);
    const result = await first.json();
    expect(result.seed).toBe(data.seed);
    expect(result.prompt.length).toBeGreaterThan(5);
    expect(await second.json()).toEqual(result);
  }
  for (const invalid of [{ seed: '-1' }, { seed: '18446744073709551616' }, { difficulty: 257 }, { prompt_profile: 'v2' }]) {
    const response = await request.post('/api/suggest', { data: { seed: '42', difficulty: 2, ...invalid } });
    expect(response.status()).toBe(400);
  }
});

test('native prompt controls reroll seeds, preserve generation profile, and protect manual edits', async ({ page }, testInfo) => {
  await page.goto('/');
  await expect(page.locator('#profile')).toHaveValue('v1');
  await page.locator('#profile').selectOption('v2');
  await page.locator('#prompt-profile').selectOption('necromancer');
  await page.locator('#seed').fill('18446744073709551615');
  await page.locator('#random-prompt').click();
  await expect(page.locator('#suggestion-status')).toContainText('Prompt seed 18446744073709551615');
  await expect(page.locator('#profile')).toHaveValue('v2');
  await page.locator('#random-prompt').click();
  await expect(page.locator('#suggestion-status')).toContainText('Prompt seed 0');
  await expect(page.locator('#seed')).toHaveValue('0');
  await page.screenshot({ path: testInfo.outputPath('native-prompt-summoner-controls.png'), fullPage: true });
  await page.route('**/api/suggest', async route => {
    const response = await route.fetch();
    await new Promise(resolve => setTimeout(resolve, 250));
    await route.fulfill({ response });
  });
  await page.locator('#random-prompt').click();
  await page.locator('#prompt').fill('my custom creature with a sword');
  await page.waitForTimeout(400);
  await expect(page.locator('#prompt')).toHaveValue('my custom creature with a sword');
  await expect(page.locator('#random-prompt')).toBeEnabled();
});
