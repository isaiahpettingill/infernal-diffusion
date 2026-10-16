// Shared by the native arena and the standalone WASM demo. Prompt composition
// lives in Rust; this controller only handles seeds and asynchronous UI state.
globalThis.InfernalPromptControls = class InfernalPromptControls {
  static seed(value) {
    if (typeof value !== 'string' || !/^\d+$/.test(value)) throw new TypeError('Seed must be an unsigned decimal integer');
    const seed = BigInt(value);
    if (seed > 18446744073709551615n) throw new RangeError('Seed must fit an unsigned 64-bit integer');
    return seed;
  }

  constructor({ prompt, seed, difficulty, profile, button, status, suggest, disabled = false }) {
    Object.assign(this, { prompt, seed, difficulty, profile, button, status, suggest });
    this.disabled = disabled;
    this.pending = null;
    this.lastSeed = null;
    this.listeners = [];
    for (const input of [prompt, seed, difficulty, profile]) {
      const listener = () => {
        // Explicit option edits restart from the displayed seed, even when the
        // user re-enters the same value to replay a previous suggestion.
        if (input !== prompt) this.lastSeed = null;
        this.invalidate('Inputs changed; choose Random prompt again for a new suggestion.');
        if (input !== prompt) this.status.textContent = 'Options changed. Next Random prompt uses the displayed seed.';
      };
      input.addEventListener('input', listener);
      input.addEventListener('change', listener);
      this.listeners.push([input, 'input', listener], [input, 'change', listener]);
    }
    const click = () => this.randomize();
    button.addEventListener('click', click);
    this.listeners.push([button, 'click', click]);
    this.refresh();
  }

  refresh() { this.button.disabled = this.disabled || this.pending !== null; }
  invalidate(message = '') {
    if (this.pending && message) this.status.textContent = message;
    this.pending = null;
    this.refresh();
  }
  setDisabled(value) {
    this.disabled = value;
    if (value) this.invalidate('Suggestion discarded while generation is running.');
    this.refresh();
  }
  snapshot() { return [this.prompt.value, this.seed.value, this.difficulty.value, this.profile.value]; }

  async randomize() {
    if (this.disabled || this.pending) return;
    let seed, difficulty;
    try {
      seed = InfernalPromptControls.seed(this.seed.value);
      difficulty = Number(this.difficulty.value);
      if (!Number.isInteger(difficulty) || difficulty < 1 || difficulty > 3) throw new RangeError('Difficulty must be 1, 2, or 3');
    } catch (error) { this.status.textContent = error.message; return; }
    // The first click after an option edit uses the displayed seed. Repeats advance it;
    // wrap at u64::MAX rather than silently rounding through a JS Number.
    if (seed === this.lastSeed) seed = (seed + 1n) & 18446744073709551615n;
    this.lastSeed = seed;
    this.seed.value = seed.toString();
    const pending = { snapshot: this.snapshot() };
    this.pending = pending;
    this.refresh();
    this.status.textContent = `Choosing a ${this.profile.value} prompt with seed ${seed}…`;
    try {
      const result = await this.suggest(seed, difficulty, this.profile.value);
      if (this.pending !== pending || this.disabled || this.snapshot().some((value, i) => value !== pending.snapshot[i])) return;
      if (typeof result !== 'string' || !result.trim()) throw new Error('Generator returned an empty prompt');
      this.prompt.value = result;
      this.status.textContent = `Prompt seed ${seed} · edit the text or generate a monster. Next random click advances the seed.`;
    } catch (error) {
      if (this.pending === pending) this.status.textContent = `Random prompt failed: ${error.message}`;
    } finally {
      if (this.pending === pending) { this.pending = null; this.refresh(); }
    }
  }

  dispose() {
    this.setDisabled(true);
    for (const [element, type, listener] of this.listeners) element.removeEventListener(type, listener);
    this.listeners = [];
  }
};
