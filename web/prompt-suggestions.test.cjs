const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

function playground(fetchRequest) {
  const elements = new Map();
  const defaults = { prompt: 'my handwritten creature', seed: '18446744073709551615',
    difficulty: '2', 'prompt-profile': 'ocean', profile: 'v1', 'package-format': 'protobuf' };
  const element = id => {
    if (!elements.has(id)) elements.set(id, {
      value: defaults[id] || '', textContent: '', disabled: false,
      addEventListener() {}, removeEventListener() {}, classList: { toggle() {} },
      width: 960, height: 520, getContext: () => ({}),
    });
    return elements.get(id);
  };
  const requests = [];
  const context = vm.createContext({
    document: { getElementById: element, querySelectorAll: () => [] },
    window: { addEventListener() {} }, requestAnimationFrame() {},
    fetch: async (url, options) => {
      if (url === '/api/info') return { ok: true, json: async () => ({ bert: false }) };
      const request = { url, body: JSON.parse(options.body) };
      requests.push(request);
      return fetchRequest(request);
    },
  });
  for (const file of ['wasm/prompt-controls.js', 'app.js']) {
    vm.runInContext(fs.readFileSync(path.join(__dirname, file), 'utf8'), context);
  }
  return { context, element, requests, controls: vm.runInContext('promptControls', context) };
}

test('native UI sends lossless suggestion/generation seeds and keeps generation profile independent', async () => {
  const ui = playground(async request => request.url === '/api/suggest'
    ? { ok: true, json: async () => ({ prompt: 'Rust ocean creature' }) }
    : { ok: false, json: async () => ({ error: 'controlled renderer error' }) });
  await ui.controls.randomize();
  assert.deepEqual(ui.requests[0], { url: '/api/suggest', body: {
    seed: '18446744073709551615', difficulty: 2, prompt_profile: 'ocean',
  } });
  assert.equal(ui.element('prompt').value, 'Rust ocean creature');
  assert.equal(ui.element('profile').value, 'v1');
  await ui.context.generate({ preventDefault() {} });
  assert.deepEqual(ui.requests[1], { url: '/api/generate', body: {
    seed: '18446744073709551615', prompt: 'Rust ocean creature', profile: 'v1', format: 'protobuf',
  } });
  assert.equal(ui.element('generate').disabled, false);
  assert.equal(ui.element('random-prompt').disabled, false);
  assert.match(ui.element('status').textContent, /controlled renderer error/);
  assert.equal(ui.element('prompt').value, 'Rust ocean creature');
});

test('native generation blocks duplicate submit and stale pending suggestions', async () => {
  let resolveSuggestion, resolveGeneration;
  const ui = playground(request => new Promise(resolve => {
    if (request.url === '/api/suggest') resolveSuggestion = resolve;
    else resolveGeneration = resolve;
  }));
  const suggestion = ui.controls.randomize();
  const generation = ui.context.generate({ preventDefault() {} });
  await ui.context.generate({ preventDefault() {} });
  assert.equal(ui.requests.length, 2);
  assert.equal(ui.element('random-prompt').disabled, true);
  resolveSuggestion({ ok: true, json: async () => ({ prompt: 'too late' }) });
  await suggestion;
  assert.equal(ui.element('prompt').value, 'my handwritten creature');
  resolveGeneration({ ok: false, json: async () => ({ error: 'renderer unavailable' }) });
  await generation;
  assert.equal(ui.element('random-prompt').disabled, false);
});

test('native generation rejects overflow and fractional seeds before HTTP', async () => {
  const ui = playground(() => { throw new Error('should not fetch'); });
  for (const seed of ['18446744073709551616', '-1', '1.5', '1e3', '']) {
    ui.element('seed').value = seed;
    await ui.context.generate({ preventDefault() {} });
    assert.match(ui.element('status').textContent, /Seed/);
  }
  assert.equal(ui.requests.length, 0);
});
