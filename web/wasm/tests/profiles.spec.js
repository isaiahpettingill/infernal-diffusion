import { test, expect } from '@playwright/test';

test('actual Candle BERT preserves exact u64 seeds, world sizing, metadata and every V1/V2 RGBA atlas on replay', async ({ page }) => {
  await page.goto('/tests/browser-check.html');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('../client.js');
    const generator = new MonsterGenerator();
    const digest = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)))
      .map(byte => byte.toString(16).padStart(2, '0')).join('');
    const summarize = async output => Promise.all(output.packages.map(async pkg => ({
      path: pkg.path,
      metadata: JSON.parse(JSON.stringify(pkg.monster, (_, value) => typeof value === 'bigint' ? value.toString() : value)),
      atlases: await Promise.all([pkg.sprites, pkg.emission, pkg.projectiles].map(async atlas => atlas && ({
        width: atlas.width, height: atlas.height, bytes: atlas.rgba.length,
        typed: atlas.rgba instanceof Uint8Array, hash: await digest(atlas.rgba),
      }))),
    })));
    try {
      const seed = '18446744073709551615';
      const legacy = await summarize(await generator.generate('small wolf', seed));
      const legacyReplay = await summarize(await generator.generate('small wolf', BigInt(seed), { profile: 'v1' }));
      const first = await summarize(await generator.generate('small wolf', seed, { profile: 'v2' }));
      const replay = await summarize(await generator.generate('small wolf', BigInt(seed), { profile: 'v2' }));
      let invalid = '';
      try { await generator.generate('wolf', 42n, { profile: 'v3' }); }
      catch (error) { invalid = error.message; }
      return { legacy, legacyReplay, first, replay, invalid };
    } finally { generator.dispose(); }
  });
  expect(result.legacy).toEqual(result.legacyReplay);
  expect(result.first).toEqual(result.replay);
  expect(result.legacy.at(-1).metadata.v2_profile ?? null).toBeNull();
  expect(result.first.at(-1).metadata.v2_profile.profile_version).toBe(1);
  for (const packages of [result.legacy, result.first]) {
    expect(packages.length).toBeGreaterThan(0);
    const root = packages.at(-1).metadata;
    expect(root.generation.seed).toBe('18446744073709551615');
    expect(root.generation.parser).toBe('candle-bert-prototypes-v2');
    for (const key of ['world_scale', 'pixels_per_unit', 'reference_height_px', 'target_height_px', 'reference_player_height_px']) {
      expect(Number.isFinite(root.size[key]), key).toBe(true);
      expect(root.size[key], key).toBeGreaterThan(0);
    }
    for (const pkg of packages) for (const atlas of pkg.atlases.filter(Boolean)) {
      expect(atlas.typed).toBe(true);
      expect(atlas.width).toBeGreaterThan(0);
      expect(atlas.height).toBeGreaterThan(0);
      expect(atlas.bytes).toBe(atlas.width * atlas.height * 4);
      expect(atlas.hash).toMatch(/^[a-f0-9]{64}$/);
    }
  }
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

// Exercise the real WebAssembly parser, anatomy, baker and package exporter.
test('actual WASM preserves explicit head and arm counts through V2 rendering', async ({ page }) => {
  await page.goto('/');
  const results = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const generator = new MonsterGenerator();
    try {
      const cases = [
        ['three-headed dinosaur with eight arms', 3, 8],
        ['3-headed t rex with 8 arms', 3, 8],
        ['skeleton mermaid with eight arms', 1, 8],
      ];
      const results = [];
      for (const [prompt, heads, arms] of cases) {
        const result = await generator.generate(prompt, 42n, { profile: 'v2' });
        const root = result.packages.at(-1);
        results.push({ prompt, heads, arms,
          actualHeads: root.monster.colliders.filter(c => /^head_\d+$/.test(c.node_id)).length,
          actualArms: root.monster.colliders.filter(c => /^hand_\d+$/.test(c.node_id)).length,
          rendered: root.sprites.rgba.some((value, i) => i % 4 === 3 && value > 0),
          animated: root.monster.animations.some(a => a.semantic_state === 'ATTACK'),
        });
      }
      return results;
    } finally { generator.dispose(); }
  });
  for (const result of results) {
    expect(result.actualHeads, result.prompt).toBe(result.heads);
    expect(result.actualArms, result.prompt).toBe(result.arms);
    expect(result.rendered, result.prompt).toBe(true);
    expect(result.animated, result.prompt).toBe(true);
  }
});
