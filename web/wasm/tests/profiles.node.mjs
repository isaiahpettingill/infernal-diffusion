import test from 'node:test';
import assert from 'node:assert/strict';
import { MonsterGenerator } from '../client.js';

// No browser or generated WASM binary is needed to check the public request contract.
class RecordingGenerator extends MonsterGenerator {
  _request(type, payload) { return Promise.resolve({ type, ...payload }); }
}

test('existing calls default to V1 and preserve seeds', async () => {
  const result = await new RecordingGenerator().generate('ogre', '18446744073709551615');
  assert.equal(result.profile, 'v1');
  assert.equal(result.seed, 18446744073709551615n);
});

test('V2 must be selected explicitly', async () => {
  const result = await new RecordingGenerator().generate('ogre', 42, { profile: 'v2' });
  assert.equal(result.profile, 'v2');
  assert.equal(result.seed, 42n);
});

test('unknown profiles reject before worker initialization', async () => {
  for (const profile of ['v3', '', 'V2', 2, null]) {
    await assert.rejects(new RecordingGenerator().generate('ogre', 42, { profile }), /Profile must be v1 or v2/);
  }
});

import { readFile } from 'node:fs/promises';
import vm from 'node:vm';

async function runWorker(profile, supportsV2) {
  const calls = [];
  const messages = [];
  const api = {
    default: async () => {},
    initThreadPool: async () => {},
    generate_monster: (...args) => { calls.push(['v1', ...args]); return { packages: [] }; },
  };
  if (supportsV2) api.generate_monster_with_profile = (...args) => {
    calls.push(['v2', ...args]); return { packages: [] };
  };
  // Substitute only the WASM module loader, preserving the worker's real request
  // validation, export selection and error serialization.
  const source = (await readFile(new URL('../worker.js', import.meta.url), 'utf8'))
    .replace("await import('./pkg/infernal_diffusion.js')", '__api');
  const self = { crossOriginIsolated: true, postMessage: message => messages.push(message) };
  vm.runInNewContext(source, { self, __api: api, WebAssembly, SharedArrayBuffer,
    navigator: { hardwareConcurrency: 4 }, setTimeout, clearTimeout });
  await self.onmessage({ data: { id: 1, type: 'generate', prompt: 'ogre', seed: 42n, profile } });
  return { calls, messages };
}

test('worker preserves V1 on old binaries', async () => {
  const { calls, messages } = await runWorker(undefined, false);
  assert.equal(calls[0][0], 'v1');
  assert.equal(messages[0].type, 'result');
});

test('worker routes explicit V2 to the profile-aware export', async () => {
  const { calls, messages } = await runWorker('v2', true);
  assert.deepEqual(calls[0], ['v2', 'ogre', 42n, 'v2']);
  assert.equal(messages[0].type, 'result');
});

test('worker never downgrades V2 on an old binary', async () => {
  const { calls, messages } = await runWorker('v2', false);
  assert.equal(calls.length, 0);
  assert.equal(messages[0].type, 'error');
  assert.match(messages[0].error.message, /does not support V2/);
});

test('worker independently rejects unknown profiles', async () => {
  const { calls, messages } = await runWorker('v3', true);
  assert.equal(calls.length, 0);
  assert.equal(messages[0].type, 'error');
  assert.match(messages[0].error.message, /Profile must be v1 or v2/);
});
