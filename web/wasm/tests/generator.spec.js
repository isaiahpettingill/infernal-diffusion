import { test, expect } from '@playwright/test';

test('actual WASM worker generates deterministic RGBA and complete stormtrooper metadata without blocking UI', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
  expect(await page.evaluate(() => crossOriginIsolated && typeof SharedArrayBuffer !== 'undefined')).toBe(true);
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const g = new MonsterGenerator();
    let ticks = 0;
    const timer = setInterval(() => ticks++, 10);
    const digest = async (bytes) => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))).map(x => x.toString(16).padStart(2, '0')).join('');
    try {
      const a = await g.generate('stormtrooper with a laser blaster', 42n);
      const generationTicks = ticks;
      const b = await g.generate('stormtrooper with a laser blaster', '42');
      const compact = async (result) => Promise.all(result.packages.map(async (pkg) => ({
        path: pkg.path,
        monster: JSON.stringify(pkg.monster, (_, v) => typeof v === 'bigint' ? v.toString() : v),
        atlases: await Promise.all([pkg.sprites, pkg.emission, pkg.projectiles].map(async (atlas) => atlas && ({
          width: atlas.width, height: atlas.height, bytes: atlas.rgba.length,
          typed: atlas.rgba instanceof Uint8Array, hash: await digest(atlas.rgba),
        }))),
      })));
      return { a: await compact(a), b: await compact(b), generationTicks, monster: JSON.parse(JSON.stringify(a.packages[0].monster, (_, v) => typeof v === 'bigint' ? v.toString() : v)) };
    } finally { clearInterval(timer); g.dispose(); }
  });
  expect(result.generationTicks).toBeGreaterThan(0);
  expect(result.a).toEqual(result.b);
  expect(result.a.length).toBeGreaterThan(0);
  for (const pkg of result.a) for (const atlas of pkg.atlases.filter(Boolean)) {
    expect(atlas.typed).toBe(true);
    expect(atlas.width).toBeGreaterThan(0);
    expect(atlas.bytes).toBe(atlas.width * atlas.height * 4);
  }
  expect(result.monster.generation.prompt).toContain('stormtrooper');
  for (const key of ['physics', 'gameplay', 'size', 'animations', 'attacks', 'movement_modes', 'colliders', 'projectiles']) expect(result.monster).toHaveProperty(key);
  expect(JSON.stringify(result.monster)).toContain('RANDOM_DIRECTION');
  expect(result.monster.attacks.length).toBeGreaterThan(0);
  expect(result.monster.projectiles.length).toBeGreaterThan(0);
});

test('validation, cancellation of all queued requests, restart, and disposal', async ({ page }) => {
  await page.goto('/');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('./client.js');
    const g = new MonsterGenerator();
    await g.ready();
    const errorOf = async (promise) => { try { await promise; return null; } catch (e) { return { name: e.name, message: e.message, code: e.code }; } };
    const invalid = await Promise.all([errorOf(g.generate('', 1)), errorOf(g.generate('slime', -1)), errorOf(g.generate('slime', 2 ** 60)), errorOf(g.generate('slime', '18446744073709551616'))]);
    const controller = new AbortController();
    const a = errorOf(g.generate('fire dragon', 7n, { signal: controller.signal }));
    const b = errorOf(g.generate('ice slime', 8n));
    await new Promise(resolve => setTimeout(resolve, 25));
    controller.abort();
    const cancelled = await Promise.all([a, b]);
    const next = await g.generate('small ice slime', 9n);
    g.dispose();
    return { invalid, cancelled, prompt: next.packages[0].monster.generation.prompt, disposed: await errorOf(g.ready()) };
  });
  expect(result.invalid.every(Boolean)).toBe(true);
  expect(result.cancelled.map(e => e?.name)).toEqual(['AbortError', 'AbortError']);
  expect(result.prompt).toBe('small ice slime');
  expect(result.disposed.message).toContain('disposed');
});

test('worker reports real instantiation errors when WASM is unavailable', async ({ page }) => {
  let attemptedBinary = false;
  await page.route('**/pkg/*.wasm', route => {
    attemptedBinary = true;
    return route.fulfill({ status: 200, contentType: 'application/wasm', body: 'not a wasm binary' });
  });
  await page.goto('/');
  await expect(page.locator('#status')).toContainText('Unable to load the generator');
  await expect(page.locator('#generate')).toBeDisabled();
  expect(attemptedBinary).toBe(true);
});

test('demo renders generated sprite atlases and exposes metadata', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
  await page.locator('#prompt').fill('small green slime');
  await page.locator('#generate').click();
  await expect(page.locator('#status')).toContainText('Generated', { timeout: 240_000 });
  expect(await page.locator('#results canvas').count()).toBeGreaterThanOrEqual(2);
  await expect(page.locator('#results pre').first()).toContainText('small green slime');
  expect(Number(await page.locator('#heartbeat').textContent())).toBeGreaterThan(0);
});

test('worker correlates protocol errors and remains usable', async ({ page }) => {
  await page.goto('/');
  const replies = await page.evaluate(async () => {
    const worker = new Worker(new URL('./worker.js', location.href), { type: 'module' });
    try {
      return await new Promise((resolve, reject) => {
        const results = [];
        worker.onerror = (event) => reject(new Error(event.message));
        worker.onmessage = ({ data }) => {
          results.push(data);
          if (results.length === 3) resolve(results.sort((a, b) => a.id - b.id));
        };
        worker.postMessage({ id: 1, type: 'unknown' });
        worker.postMessage({ id: 2, type: 'generate', prompt: '', seed: 1n });
        worker.postMessage({ id: 3, type: 'ready' });
      });
    } finally { worker.terminate(); }
  });
  expect(replies[0]).toMatchObject({ id: 1, type: 'error', error: { code: 'INVALID_REQUEST' } });
  expect(replies[1]).toMatchObject({ id: 2, type: 'error', error: { code: 'GENERATION_FAILED' } });
  expect(replies[2]).toMatchObject({ id: 3, type: 'ready' });
  expect(replies[2].threadCount).toBeGreaterThanOrEqual(2);
  expect(replies[2].threadCount).toBeLessThanOrEqual(4);
});

test('missing isolation headers produces an actionable threading error', async ({ page }) => {
  await page.route('**/*', async route => {
    const response = await route.fetch();
    const headers = { ...response.headers() };
    delete headers['cross-origin-opener-policy'];
    delete headers['cross-origin-embedder-policy'];
    await route.fulfill({ response, headers });
  });
  await page.goto('/');
  expect(await page.evaluate(() => crossOriginIsolated)).toBe(false);
  await expect(page.locator('#status')).toContainText('cross-origin isolation');
  await expect(page.locator('#generate')).toBeDisabled();
});
