import { test, expect } from '@playwright/test';

test('actual WASM suggestions share all summoner profiles, exact u64 seeds, and deterministic replay', async ({ page }) => {
  await page.goto('/');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const generator = new MonsterGenerator();
    try {
      const profiles = ['default', 'necromancer', 'shaman', 'lunatic', 'boss', 'ocean'];
      const prompts = [];
      for (const promptProfile of profiles) {
        const a = await generator.suggestPrompt('18446744073709551615', 2, { promptProfile });
        const b = await generator.suggestPrompt(18446744073709551615n, 2, { promptProfile: promptProfile.toUpperCase() });
        prompts.push({ a, b });
      }
      // Validate the direct binding too: u8 would silently wrap 257 to 1.
      const api = await import('./pkg/infernal_diffusion.js');
      await api.default();
      const rejected = [0, 4, 257, 1.5, NaN].map(difficulty => {
        try { api.suggest_prompt(42n, difficulty, 'default'); return false; } catch { return true; }
      });
      return { prompts, rejected };
    } finally { generator.dispose(); }
  });
  for (const { a, b } of result.prompts) { expect(a.length).toBeGreaterThan(5); expect(a).toBe(b); }
  expect(result.rejected).toEqual([true, true, true, true, true]);
});

test('WASM random prompt controls leave V1/V2 independent and visibly reroll exact seeds', async ({ page }, testInfo) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
  await expect(page.locator('#profile')).toHaveValue('v1');
  await expect(page.locator('#prompt-profile')).toHaveValue('default');
  await page.locator('#profile').selectOption('v2');
  await page.locator('#prompt-profile').selectOption('ocean');
  await page.locator('#seed').fill('18446744073709551615');
  await page.locator('#random-prompt').click();
  await expect(page.locator('#suggestion-status')).toContainText('Prompt seed 18446744073709551615');
  await expect(page.locator('#prompt')).not.toHaveValue('stormtrooper with a laser blaster');
  await expect(page.locator('#profile')).toHaveValue('v2');
  await page.locator('#random-prompt').click();
  await expect(page.locator('#suggestion-status')).toContainText('Prompt seed 0');
  await expect(page.locator('#seed')).toHaveValue('0');
  await page.locator('#prompt').fill('my arbitrary edited monster');
  await expect(page.locator('#prompt')).toHaveValue('my arbitrary edited monster');
  await page.screenshot({ path: testInfo.outputPath('prompt-summoner-controls.png'), fullPage: true });
});

test('WASM pending suggestions cannot overwrite edits or a newer example', async ({ page }) => {
  await page.route('**/worker.js', async route => {
    const response = await route.fetch();
    const body = (await response.text()).replace('const result = api.suggest_prompt(',
      'await new Promise(resolve => setTimeout(resolve, 200));\n      const result = api.suggest_prompt(');
    await route.fulfill({ response, body });
  });
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
  await page.locator('#random-prompt').click();
  await page.locator('#prompt').fill('keep my handwritten monster');
  await page.waitForTimeout(350);
  await expect(page.locator('#prompt')).toHaveValue('keep my handwritten monster');
  await page.locator('#random-prompt').click();
  await page.getByRole('button', { name: 'Acid frog', exact: true }).click();
  await page.waitForTimeout(350);
  await expect(page.locator('#prompt')).toHaveValue('friendly frog that sprays acid');
  await expect(page.locator('#random-prompt')).toBeEnabled();
});
