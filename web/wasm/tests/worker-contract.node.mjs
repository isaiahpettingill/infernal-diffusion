import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import vm from 'node:vm';
import { MonsterGenerator } from '../client.js';

const modelInfo = {
  model: 'google/bert_uncased_L-2_H-128_A-2',
  parser: 'candle-bert-prototypes-v2', bundledModel: true, threaded: false,
  liveGenerationBudgetVersion: 1,
};
const phases = ['parsing', 'building', 'rendering', 'packaging'];

async function worker({ threaded = false, isolated = false, initializationError, generationError, compact = false, budgetVersion = 1 } = {}) {
  const calls = [], messages = [];
  const info = { ...modelInfo, threaded };
  if (budgetVersion === null) delete info.liveGenerationBudgetVersion;
  else info.liveGenerationBudgetVersion = budgetVersion;
  const result = { packages: [] };
  const api = {
    default: async () => { calls.push(['wasm']); },
    initialize_model: () => {
      calls.push(['initialize_model']);
      if (initializationError) throw initializationError;
      return info.parser;
    },
    generator_info: () => info,
    generate_monster: (...args) => {
      calls.push(['generate', ...args]);
      if (generationError) throw generationError;
      return result;
    },
    generate_monster_with_profile: (...args) => { calls.push(['profile', ...args]); return result; },
    generate_monster_with_profile_progress: (prompt, seed, profile, onProgress) => {
      calls.push(['progress', prompt, seed, profile]);
      for (const phase of phases) onProgress(phase, 0, 1);
      onProgress('rendering', 1, 1);
      if (generationError) throw generationError;
      onProgress('complete', 1, 1);
      return result;
    },
    generate_hybrid_json: (...args) => { calls.push(['hybrid', ...args]); return result; },
    generate_hybrid_json_progress: (prompt, seed, onProgress) => {
      calls.push(['hybrid-progress', prompt, seed]);
      for (const phase of phases.slice(1)) onProgress(phase, 0, 1);
      onProgress('complete', 1, 1);
      return result;
    },
    suggest_prompt: () => 'a small goblin',
  };
  if (compact) {
    api.generate_monster_compact_progress = (prompt, seed, profile, progress) => {
      calls.push(['compact', prompt, seed, profile]); progress('complete',1,1); return result;
    };
    api.generate_hybrid_compact_progress = (prompt, seed, progress) => {
      calls.push(['compact-hybrid',prompt,seed]); progress('complete',1,1); return result;
    };
  }
  if (threaded) api.initThreadPool = async count => { calls.push(['pool', count]); };
  const source = (await readFile(new URL('../worker.js', import.meta.url), 'utf8'))
    .replace("await import('./pkg/infernal_diffusion.js')", '__api');
  const self = { crossOriginIsolated: isolated,
    postMessage: (message, transfers) => messages.push({ ...message, transfers }) };
  vm.runInNewContext(source, {
    self, __api: api, WebAssembly, Uint8Array, TextEncoder, navigator: { hardwareConcurrency: 16 },
    setTimeout, clearTimeout,
    ...(isolated ? { SharedArrayBuffer } : {}),
  });
  return { send: data => self.onmessage({ data }), calls, messages };
}

const plain = value => JSON.parse(JSON.stringify(value));

test('portable readiness initializes real model once without isolation or shared memory', async () => {
  const w = await worker();
  await Promise.all([w.send({ id: 1, type: 'ready' }), w.send({ id: 2, type: 'ready' })]);
  assert.deepEqual(w.calls, [['wasm'], ['initialize_model']]);
  assert.equal(w.messages.length, 2);
  for (const [index, message] of w.messages.entries()) {
    const { transfers, ...reply } = plain(message);
    assert.deepEqual(reply, { id: index + 1, type: 'ready', threadCount: 1, ...modelInfo });
  }
});

test('readiness reports progress only when explicitly requested', async () => {
  const w = await worker();
  await w.send({ id: 1, type: 'ready', progress: true });
  const progress = w.messages.filter(message => message.type === 'progress');
  assert.ok(progress.length >= 2);
  assert.ok(progress.every(message => message.id === 1));
  assert.ok(progress.some(message => message.progress.phase === 'loading-wasm'));
  assert.ok(progress.some(message => message.progress.phase === 'initializing-model'));
  assert.equal(w.messages.at(-1).type, 'ready');
  const before = w.messages.length;
  await w.send({ id: 2, type: 'ready' });
  assert.deepEqual(w.messages.slice(before).map(message => message.type), ['ready']);
  assert.equal(w.calls.filter(([name]) => name === 'initialize_model').length, 1);
});

