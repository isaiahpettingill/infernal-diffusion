/**
 * Real portable WASM/client/worker integration without a browser or HTTP server.
 * Run after ./tools/build_wasm.sh:
 *   node web/wasm/tests/worker.actual-wasm.node.mjs
 *
 * ONLY browser host facilities are adapted: Worker -> node:worker_threads,
 * self/postMessage/navigator, and the generated loader's fetch -> local WASM bytes.
 * client.js, worker.js, generated bindings, BERT inference, generation, metadata,
 * rendering and transfer lists are the actual production code, without mocks.
 * This is Node-host validation, NOT proof of browser deployment or CSP behavior.
 */
import assert from 'node:assert/strict';
import { createHash, webcrypto } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { Worker as NodeWorker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { setImmediate as nextTurn } from 'node:timers/promises';

const workerURL = new URL('../worker.js', import.meta.url);
const wasmURL = new URL('../pkg/infernal_diffusion_bg.wasm', import.meta.url);
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const parser = 'candle-bert-prototypes-v2';
const model = 'google/bert_uncased_L-2_H-128_A-2';

if (!isMainThread) {
  globalThis.self = globalThis;
  Object.defineProperty(globalThis, 'crossOriginIsolated', { value: false });
  Object.defineProperty(globalThis, 'SharedArrayBuffer', { value: undefined });
  Object.defineProperty(globalThis, 'navigator', { value: { hardwareConcurrency: 1 } });
  if (!globalThis.crypto) globalThis.crypto = webcrypto;
  globalThis.fetch = async input => {
    const url = input instanceof URL ? input.href : typeof input === 'string' ? input : input.url;
    parentPort.postMessage({ __host: 'fetch', url });
    if (url !== workerData.wasmURL) {
      throw new Error(`Network access forbidden by the real-WASM test host: ${url}`);
    }
    return new Response(await readFile(new URL(url)), { headers: { 'Content-Type': 'application/wasm' } });
  };
  // The production runtime needs no socket connection. Any attempted future
  // WebSocket fetch must also fail rather than escaping the offline test host.
  globalThis.WebSocket = class { constructor() { throw new Error('Network access forbidden by the real-WASM test host'); } };
  globalThis.postMessage = (data, transfers = []) => {
    const before = transfers.map(buffer => buffer.byteLength);
    parentPort.postMessage(data, transfers);
    if (transfers.length) parentPort.postMessage({ __host: 'transfer',
      before, after: transfers.map(buffer => buffer.byteLength) });
  };
  await import(workerData.workerURL);
  parentPort.on('message', data => globalThis.onmessage({ data }));
  parentPort.postMessage({ __host: 'ready', isolated: globalThis.crossOriginIsolated,
    sharedArrayBuffer: typeof globalThis.SharedArrayBuffer });
} else {
  const { MonsterGenerator } = await import('../client.js');
  const originalWorker = globalThis.Worker;
  const workers = [], terminations = [];
  class BrowserWorkerAdapter {
    constructor(url, options) {
      assert.equal(new URL(url).href, workerURL.href, 'Use the production module worker');
      assert.equal(options.type, 'module');
      this.messages = [];
      this.replies = [];
      this.hostEvents = [];
      this.terminated = false;
      this.native = new NodeWorker(new URL(import.meta.url), {
        workerData: { workerURL: new URL(url).href, wasmURL: wasmURL.href },
      });
      this.native.on('message', data => {
        if (data.__host) this.hostEvents.push(data);
        else {
          // Retain protocol metadata only; retaining atlas payloads would keep
          // hundreds of megabytes alive after each assertion.
          this.replies.push({ id: data.id, type: data.type, progress: data.progress });
          this.onmessage?.({ data });
        }
      });
      this.native.on('error', error => this.onerror?.({ message: error.message, preventDefault() {} }));
      this.native.on('exit', code => {
        if (!this.terminated) this.onerror?.({ message: `Node worker exited unexpectedly (${code})`, preventDefault() {} });
      });
      workers.push(this);
    }
    postMessage(message, transfers) {
      this.messages.push(message);
      this.native.postMessage(message, transfers);
    }
    terminate() {
      this.terminated = true;
      terminations.push(this.native.terminate());
    }
  }
  globalThis.Worker = BrowserWorkerAdapter;
  const generator = new MonsterGenerator({ timeoutMs: 240_000 });
  const summary = { host: 'Node worker_threads browser-host adapter', browserVerified: false,
    parser, checks: [], profiles: [] };
  const initialBinaryHash = hash(await readFile(wasmURL));
  let ticks = 0;
  const timer = setInterval(() => ticks++, 5);
  const started = performance.now();
  function compact(result) {
    return result.packages.map(pkg => ({
      path: pkg.path,
      metadata: JSON.parse(JSON.stringify(pkg.monster, (_, value) => typeof value === 'bigint' ? value.toString() : value)),
      atlases: [pkg.sprites, pkg.emission, pkg.projectiles].map(atlas => {
        if (!atlas) return null;
        assert.ok(atlas.rgba instanceof Uint8Array, 'Transferred pixels retain Uint8Array type');
        assert.ok(atlas.width > 0 && atlas.height > 0, 'Nonempty full atlas');
        assert.equal(atlas.rgba.byteLength, atlas.width * atlas.height * 4, 'Full-resolution RGBA dimensions');
        return { width: atlas.width, height: atlas.height, bytes: atlas.rgba.byteLength, sha256: hash(atlas.rgba) };
      }),
    }));
  }
  async function saveSample(result, profile) {
    const destination = process.env.INFERNAL_WASM_SAMPLE_DIR;
    if (!destination) return;
    const directory = join(destination, profile);
    await mkdir(directory, { recursive: true });
    const packages = [];
    for (const [index, pkg] of result.packages.entries()) {
      const folder = `package-${index}`;
      await mkdir(join(directory, folder), { recursive: true });
      const sample = { path: pkg.path, monster: pkg.monster };
      await writeFile(join(directory, folder, 'monster.json'), JSON.stringify(pkg.monster,
        (_, value) => typeof value === 'bigint' ? value.toString() : value, 2) + '\n');
      for (const name of ['sprites', 'emission', 'projectiles']) {
        const atlas = pkg[name];
        if (!atlas) { sample[name] = null; continue; }
        const file = `${folder}/${name}.rgba`;
        await writeFile(join(directory, file), atlas.rgba);
        sample[name] = { width: atlas.width, height: atlas.height, rgbaFile: file,
          byteLength: atlas.rgba.byteLength, sha256: hash(atlas.rgba) };
      }
      packages.push(sample);
    }
    await writeFile(join(directory, 'result.json'), JSON.stringify({ packages },
      (_, value) => typeof value === 'bigint' ? value.toString() : value, 2) + '\n');
    summary.sampleDirectory = destination;
  }
  function validateProgress(updates) {
    for (const phase of ['parsing', 'building', 'rendering', 'packaging', 'complete']) {
      assert.ok(updates.some(update => update.phase === phase), `Real progress: ${phase}`);
    }
    assert.equal(updates.at(-1).phase, 'complete');
    assert.ok(updates.some(update => update.phase === 'rendering' && update.total > 0));
  }
  try {
    const initializationProgress = [];
    const info = await generator.ready({ onProgress: update => initializationProgress.push(update) });
    assert.equal(info.model, model);
    assert.equal(info.parser, parser);
    assert.equal(info.bundledModel, true);
    assert.equal(info.threaded, false, 'Build the default portable distribution before this test');
    assert.equal(info.threadCount, 1);
    assert.equal(info.modelSha256, hash(await readFile(new URL('../../../models/bert-mini/model.safetensors', import.meta.url))));
    const secondProgress = [];
    assert.deepEqual(await generator.ready({ onProgress: update => secondProgress.push(update) }), info);
    assert.deepEqual(await generator.ready(), info);
    assert.equal(initializationProgress.filter(update => update.phase === 'initializing-model' && update.completed === 0).length, 1);
    assert.ok(initializationProgress.some(update => update.phase === 'loading-wasm'));
    assert.ok(!secondProgress.some(update => ['loading-wasm', 'initializing-model'].includes(update.phase)), 'Repeated readiness reuses the model');
    assert.equal(workers.length, 1);
    summary.runtime = info;
    summary.checks.push('real bundled BERT initialized once; repeated readiness reuses model');
    console.log('PASS real portable BERT readiness and model reuse');

    const prompt = 'small ice slime';
    const seed = '18446744073709551615';
    for (const profile of ['v1', 'v2']) {
      const progress = [];
      const beforeTicks = ticks;
      const options = { onProgress: update => progress.push(update) };
      if (profile === 'v2') options.profile = profile;
      let generated = await generator.generate(prompt, seed, options);
      await saveSample(generated, profile);
      const first = compact(generated);
      generated = null;
      const heartbeatTicks = ticks - beforeTicks;
      assert.ok(heartbeatTicks > 0, `${profile}: main thread remains responsive`);
      const replay = compact(await generator.generate(prompt, BigInt(seed), { profile }));
      assert.deepEqual(replay, first, `${profile}: complete metadata and every RGBA hash replay exactly`);
      validateProgress(progress);
      const root = first.at(-1).metadata;
      assert.equal(root.generation.parser, parser);
      assert.equal(root.generation.seed, seed);
      assert.equal(root.generation.prompt, prompt);
      if (profile === 'v1') assert.equal(root.v2_profile ?? null, null);
      else assert.equal(root.v2_profile.profile_version, 1);
      for (const key of ['world_scale', 'pixels_per_unit', 'reference_height_px', 'target_height_px', 'reference_player_height_px']) {
        assert.ok(Number.isFinite(root.size[key]) && root.size[key] > 0, `${profile}: ${key}`);
      }
      summary.profiles.push({ profile, seed, heartbeatTicks, progressEvents: progress.length,
        size: root.size, atlases: first.map(pkg => ({ path: pkg.path, atlases: pkg.atlases })) });
      summary.checks.push(`${profile}: exact u64, full metadata/all-atlas replay, world sizing, real progress and heartbeat`);
      console.log(`PASS ${profile} exact-u64 metadata + all-atlas SHA-256 replay (${heartbeatTicks} heartbeat ticks)`);
    }

    // Exercise an invalid low-level request so recovery is verified inside the
    // real worker queue rather than only through the client's early validation.
    await assert.rejects(generator._request('generate', { prompt: '', seed: 1n }), { code: 'GENERATION_FAILED' });
    assert.deepEqual(await generator.ready(), info);
    const suggestion = await generator.suggestPrompt(seed, 2, { promptProfile: 'ocean' });
    assert.equal(typeof suggestion, 'string');
    assert.ok(suggestion.length > 0);
    summary.checks.push('real worker queue recovers after generation error; suggestions remain usable');

    const controller = new AbortController();
    let cancelledPhase;
    const busy = generator.generate(prompt, 17n, { signal: controller.signal, onProgress: update => {
      if (update.phase === 'rendering' && !cancelledPhase) {
        cancelledPhase = update.phase;
        controller.abort();
      }
    } });
    const queued = generator.generate('small green slime', 18n);
    const queuedReady = generator.ready();
    await Promise.all([busy, queued, queuedReady].map(promise => assert.rejects(promise, { name: 'AbortError' })));
    assert.equal(cancelledPhase, 'rendering');
    assert.equal(workers[0].terminated, true);
    const restartProgress = [];
    assert.deepEqual(await generator.ready({ onProgress: update => restartProgress.push(update) }), info);
    assert.ok(restartProgress.some(update => update.phase === 'initializing-model'));
    assert.equal(workers.length, 2);
    const restarted = compact(await generator.generate(prompt, 19n));
    assert.equal(restarted.at(-1).metadata.generation.parser, parser);
    assert.equal(restarted.at(-1).metadata.generation.seed, '19');
    summary.checks.push('busy generation and all queued work cancel; fresh worker/model generates successfully');
    console.log('PASS cancellation during rendering, queued rejection and real worker/model restart');

    await nextTurn();
    for (const worker of workers) {
      const hostReady = worker.hostEvents.find(event => event.__host === 'ready');
      assert.deepEqual(hostReady, { __host: 'ready', isolated: false, sharedArrayBuffer: 'undefined' });
      const fetches = worker.hostEvents.filter(event => event.__host === 'fetch');
      assert.deepEqual(fetches.map(event => event.url), [wasmURL.href], 'Exactly one local WASM fetch per worker and no model/network requests');
      for (const reply of worker.replies.filter(reply => reply.type === 'progress')) {
        assert.equal(worker.messages.find(message => message.id === reply.id)?.progress, true, 'Progress is strictly opt-in and request-correlated');
      }
    }
    const transferEvents = workers.flatMap(worker => worker.hostEvents.filter(event => event.__host === 'transfer'));
    assert.ok(transferEvents.length >= 5, 'Actual generated atlas buffers use transfers');
    assert.ok(transferEvents.every(event => event.before.every(bytes => bytes > 0) && event.after.every(bytes => bytes === 0)), 'Transferred worker buffers are detached');
    assert.equal(hash(await readFile(wasmURL)), initialBinaryHash, 'The tested binary remained unchanged');
    summary.checks.push('no isolation/shared-memory requirement; local-only WASM loading; transferred worker buffers detached');
    generator.dispose();
    await assert.rejects(generator.ready(), /disposed/);
    summary.checks.push('permanent disposal enforced');
    summary.wasmSha256 = initialBinaryHash;
    summary.elapsedMs = Math.round(performance.now() - started);
    summary.memory = { peakRssMiB: Math.round(process.resourceUsage().maxRSS / 1024),
      finalRssMiB: Math.round(process.memoryUsage().rss / 1024 ** 2) };
    summary.status = 'PASS';
    console.log(JSON.stringify(summary, null, 2));
  } finally {
    clearInterval(timer);
    generator.dispose();
    globalThis.Worker = originalWorker;
    await Promise.all(terminations);
  }
}
