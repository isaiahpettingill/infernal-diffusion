import { test, expect } from '@playwright/test';

test('actual WASM keeps default V1 and explicitly generates deterministic V2', async ({ page }) => {
  await page.goto('/');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const generator = new MonsterGenerator();
    const digest = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).join(',');
    try {
      const legacy = await generator.generate('wolf', 42n);
      const first = await generator.generate('wolf', 42n, { profile: 'v2' });
      const replay = await generator.generate('wolf', 42n, { profile: 'v2' });
      const root = first.packages.at(-1);
      let invalid = '';
      try { await generator.generate('wolf', 42n, { profile: 'v3' }); }
      catch (error) { invalid = error.message; }
      return {
        legacyProfile: legacy.packages.at(-1).monster.v2_profile ?? null,
        version: root.monster.v2_profile.profile_version,
        hash: await digest(root.sprites.rgba),
        replayHash: await digest(replay.packages.at(-1).sprites.rgba),
        width: root.sprites.width,
        bytes: root.sprites.rgba.length,
        expectedBytes: root.sprites.width * root.sprites.height * 4,
        invalid,
      };
    } finally { generator.dispose(); }
  });
  expect(result.legacyProfile).toBeNull();
  expect(result.version).toBe(1);
  expect(result.hash).toEqual(result.replayHash);
  expect(result.width).toBeGreaterThan(0);
  expect(result.bytes).toBe(result.expectedBytes);
  expect(result.invalid).toContain('Profile must be v1 or v2');
});


test('actual WASM validates and generates structured hybrid recipes', async ({ page }) => {
  await page.goto('/');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const generator = new MonsterGenerator();
    const recipe = { schema_version: 1, upper_body: 'STORMTROOPER', lower_body: 'MERMAID',
      head: 'STORMTROOPER', lower_limb_count: 0, upper_arm_count: 2 };
    try {
      const generated = await generator.generateHybrid(recipe, 42n);
      const rejected = [];
      for (const invalid of [{ ...recipe, schema_version: 99 }, { ...recipe, unexpected: true }]) {
        try { await generator.generateHybrid(invalid, 42n); rejected.push(false); }
        catch { rejected.push(true); }
      }
      const root = generated.packages.at(-1);
      return { version: root.monster.v2_profile.profile_version,
        source: JSON.parse(root.monster.generation.prompt),
        bytes: root.sprites.rgba.length, rejected };
    } finally { generator.dispose(); }
  });
  expect(result.version).toBe(1);
  expect(result.source.lower_body).toBe('MERMAID');
  expect(result.bytes).toBeGreaterThan(0);
  expect(result.rejected).toEqual([true, true]);
});

test('playground defaults to V1 and generates V2 from its selector', async ({ page }, testInfo) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
  await expect(page.locator('#profile')).toHaveValue('v1');
  await page.locator('#profile').selectOption('v2');
  await page.getByRole('button', { name: 'Acid frog', exact: true }).click();
  await expect(page.locator('#prompt')).toHaveValue('friendly frog that sprays acid');
  await expect(page.locator('#profile')).toHaveValue('v2');
  await page.locator('#generate').click();
  await expect(page.locator('#status')).toContainText('Generated', { timeout: 240_000 });
  const metadata = JSON.parse(await page.locator('#results pre').last().textContent());
  expect(metadata.v2_profile.profile_version).toBe(1);
  await page.screenshot({ path: testInfo.outputPath('playground-v2.png') });
  expect(await page.locator('#results canvas').count()).toBeGreaterThanOrEqual(2);
  await page.locator('#profile').selectOption('v1');
  await page.locator('#generate').click();
  await expect(page.locator('#status')).toContainText('Generated', { timeout: 240_000 });
  const legacy = JSON.parse(await page.locator('#results pre').last().textContent());
  expect(legacy.v2_profile ?? null).toBeNull();
});
