// This module and all synchronous generation run off the browser's main thread.
let api;
let initialization;
let info;
let threadCount = 1;
async function ready(progress) {
  if (!initialization) initialization = (async () => {
    if (typeof WebAssembly === 'undefined') throw new Error('WebAssembly is unavailable in this browser');
    progress('loading-wasm', 0, 1);
    api = await import('./pkg/infernal_diffusion.js');
    if (typeof api.initThreadPool === 'function' && (!self.crossOriginIsolated || typeof SharedArrayBuffer === 'undefined')) {
      throw new Error('This optional threaded build requires cross-origin isolation (COOP/COEP). Use the default portable build on ordinary static hosts');
    }
    await api.default();
    if (typeof api.initialize_model !== 'function' || typeof api.generator_info !== 'function') {
      throw new Error('This WASM package has no bundled BERT encoder; rebuild with tools/build_wasm.sh');
    }
    info = api.generator_info();
    if (!info.bundledModel) throw new Error('This WASM package was built without the bundled BERT model');
    if (info.threaded) {
      if (!self.crossOriginIsolated || typeof SharedArrayBuffer === 'undefined') {
        throw new Error('This optional threaded build requires cross-origin isolation (COOP/COEP). Use the default portable build on ordinary static hosts');
      }
      threadCount = Math.min(4, Math.max(2, Math.floor(navigator.hardwareConcurrency || 2)));
      let timeout;
      try {
        await Promise.race([
          api.initThreadPool(threadCount),
          new Promise((_, reject) => {
            timeout = setTimeout(() => reject(new Error('Rayon worker pool initialization timed out; check worker URLs and COOP/COEP headers')), 45000);
          }),
        ]);
      } finally { clearTimeout(timeout); }
    }
    progress('loading-wasm', 1, 1);
    progress('initializing-model', 0, 1);
    const parser = api.initialize_model();
    if (parser !== 'candle-bert-prototypes-v2') throw new Error('Unexpected bundled BERT parser');
    progress('initializing-model', 1, 1);
  })();
  return initialization;
}

function serializeError(error, code) {
  return { name: error?.name || 'Error', message: String(error?.message || error), code };
}
function transfers(result) {
  const buffers = new Set();
  for (const pkg of result.packages) {
    for (const atlas of [pkg.sprites, pkg.emission, pkg.projectiles]) {
      if (atlas?.rgba instanceof Uint8Array) buffers.add(atlas.rgba.buffer);
    }
  }
  return [...buffers];
}
// Serialize initialization, suggestions, and generation on one explicit queue.
// Each request handles its own error so a bad request cannot poison later work.
let queue = Promise.resolve();
self.onmessage = ({ data }) => {
  queue = queue.then(() => handleRequest(data)).catch(error => {
    if (Number.isSafeInteger(data?.id)) self.postMessage({ id: data.id, type: 'error', error: serializeError(error, 'WORKER_FAILED') });
  });
  return queue;
};
async function handleRequest(data) {
  const { id, type, prompt, seed, difficulty, prompt_profile = 'default', profile = 'v1' } = data || {};
  if (!Number.isSafeInteger(id)) return;
  if (type !== 'generate' && type !== 'generate_hybrid' && type !== 'suggest' && type !== 'ready') {
    self.postMessage({ id, type: 'error', error: serializeError(new Error('Unknown worker request'), 'INVALID_REQUEST') });
    return;
  }
  const progress = (phase, completed = 0, total = 1) => {
    if (data.progress === true) self.postMessage({ id, type: 'progress', progress: { phase, completed, total } });
  };
  try {
    await ready(progress);
  } catch (error) {
    self.postMessage({ id, type: 'error', error: serializeError(new Error(`Unable to load the generator. Serve the complete bundled pkg directory over localhost or HTTPS using a current browser. ${error?.message || error}`), 'WASM_UNAVAILABLE') });
    return;
  }
  try {
    if (type === 'ready') {
      self.postMessage({ id, type: 'ready', threadCount, ...info });
      return;
    }
    if (typeof seed !== 'bigint' || seed < 0n || seed > 18446744073709551615n) throw new RangeError('Seed must be an unsigned 64-bit bigint');
    if (type === 'suggest') {
      if (!Number.isInteger(difficulty) || difficulty < 1 || difficulty > 3) throw new RangeError('Difficulty must be 1, 2, or 3');
      if (typeof prompt_profile !== 'string' || !['default', 'necromancer', 'shaman', 'lunatic', 'boss', 'ocean'].includes(prompt_profile.toLowerCase())) throw new RangeError('Unknown summoner prompt profile');
      if (typeof api.suggest_prompt !== 'function') throw new Error('This WASM package does not support random prompts; rebuild the generator');
      const result = api.suggest_prompt(seed, difficulty, prompt_profile.toLowerCase());
      self.postMessage({ id, type: 'result', result });
      return;
    }
    if (typeof prompt !== 'string' || !prompt.trim()) throw new TypeError('Prompt must be a nonempty string');
    if (type === 'generate' && new TextEncoder().encode(prompt).length > 4096) throw new RangeError('Prompt must be at most 4096 UTF-8 bytes');
    if (profile !== 'v1' && profile !== 'v2') throw new RangeError('Profile must be v1 or v2');
    // Keep V1 compatible with previously built WASM packages. Never silently
    // downgrade an explicit V2 request when paired with an older binary.
    if (type !== 'generate_hybrid' && profile === 'v2' && typeof api.generate_monster_with_profile !== 'function') {
      throw new Error('This WASM package does not support V2; rebuild the generator');
    }
    let result;
    if (type === 'generate_hybrid') {
      if (profile !== 'v2') throw new RangeError('Structured hybrid generation requires V2');
      if (typeof api.generate_hybrid_json !== 'function') throw new Error('This WASM package does not support structured hybrids; rebuild the generator');
      result = data.progress === true && typeof api.generate_hybrid_json_progress === 'function'
        ? api.generate_hybrid_json_progress(prompt, seed, progress)
        : api.generate_hybrid_json(prompt, seed);
    } else {
      result = data.progress === true && typeof api.generate_monster_with_profile_progress === 'function'
        ? api.generate_monster_with_profile_progress(prompt, seed, profile, progress)
        : profile === 'v1' ? api.generate_monster(prompt, seed)
          : api.generate_monster_with_profile(prompt, seed, profile);
    }
    self.postMessage({ id, type: 'result', result }, transfers(result));
  } catch (error) {
    self.postMessage({ id, type: 'error', error: serializeError(error, type === 'suggest' ? 'SUGGESTION_FAILED' : 'GENERATION_FAILED') });
  }
}
