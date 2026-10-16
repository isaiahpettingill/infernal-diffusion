import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
import { MonsterGenerator } from '../client.js';

class RecordingGenerator extends MonsterGenerator {
  _request(type, payload) { return Promise.resolve({ type, ...payload }); }
}

test('suggestPrompt is independent of V1/V2, defaults correctly, and preserves maximum u64', async () => {
  const generator = new RecordingGenerator();
  assert.deepEqual(await generator.suggestPrompt('18446744073709551615'), {
    type: 'suggest', seed: 18446744073709551615n, difficulty: 2, prompt_profile: 'default',
  });
  for (const promptProfile of ['default', 'necromancer', 'shaman', 'lunatic', 'boss', 'ocean']) {
    assert.equal((await generator.suggestPrompt(42n, 3, { promptProfile: promptProfile.toUpperCase() })).prompt_profile, promptProfile);
  }
});

test('suggestPrompt strictly validates before initializing a worker', async () => {
  const generator = new MonsterGenerator();
  for (const seed of [-1n, 18446744073709551616n, 1.5, 2 ** 60, '-1', '1e2', '', ' 42', {}, null, true]) {
    await assert.rejects(generator.suggestPrompt(seed));
  }
  for (const difficulty of [0, 4, -1, 257, 1.5, NaN, Infinity, '2', null]) {
    await assert.rejects(generator.suggestPrompt(42n, difficulty), /Difficulty/);
  }
  for (const promptProfile of ['v1', 'v2', '', ' ocean ', null, 1]) {
    await assert.rejects(generator.suggestPrompt(42n, 2, { promptProfile }), /profile/);
  }
  assert.equal(generator.worker, null);
});

async function worker({ supportsSuggestion = true } = {}) {
  const calls = [], messages = [];
  let release;
  const initialized = new Promise(resolve => { release = resolve; });
  const api = { default: () => initialized, initThreadPool: async () => {},
    generate_monster: (...args) => { calls.push(['generate', ...args]); return { packages: [] }; } };
  if (supportsSuggestion) api.suggest_prompt = (...args) => {
    calls.push(['suggest', ...args]);
    return `Rust prompt ${args.join(':')}`;
  };
  const source = (await readFile(new URL('../worker.js', import.meta.url), 'utf8'))
    .replace("await import('./pkg/infernal_diffusion.js')", '__api');
  const self = { crossOriginIsolated: true, postMessage: (message, transfers) => messages.push({ ...message, transfers }) };
  vm.runInNewContext(source, { self, __api: api, WebAssembly, SharedArrayBuffer,
    navigator: { hardwareConcurrency: 4 }, setTimeout, clearTimeout });
  return { send: data => self.onmessage({ data }), calls, messages, release };
}

test('worker queues mixed suggestions/generation through initialization and calls Rust with exact u64', async () => {
  const w = await worker();
  const first = w.send({ id: 1, type: 'suggest', seed: 18446744073709551615n, difficulty: 3, prompt_profile: 'OCEAN' });
  const second = w.send({ id: 2, type: 'generate', seed: 7n, prompt: 'arbitrary creature' });
  const third = w.send({ id: 3, type: 'suggest', seed: 8n, difficulty: 1 });
  assert.equal(w.calls.length, 0);
  w.release();
  await Promise.all([first, second, third]);
  assert.deepEqual(w.calls, [['suggest', 18446744073709551615n, 3, 'ocean'],
    ['generate', 'arbitrary creature', 7n], ['suggest', 8n, 1, 'default']]);
  assert.deepEqual(w.messages.map(message => message.id), [1, 2, 3]);
  assert.equal(w.messages[0].result, 'Rust prompt 18446744073709551615:3:ocean');
  assert.equal(w.messages[0].transfers, undefined, 'plain text does not use atlas transfers');
});

test('worker rejects invalid direct suggestions and remains usable afterward', async () => {
  const w = await worker(); w.release();
  const valid = { id: 1, type: 'suggest', seed: 1n, difficulty: 2, prompt_profile: 'default' };
  const invalid = [{ seed: 1 }, { seed: -1n }, { seed: 18446744073709551616n },
    { difficulty: 257 }, { difficulty: '2' }, { difficulty: 1.5 }, { prompt_profile: 'v2' }, { prompt_profile: null }];
  for (const [index, changed] of invalid.entries()) await w.send({ ...valid, ...changed, id: index + 1 });
  assert.equal(w.calls.length, 0);
  assert.ok(w.messages.every(message => message.type === 'error' && message.error.code === 'SUGGESTION_FAILED'));
  await w.send({ ...valid, id: 99 });
  assert.equal(w.messages.at(-1).type, 'result');
});

test('old WASM builds reject suggestions explicitly but retain V1 generation', async () => {
  const w = await worker({ supportsSuggestion: false }); w.release();
  await w.send({ id: 1, type: 'suggest', seed: 1n, difficulty: 2 });
  assert.match(w.messages[0].error.message, /does not support random prompts/);
  await w.send({ id: 2, type: 'generate', seed: 1n, prompt: 'goblin' });
  assert.equal(w.messages[1].type, 'result');
});

test('client correlates mixed results and cancellation rejects all queued requests before restart', async () => {
  const oldWorker = globalThis.Worker;
  const workers = [];
  globalThis.Worker = class {
    constructor() { this.messages = []; this.terminated = false; workers.push(this); }
    postMessage(message) { this.messages.push(message); }
    terminate() { this.terminated = true; }
  };
  const generator = new MonsterGenerator();
  try {
    const first = generator.suggestPrompt(1n);
    const second = generator.generate('manual prompt', 2n);
    const worker = workers[0];
    worker.onmessage({ data: { id: worker.messages[1].id, type: 'result', result: { packages: [] } } });
    worker.onmessage({ data: { id: worker.messages[0].id, type: 'result', result: 'suggestion' } });
    assert.equal(await first, 'suggestion');
    assert.deepEqual(await second, { packages: [] });
    const controller = new AbortController();
    const pending = generator.suggestPrompt(3n, 1, { signal: controller.signal });
    const generation = generator.generate('another prompt', 4n);
    const rejected = [assert.rejects(pending, { name: 'AbortError' }), assert.rejects(generation, { name: 'AbortError' })];
    controller.abort();
    await Promise.all(rejected);
    assert.equal(worker.terminated, true);
    const next = generator.suggestPrompt(5n);
    const restarted = workers[1];
    worker.onmessage({ data: { id: restarted.messages[0].id, type: 'result', result: 'stale' } });
    restarted.onmessage({ data: { id: restarted.messages[0].id, type: 'result', result: 'fresh' } });
    assert.equal(await next, 'fresh');
    generator.dispose();
    await assert.rejects(generator.suggestPrompt(), /disposed/);
  } finally { generator.dispose(); globalThis.Worker = oldWorker; }
});