test('model initialization failure has an actionable WASM_UNAVAILABLE response', async () => {
  const w = await worker({ initializationError: new Error('bundled model is invalid') });
  await w.send({ id: 1, type: 'ready' });
  assert.equal(w.messages.at(-1).type, 'error');
  assert.equal(w.messages.at(-1).error.code, 'WASM_UNAVAILABLE');
  assert.match(w.messages.at(-1).error.message, /bundled model is invalid/);
});

test('optional threaded builds initialize a bounded real pool', async () => {
  const w = await worker({ threaded: true, isolated: true });
  await w.send({ id: 1, type: 'ready' });
  const reply = w.messages.at(-1);
  assert.equal(reply.type, 'ready');
  assert.equal(reply.threaded, true);
  assert.equal(reply.threadCount, 4);
  assert.deepEqual(w.calls.filter(([name]) => name === 'pool'), [['pool', 4]]);
});

test('only optional threaded builds require isolation', async () => {
  const w = await worker({ threaded: true });
  await w.send({ id: 1, type: 'ready' });
  const reply = w.messages.at(-1);
  assert.equal(reply.type, 'error');
  assert.equal(reply.error.code, 'WASM_UNAVAILABLE');
  assert.match(reply.error.message, /cross-origin isolation/);
});

test('generation progress is request-correlated and keeps V1/V2 and seed intact', async () => {
  const w = await worker();
  for (const [index, profile] of ['v1', 'v2'].entries()) {
    const id = index + 1;
    await w.send({ id, type: 'generate', prompt: 'ogre', seed: 18446744073709551615n, profile, progress: true });
    const messages = w.messages.filter(message => message.id === id);
    const progress = messages.filter(message => message.type === 'progress').map(message => message.progress);
    for (const phase of [...phases, 'complete']) assert.ok(progress.some(update => update.phase === phase), phase);
    assert.ok(progress.some(update => update.phase === 'rendering' && update.completed === 1 && update.total === 1));
    assert.equal(messages.at(-1).type, 'result');
    assert.ok(w.calls.some(call => call[0] === 'progress' && call[2] === 18446744073709551615n && call[3] === profile));
  }
});

test('unsolicited progress is forbidden, including truthy nonboolean flags', async () => {
  const w = await worker();
  for (const [index, progress] of [undefined, false, 1, 'true'].entries()) {
    await w.send({ id: index + 1, type: 'generate', prompt: 'ogre', seed: 42n, progress });
  }
  assert.equal(w.messages.length, 4);
  assert.ok(w.messages.every(message => message.type === 'result'));
});

test('a failed generation cannot poison readiness or suggestions later in the queue', async () => {
  const w = await worker({ generationError: new Error('invalid monster recipe') });
  await Promise.all([
    w.send({ id: 1, type: 'generate', prompt: 'ogre', seed: 42n, progress: true }),
    w.send({ id: 2, type: 'ready' }),
    w.send({ id: 3, type: 'suggest', seed: 1n, difficulty: 2 }),
  ]);
  const terminal = w.messages.filter(message => message.type !== 'progress');
  assert.deepEqual(terminal.map(message => [message.id, message.type]), [[1, 'error'], [2, 'ready'], [3, 'result']]);
  assert.equal(terminal[0].error.code, 'GENERATION_FAILED');
  assert.equal(terminal[2].result, 'a small goblin');
});

test('hybrids retain their dedicated export with opt-in progress', async () => {
  const w = await worker();
  await w.send({ id: 1, type: 'generate_hybrid', prompt: '{}', seed: 42n, profile: 'v2', progress: true });
  assert.ok(w.calls.some(call => call[0] === 'hybrid-progress'));
  assert.ok(!w.calls.some(call => call[0] === 'progress'));
  assert.ok(w.messages.some(message => message.type === 'progress' && message.progress.phase === 'rendering'));
  assert.equal(w.messages.at(-1).type, 'result');
});

