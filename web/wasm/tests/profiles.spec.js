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
