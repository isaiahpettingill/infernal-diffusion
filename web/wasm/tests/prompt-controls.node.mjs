import test from 'node:test';
import assert from 'node:assert/strict';
import '../prompt-controls.js';

class Element extends EventTarget {
  constructor(value = '') { super(); this.value = value; this.disabled = false; this.textContent = ''; }
  input(value) { this.value = value; this.dispatchEvent(new Event('input')); }
}
function setup() {
  const elements = { prompt: new Element('arbitrary text'), seed: new Element('42'),
    difficulty: new Element('2'), profile: new Element('default'), button: new Element(), status: new Element() };
  const calls = [];
  const controls = new InfernalPromptControls({ ...elements, suggest: (...args) => new Promise((resolve, reject) => calls.push({ args, resolve, reject })) });
  return { ...elements, controls, calls };
}

test('first suggestion uses displayed full-u64 seed; repeated clicks advance and wrap visibly', async () => {
  const ui = setup();
  ui.seed.value = '18446744073709551615';
  let pending = ui.controls.randomize();
  assert.equal(ui.calls[0].args[0], 18446744073709551615n);
  assert.equal(ui.button.disabled, true);
  await ui.controls.randomize();
  assert.equal(ui.calls.length, 1, 'busy clicks cannot enqueue duplicate work');
  ui.calls[0].resolve('a sea monster');
  await pending;
  assert.equal(ui.prompt.value, 'a sea monster');
  assert.equal(ui.seed.value, '18446744073709551615');
  assert.equal(ui.button.disabled, false);
  pending = ui.controls.randomize();
  assert.equal(ui.calls[1].args[0], 0n);
  assert.equal(ui.seed.value, '0');
  ui.calls[1].resolve('a different monster');
  await pending;
  assert.match(ui.status.textContent, /Prompt seed 0/);
});

test('explicitly re-entering the same seed replays it instead of rerolling', async () => {
  const ui = setup();
  let pending = ui.controls.randomize();
  ui.calls[0].resolve('prompt from seed 42');
  await pending;
  ui.seed.input('42');
  assert.match(ui.status.textContent, /uses the displayed seed/);
  pending = ui.controls.randomize();
  assert.equal(ui.calls[1].args[0], 42n);
  assert.equal(ui.seed.value, '42');
  ui.calls[1].resolve('prompt from seed 42');
  await pending;
  pending = ui.controls.randomize();
  assert.equal(ui.calls[2].args[0], 43n, 'unchanged repeated clicks still reroll');
  ui.calls[2].resolve('prompt from seed 43');
  await pending;
});

test('changing summoner profile or difficulty uses the displayed seed before rerolling again', async () => {
  for (const [field, value] of [['profile', 'ocean'], ['difficulty', '3']]) {
    const ui = setup();
    let pending = ui.controls.randomize();
    ui.calls[0].resolve('original suggestion');
    await pending;
    ui[field].input(value);
    pending = ui.controls.randomize();
    assert.equal(ui.calls[1].args[0], 42n);
    assert.equal(ui.seed.value, '42');
    assert.equal(ui.calls[1].args[1], field === 'difficulty' ? 3 : 2);
    assert.equal(ui.calls[1].args[2], field === 'profile' ? 'ocean' : 'default');
    ui.calls[1].resolve('new option suggestion');
    await pending;
    pending = ui.controls.randomize();
    assert.equal(ui.calls[2].args[0], 43n);
    ui.calls[2].resolve('rerolled suggestion');
    await pending;
  }
});

test('manual prompt edits, seed edits, profile changes and difficulty changes discard stale responses', async () => {
  for (const [field, value] of [['prompt', 'my own text'], ['seed', '99'], ['profile', 'ocean'], ['difficulty', '3']]) {
    const ui = setup();
    const pending = ui.controls.randomize();
    ui[field].input(value);
    ui.calls[0].resolve('stale suggestion');
    await pending;
    assert.equal(ui.prompt.value, field === 'prompt' ? value : 'arbitrary text');
    assert.equal(ui.button.disabled, false);
  }
});

test('older response/error cannot overwrite a newer suggestion or clear its busy state', async () => {
  const ui = setup();
  const first = ui.controls.randomize();
  ui.prompt.input('edited');
  const second = ui.controls.randomize();
  ui.calls[0].reject(new Error('stale network failure'));
  await first;
  assert.equal(ui.button.disabled, true);
  assert.doesNotMatch(ui.status.textContent, /stale network failure/);
  ui.calls[1].resolve('new suggestion');
  await second;
  assert.equal(ui.prompt.value, 'new suggestion');
  assert.equal(ui.button.disabled, false);
});

test('generation disables suggestions and invalidates their late completion', async () => {
  const ui = setup();
  const pending = ui.controls.randomize();
  ui.controls.setDisabled(true);
  ui.calls[0].resolve('late suggestion');
  await pending;
  await ui.controls.randomize();
  assert.equal(ui.prompt.value, 'arbitrary text');
  assert.equal(ui.calls.length, 1);
  assert.equal(ui.button.disabled, true);
  ui.controls.setDisabled(false);
  assert.equal(ui.button.disabled, false);
});

test('failed requests preserve manual text and allow retry with a different visible seed', async () => {
  const ui = setup();
  const pending = ui.controls.randomize();
  ui.calls[0].reject(new Error('server unreachable'));
  await pending;
  assert.equal(ui.prompt.value, 'arbitrary text');
  assert.match(ui.status.textContent, /server unreachable/);
  assert.equal(ui.button.disabled, false);
  const retry = ui.controls.randomize();
  assert.equal(ui.seed.value, '43');
  ui.calls[1].resolve('retry succeeds');
  await retry;
  assert.equal(ui.prompt.value, 'retry succeeds');
});

test('invalid seed/difficulty rejects before any request; disposal blocks late writes', async () => {
  for (const seed of ['', '-1', '1e3', '1.5', '18446744073709551616', ' 1']) {
    const ui = setup(); ui.seed.value = seed;
    await ui.controls.randomize();
    assert.equal(ui.calls.length, 0);
    assert.match(ui.status.textContent, /Seed/);
  }
  const ui = setup();
  ui.difficulty.value = '257';
  await ui.controls.randomize();
  assert.equal(ui.calls.length, 0);
  ui.difficulty.value = '2';
  const pending = ui.controls.randomize();
  ui.controls.dispose();
  ui.calls[0].resolve('discarded after pagehide');
  await pending;
  assert.equal(ui.prompt.value, 'arbitrary text');
  assert.equal(ui.button.disabled, true);
});
