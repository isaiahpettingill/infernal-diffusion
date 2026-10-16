/**
 * Real Candle BERT WASM regression for environments without a browser.
 * Run after building pkg: node web/wasm/tests/explicit-counts.actual-wasm.node.mjs
 *
 * The generated package and its WASM binary are copied to a temporary directory.
 * For optional threaded builds, only Rayon's Web Worker launcher is replaced with worker_threads;
 * parsing, anatomy, rendering, parallel work, and package export remain real WASM.
 * Portable builds need no worker shim. This does not replace real browser coverage.
 */
import assert from 'node:assert/strict';
import { cp, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const temp = await mkdtemp(join(tmpdir(), 'infernal-actual-wasm-'));
let host;
try {
  const pkg = join(temp, 'pkg');
  await cp(new URL('../pkg/', import.meta.url), pkg, { recursive: true });
  await writeFile(join(temp, 'package.json'), '{"type":"module"}\n');
  const threaded = (await readFile(join(pkg, 'infernal_diffusion.js'), 'utf8')).includes('initThreadPool');
  if (threaded) {
    const snippets = await readdir(join(pkg, 'snippets'));
    const rayon = snippets.find(name => name.startsWith('wasm-bindgen-rayon-'));
    assert.ok(rayon, 'Built package must contain the Rayon worker helper');
    const helper = join(pkg, 'snippets', rayon, 'src', 'workerHelpers.no-bundler.js');
    await writeFile(helper, `
  import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
  const workers = [];
  if (!isMainThread) setImmediate(async () => {
    const api = await import(workerData.mainJS);
    await api.default({ module_or_path: workerData.module, memory: workerData.memory });
    parentPort.postMessage('ready');
    api.wbg_rayon_start_worker(workerData.receiver);
  });
  export async function startWorkers(module, memory, builder) {
    if (builder.numThreads() === 0) throw new Error('Rayon needs at least one worker');
    const workerData = { module, memory, receiver: builder.receiver(), mainJS: builder.mainJS() };
    await Promise.all(Array.from({ length: builder.numThreads() }, () => new Promise((resolve, reject) => {
      const worker = new Worker(new URL(import.meta.url), { workerData });
      workers.push(worker);
      worker.once('message', message => message === 'ready' ? resolve() : reject(new Error('Unexpected worker response')));
      worker.once('error', reject);
      worker.once('exit', code => { if (code !== 0) reject(new Error('Rayon worker exited: ' + code)); });
    })));
    builder.build();
  }
  export async function disposeWorkers() {
    await Promise.all(workers.map(worker => worker.terminate()));
  }
  `);
    host = await import(pathToFileURL(helper).href);
  }
  const api = await import(pathToFileURL(join(pkg, 'infernal_diffusion.js')).href);
  await api.default({ module_or_path: await readFile(join(pkg, 'infernal_diffusion_bg.wasm')) });
  let timer;
  if (threaded) {
    try {
      await Promise.race([
        api.initThreadPool(2),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Rayon pool initialization timed out')), 45_000); }),
      ]);
    } finally { clearTimeout(timer); }
  }
  const info = api.generator_info();
  assert.equal(info.bundledModel, true);
  assert.equal(info.threaded, threaded);
  assert.equal(info.model, 'google/bert_uncased_L-2_H-128_A-2');
  assert.equal(api.initialize_model(), 'candle-bert-prototypes-v2');
  console.log(`Actual WebAssembly and bundled Candle BERT loaded; ${threaded ? 'two Rayon workers' : 'portable runtime'} ready`);
  for (const [prompt, heads, arms] of [
    ['three-headed dinosaur with eight arms', 3, 8],
    ['3-headed t rex with 8 arms', 3, 8],
    ['skeleton mermaid with eight arms', 1, 8],
  ]) {
    const started = performance.now();
    const result = api.generate_monster_with_profile(prompt, 42n, 'v2');
    const root = result.packages.at(-1);
    assert.equal(root.monster.generation.parser, 'candle-bert-prototypes-v2');
    assert.equal(String(root.monster.generation.seed), '42');
    const actualHeads = root.monster.colliders.filter(c => /^head_\d+$/.test(c.node_id)).length;
    const actualArms = root.monster.colliders.filter(c => /^hand_\d+$/.test(c.node_id)).length;
    assert.equal(actualHeads, heads, prompt + ': head count');
    assert.equal(actualArms, arms, prompt + ': arm count');
    assert.ok(root.sprites.rgba.some((value, i) => i % 4 === 3 && value > 0), prompt + ': visible pixels');
    assert.equal(root.sprites.rgba.length, root.sprites.width * root.sprites.height * 4, prompt + ': complete atlas');
    assert.ok(root.monster.animations.some(a => a.semantic_state === 'ATTACK'), prompt + ': attack animation');
    console.log(JSON.stringify({ prompt, heads: actualHeads, arms: actualArms, width: root.sprites.width,
      height: root.sprites.height, rgbaBytes: root.sprites.rgba.length, elapsedMs: Math.round(performance.now() - started) }));
  }
} finally {
  await host?.disposeWorkers();
  await rm(temp, { recursive: true, force: true });
}
