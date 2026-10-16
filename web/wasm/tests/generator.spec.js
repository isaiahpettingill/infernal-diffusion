import { test, expect } from '@playwright/test';

test('actual WASM worker generates deterministic RGBA and complete stormtrooper metadata without blocking UI', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText('Ready');
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
  expect(result.monster.generation.parser).toBe('candle-bert-prototypes-v2');
  expect(result.monster.generation.parser_confidence).toBeGreaterThan(0);
  expect(result.monster.generation.confidences.length).toBeGreaterThan(0);
  expect(Array.isArray(result.monster.generation.token_evidence)).toBe(true);
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
  expect(replies[2]).toMatchObject({
    model: 'google/bert_uncased_L-2_H-128_A-2',
    parser: 'candle-bert-prototypes-v2', bundledModel: true,
  });
  if (replies[2].threaded) {
    expect(replies[2].threadCount).toBeGreaterThanOrEqual(2);
    expect(replies[2].threadCount).toBeLessThanOrEqual(4);
  } else expect(replies[2].threadCount).toBe(1);
});

test('portable browser loads bundled BERT and generates offline without isolation or shared memory', async ({ page, context, baseURL }) => {
  test.skip(process.env.WASM_THREADED === '1', 'The optional threaded build intentionally requires isolation');
  const seen = [], blocked = [];
  const origin = new URL(baseURL).origin;
  context.on('request', request => seen.push(request.url()));
  // Enforce local-only loading. Removing response headers also covers worker and
  // WASM responses, instead of merely overriding the window's isolation flag.
  await context.route('**/*', async route => {
    if (new URL(route.request().url()).origin !== origin) {
      blocked.push(route.request().url());
      await route.abort('blockedbyclient');
      return;
    }
    const response = await route.fetch();
    const headers = { ...response.headers() };
    delete headers['cross-origin-opener-policy'];
    delete headers['cross-origin-embedder-policy'];
    await route.fulfill({ response, headers });
  });
  await page.goto('/tests/browser-check.html');
  expect(await page.evaluate(() => crossOriginIsolated)).toBe(false);
  expect(await page.evaluate(() => typeof SharedArrayBuffer)).toBe('undefined');
  const readiness = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('../client.js');
    window.offlineGenerator = new MonsterGenerator();
    const progress = [];
    const first = await window.offlineGenerator.ready({ onProgress: update => progress.push(update) });
    const second = await window.offlineGenerator.ready();
    return { first, second, progress };
  });
  expect(readiness.first).toMatchObject({
    threadCount: 1, model: 'google/bert_uncased_L-2_H-128_A-2',
    parser: 'candle-bert-prototypes-v2', bundledModel: true, threaded: false,
  });
  expect(readiness.second).toEqual(readiness.first);
  expect(readiness.progress.map(update => update.phase)).toEqual(expect.arrayContaining(['loading-wasm', 'initializing-model']));
  expect(seen.some(url => url.endsWith('.wasm'))).toBe(true);
  expect(seen.filter(url => /\/(models|api)\/|\.(safetensors|onnx)(\?|$)|tokenizer\.json|vocab\.txt/.test(url))).toEqual([]);
  const requestsAfterReady = seen.length;
  await context.setOffline(true);
  try {
    const result = await page.evaluate(async () => {
      const progress = [];
      const output = await window.offlineGenerator.generate('small green slime', '18446744073709551615', {
        profile: 'v2', onProgress: update => progress.push(update),
      });
      const root = output.packages.at(-1);
      window.offlineGenerator.dispose();
      return {
        parser: root.monster.generation.parser,
        seed: String(root.monster.generation.seed),
        width: root.sprites.width, height: root.sprites.height, bytes: root.sprites.rgba.length,
        progress,
      };
    });
    expect(result.parser).toBe('candle-bert-prototypes-v2');
    expect(result.seed).toBe('18446744073709551615');
    expect(result.width).toBeGreaterThan(0);
    expect(result.bytes).toBe(result.width * result.height * 4);
    expect(result.progress.map(update => update.phase)).toEqual(expect.arrayContaining([
      'parsing', 'building', 'rendering', 'packaging', 'complete',
    ]));
    expect(result.progress.at(-1).phase).toBe('complete');
    expect(result.progress.some(update => update.phase === 'rendering' && update.total > 0)).toBe(true);
    expect(seen.length).toBe(requestsAfterReady);
    expect(blocked).toEqual([]);
    expect(seen.every(url => new URL(url).origin === origin)).toBe(true);
  } finally { await context.setOffline(false); }
});

test('model-loading cancellation rejects the queue and reinitializes a fresh worker', async ({ page }) => {
  await page.goto('/tests/browser-check.html');
  const result = await page.evaluate(async () => {
    const { MonsterGenerator } = await import('../client.js');
    const g = new MonsterGenerator();
    const controller = new AbortController();
    const caught = promise => promise.then(() => 'unexpected success', error => error.name);
    let cancelledPhase;
    try {
      const first = caught(g.ready({ signal: controller.signal, onProgress: update => {
        if (update.phase === 'initializing-model') {
          cancelledPhase = update.phase;
          controller.abort();
        }
      } }));
      const queued = caught(g.generate('slime', 42n));
      const rejected = await Promise.all([first, queued]);
      const restarted = await g.ready();
      const output = await g.generate('small ice slime', 9n);
      return { rejected, cancelledPhase, restarted, parser: output.packages.at(-1).monster.generation.parser };
    } finally { g.dispose(); }
  });
  expect(result.cancelledPhase).toBe('initializing-model');
  expect(result.rejected).toEqual(['AbortError', 'AbortError']);
  expect(result.restarted.parser).toBe('candle-bert-prototypes-v2');
  expect(result.parser).toBe('candle-bert-prototypes-v2');
});

// Optional threaded distribution retains its explicit deployment requirements.
test('threaded build reports missing isolation as an actionable error', async ({ page, context }) => {
  test.skip(process.env.WASM_THREADED !== '1', 'Only the opt-in threaded distribution needs COOP/COEP');
  await context.route('**/*', async route => {
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
