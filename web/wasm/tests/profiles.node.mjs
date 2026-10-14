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

async function runWorker(profile, supportsV2, type = 'generate', supportsHybrid = false) {
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
  if (supportsHybrid) api.generate_hybrid_json = (...args) => {
    calls.push(['hybrid', ...args]); return { packages: [] };
  };
  // Substitute only the WASM module loader, preserving the worker's real request
  // validation, export selection and error serialization.
  const source = (await readFile(new URL('../worker.js', import.meta.url), 'utf8'))
    .replace("await import('./pkg/infernal_diffusion.js')", '__api');
  const self = { crossOriginIsolated: true, postMessage: message => messages.push(message) };
  vm.runInNewContext(source, { self, __api: api, WebAssembly, SharedArrayBuffer,
    navigator: { hardwareConcurrency: 4 }, setTimeout, clearTimeout });
  await self.onmessage({ data: { id: 1, type, prompt: 'ogre', seed: 42n, profile } });
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


test('structured hybrid JSON uses a dedicated explicit V2 request', async () => {
  const recipe = { schema_version: 1, upper_body: 'HUMANOID', lower_body: 'HORSE',
    head: 'HUMANOID', lower_limb_count: 6, upper_arm_count: 2 };
  const result = await new RecordingGenerator().generateHybrid(recipe, '42');
  assert.equal(result.type, 'generate_hybrid');
  assert.equal(result.profile, 'v2');
  assert.equal(result.seed, 42n);
  assert.deepEqual(JSON.parse(result.prompt), recipe);
  for (const invalid of [null, [], 2, '{bad json']) {
    await assert.rejects(new RecordingGenerator().generateHybrid(invalid, 42));
  }
  await assert.rejects(new RecordingGenerator().generateHybrid(recipe, -1));
});

test('worker routes structured hybrids without falling back to a prompt parser', async () => {
  const supported = await runWorker('v2', true, 'generate_hybrid', true);
  assert.equal(supported.calls[0][0], 'hybrid');
  assert.equal(supported.messages[0].type, 'result');
  const unsupported = await runWorker('v2', true, 'generate_hybrid', false);
  assert.equal(unsupported.calls.length, 0);
  assert.match(unsupported.messages[0].error.message, /does not support structured hybrids/);
  const invalid = await runWorker('v1', true, 'generate_hybrid', true);
  assert.equal(invalid.calls.length, 0);
  assert.match(invalid.messages[0].error.message, /requires V2/);
});