function clientFixture(options) {
  const original = globalThis.Worker;
  const workers = [];
  globalThis.Worker = class {
    constructor() { this.messages = []; this.terminated = false; workers.push(this); }
    postMessage(message) { this.messages.push(message); }
    terminate() { this.terminated = true; }
  };
  const generator = new MonsterGenerator(options);
  return { generator, workers, cleanup() { generator.dispose(); globalThis.Worker = original; } };
}

test('client progress keeps a request pending and readiness preserves model metadata', async () => {
  const f = clientFixture();
  try {
    const progress = [];
    const pending = f.generator.ready({ onProgress: update => progress.push(update) });
    const w = f.workers[0], id = w.messages[0].id;
    assert.equal(w.messages[0].progress, true);
    w.onmessage({ data: { id, type: 'progress', progress: { phase: 'initializing-model', completed: 0, total: 1 } } });
    assert.equal(f.generator.pending.size, 1);
    assert.deepEqual(progress, [{ phase: 'initializing-model', completed: 0, total: 1 }]);
    w.onmessage({ data: { id, type: 'ready', threadCount: 1, ...modelInfo } });
    assert.deepEqual(await pending, { threadCount: 1, ...modelInfo });
    assert.equal(f.generator.pending.size, 0);
  } finally { f.cleanup(); }
});

test('generation callbacks are isolated by request and opt-in only', async () => {
  const f = clientFixture();
  try {
    const firstProgress = [], secondProgress = [];
    const first = f.generator.generate('ogre', 42n, { onProgress: update => firstProgress.push(update) });
    const second = f.generator.generate('slime', 43n, { profile: 'v2', onProgress: update => secondProgress.push(update) });
    const third = f.generator.generate('wolf', 44n);
    const w = f.workers[0];
    assert.equal(w.messages[0].progress, true);
    assert.equal(w.messages[1].progress, true);
    assert.notEqual(w.messages[2].progress, true);
    w.onmessage({ data: { id: w.messages[1].id, type: 'progress', progress: { phase: 'rendering' } } });
    assert.deepEqual(firstProgress, []);
    assert.deepEqual(secondProgress, [{ phase: 'rendering' }]);
    for (const { id } of w.messages) w.onmessage({ data: { id, type: 'result', result: { packages: [] } } });
    await Promise.all([first, second, third]);
  } finally { f.cleanup(); }
});

test('aborting from a progress callback cancels every request and ignores stale worker progress', async () => {
  const f = clientFixture();
  try {
    const controller = new AbortController();
    const pending = f.generator.generate('dragon', 1n, { signal: controller.signal, onProgress: () => controller.abort() });
    const queued = f.generator.ready();
    const rejected = [assert.rejects(pending, { name: 'AbortError' }), assert.rejects(queued, { name: 'AbortError' })];
    const stale = f.workers[0];
    stale.onmessage({ data: { id: stale.messages[0].id, type: 'progress', progress: { phase: 'rendering' } } });
    await Promise.all(rejected);
    assert.equal(stale.terminated, true);
    const progress = [];
    const fresh = f.generator.ready({ onProgress: update => progress.push(update) });
    const w = f.workers[1], id = w.messages[0].id;
    stale.onmessage({ data: { id, type: 'progress', progress: { phase: 'stale' } } });
    assert.deepEqual(progress, []);
    w.onmessage({ data: { id, type: 'ready', threadCount: 1, ...modelInfo } });
    assert.deepEqual(await fresh, { threadCount: 1, ...modelInfo });
  } finally { f.cleanup(); }
});

test('prompt limits count UTF-8 bytes and reject oversized input before worker startup', async () => {
  class RecordingGenerator extends MonsterGenerator {
    _request(type, payload) { return Promise.resolve({ type, ...payload }); }
  }
  const generator = new RecordingGenerator();
  for (const prompt of ['a'.repeat(4096), '🐉'.repeat(1024)]) {
    assert.equal((await generator.generate(prompt, 42n)).prompt, prompt);
  }
  for (const prompt of ['a'.repeat(4097), '🐉'.repeat(1025)]) {
    await assert.rejects(generator.generate(prompt, 42n), /4096 UTF-8 bytes/);
  }
  assert.equal(generator.worker, null);
});

