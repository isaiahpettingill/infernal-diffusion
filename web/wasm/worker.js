// This module and all synchronous generation run off the browser's main thread.
let api;
let initialization;
let threadCount = 0;
async function ready() {
  if (!initialization) initialization = (async () => {
    if (typeof WebAssembly === 'undefined') throw new Error('WebAssembly is unavailable in this browser');
    if (!self.crossOriginIsolated || typeof SharedArrayBuffer === 'undefined') {
      throw new Error('Threaded generation requires cross-origin isolation (COOP: same-origin and COEP: require-corp), SharedArrayBuffer, and a secure context such as localhost or HTTPS');
    }
    api = await import('./pkg/infernal_diffusion.js');
    // Instantiate the real binary, not an MVP-only capability probe. This catches
    // unsupported features as well as missing files and incorrect server MIME types.
    await api.default();
    threadCount = Math.min(4, Math.max(2, Math.floor(navigator.hardwareConcurrency || 2)));
    let timeout;
    try {
      await Promise.race([
        api.initThreadPool(threadCount),
        new Promise((_, reject) => {
          timeout = setTimeout(() => reject(new Error('Rayon worker pool initialization timed out; check worker module URLs, CSP worker-src permissions, and COOP/COEP headers')), 45000);
        }),
      ]);
    } finally { clearTimeout(timeout); }
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
self.onmessage = async ({ data }) => {
  const { id, type, prompt, seed, profile = 'v1' } = data || {};
  if (!Number.isSafeInteger(id)) return;
  if (type !== 'generate' && type !== 'generate_hybrid' && type !== 'ready') {
    self.postMessage({ id, type: 'error', error: serializeError(new Error('Unknown worker request'), 'INVALID_REQUEST') });
    return;
  }
  try {
    await ready();
  } catch (error) {
    self.postMessage({ id, type: 'error', error: serializeError(new Error(`Unable to load the generator. Use a current browser supporting this modern threaded WebAssembly build, and serve the built pkg directory with COOP/COEP headers over localhost or HTTPS. ${error?.message || error}`), 'WASM_UNAVAILABLE') });
    return;
  }
  try {
    if (type === 'ready') {
      self.postMessage({ id, type: 'ready', threadCount });
      return;
    }
    if (typeof prompt !== 'string' || !prompt.trim()) throw new TypeError('Prompt must be a nonempty string');
    if (typeof seed !== 'bigint' || seed < 0n || seed > 18446744073709551615n) throw new RangeError('Seed must be an unsigned 64-bit bigint');
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
      result = api.generate_hybrid_json(prompt, seed);
    } else {
      result = profile === 'v1' ? api.generate_monster(prompt, seed)
        : api.generate_monster_with_profile(prompt, seed, profile);
    }
    self.postMessage({ id, type: 'result', result }, transfers(result));
  } catch (error) {
    self.postMessage({ id, type: 'error', error: serializeError(error, 'GENERATION_FAILED') });
  }
};