test('worker enforces UTF-8 prompt limits without poisoning subsequent work', async () => {
  const w = await worker();
  await w.send({ id: 1, type: 'generate', prompt: '🐉'.repeat(1025), seed: 42n });
  await w.send({ id: 2, type: 'generate', prompt: 'slime', seed: 42n });
  assert.equal(w.messages[0].type, 'error');
  assert.equal(w.messages[0].error.code, 'GENERATION_FAILED');
  assert.match(w.messages[0].error.message, /4096 UTF-8 bytes/);
  assert.equal(w.messages[1].type, 'result');
});

test('throwing progress callbacks do not corrupt result delivery', async () => {
  const f = clientFixture();
  const originalError = console.error;
  const logged = [];
  console.error = (...args) => logged.push(args);
  try {
    const pending = f.generator.ready({ onProgress() { throw new Error('UI bug'); } });
    const w = f.workers[0], id = w.messages[0].id;
    assert.doesNotThrow(() => w.onmessage({ data: { id, type: 'progress', progress: { phase: 'loading-wasm' } } }));
    assert.equal(f.generator.pending.size, 1);
    w.onmessage({ data: { id, type: 'ready', threadCount: 1, ...modelInfo } });
    assert.deepEqual(await pending, { threadCount: 1, ...modelInfo });
    assert.equal(logged.length, 1);
  } finally { console.error = originalError; f.cleanup(); }
});

test('request timeouts terminate all queued work and a later request restarts', async () => {
  const f = clientFixture({ timeoutMs: 25 });
  try {
    const first = f.generator.ready();
    const second = f.generator.generate('slime', 42n);
    await Promise.all([
      assert.rejects(first, { code: 'TIMEOUT' }),
      assert.rejects(second, { code: 'TIMEOUT' }),
    ]);
    assert.equal(f.workers[0].terminated, true);
    const next = f.generator.ready();
    const w = f.workers[1];
    w.onmessage({ data: { id: w.messages[0].id, type: 'ready', threadCount: 1, ...modelInfo } });
    assert.deepEqual(await next, { threadCount: 1, ...modelInfo });
  } finally { f.cleanup(); }
});


test('compact storage is explicit and keeps profiles, full seeds and hybrid routing', async () => {
  const w = await worker({compact:true});
  await w.send({id:1,type:'generate',prompt:'shade humanoid',seed:18446744073709551615n,profile:'v2',compact:true});
  await w.send({id:2,type:'generate_hybrid',prompt:'{}',seed:42n,profile:'v2',compact:true,progress:true});
  assert.ok(w.calls.some(c=>c[0]==='compact' && c[2]===18446744073709551615n && c[3]==='v2'));
  assert.ok(w.calls.some(c=>c[0]==='compact-hybrid'));
  assert.equal(w.messages.filter(m=>m.id===1 && m.type==='progress').length,0);
  assert.equal(w.messages.filter(m=>m.type==='result').length,2);
});

test('compact requests never silently fall back to incompatible dense or V1 generation', async () => {
  const old = await worker();
  await old.send({id:1,type:'generate',prompt:'ogre',seed:42n,compact:true});
  assert.match(old.messages.at(-1).error.message,/compact atlases/);
  const modern=await worker({compact:true});
  await modern.send({id:1,type:'generate_hybrid',prompt:'{}',seed:42n,profile:'v1',compact:true});
  assert.match(modern.messages.at(-1).error.message,/requires V2/);
});


test('readiness reports actual budget capability and never fabricates it for older compact WASM', async () => {
  const modern = await worker({ compact: true, budgetVersion: 1 });
  await modern.send({ id: 1, type: 'ready' });
  assert.equal(modern.messages.at(-1).liveGenerationBudgetVersion, 1);
  const older = await worker({ compact: true, budgetVersion: null });
  await older.send({ id: 1, type: 'ready' });
  assert.equal(older.messages.at(-1).liveGenerationBudgetVersion, undefined);
  await older.send({ id: 2, type: 'generate', prompt: 'slime', seed: 42n, compact: true });
  assert.equal(older.messages.at(-1).type, 'result', 'Legacy manual arena use remains compatible');
});
