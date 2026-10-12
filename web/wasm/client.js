/** Async, request-correlated interface to a dedicated generator worker.
 * Cancellation terminates the worker, rejecting ALL outstanding work; the next
 * request creates a fresh worker. A synchronous WASM call cannot poll messages.
 */
export class MonsterGenerator {
  constructor({ workerUrl = new URL('./worker.js', import.meta.url) } = {}) {
    this.workerUrl = workerUrl;
    this.worker = null;
    this.pending = new Map();
    this.nextId = 1;
    this.closed = false;
  }

  _ensureWorker() {
    if (this.closed) throw new Error('Generator has been disposed');
    if (this.worker) return;
    if (typeof Worker === 'undefined' || typeof WebAssembly === 'undefined') {
      const error = new Error('This browser does not support Web Workers and WebAssembly');
      error.code = 'WASM_UNAVAILABLE';
      throw error;
    }
    const worker = new Worker(this.workerUrl, { type: 'module', name: 'infernal-generator' });
    this.worker = worker;
    worker.onmessage = ({ data }) => {
      if (this.worker !== worker) return;
      const entry = this.pending.get(data.id);
      if (!entry) return;
      this.pending.delete(data.id);
      entry.cleanup();
      if (data.type === 'error') {
        const error = new Error(data.error.message);
        error.name = data.error.name;
        error.code = data.error.code;
        entry.reject(error);
        if (error.code === 'WASM_UNAVAILABLE') this._reset(error);
      } else entry.resolve(data.type === 'ready' ? { threadCount: data.threadCount } : data.result);
    };
    worker.onerror = (event) => {
      event.preventDefault();
      if (this.worker !== worker) return;
      this._reset(new Error(`Generator worker failed: ${event.message || 'could not load worker module'}`));
    };
    worker.onmessageerror = () => {
      if (this.worker === worker) this._reset(new Error('Could not decode the generator response'));
    };
  }

  _request(type, payload, signal) {
    return new Promise((resolve, reject) => {
      if (signal?.aborted) return reject(new DOMException('Generation cancelled', 'AbortError'));
      this._ensureWorker();
      const id = this.nextId++;
      const abort = () => this.cancel();
      const cleanup = () => signal?.removeEventListener('abort', abort);
      this.pending.set(id, { resolve, reject, cleanup });
      signal?.addEventListener('abort', abort, { once: true });
      try { this.worker.postMessage({ id, type, ...payload }); }
      catch (error) { this.pending.delete(id); cleanup(); reject(error); }
    });
  }

  /** Resolves only after the actual binary has instantiated successfully. */
  ready({ signal } = {}) { return this._request('ready', {}, signal); }

  /** seed is bigint, decimal text, or a safe integer. Returns full metadata and RGBA atlases. */
  generate(prompt, seed = 0n, { signal } = {}) {
    try {
      if (typeof seed === 'number' && !Number.isSafeInteger(seed)) throw new RangeError('Numeric seed must be a safe integer; use bigint or decimal text for larger seeds');
      if (!['bigint', 'number', 'string'].includes(typeof seed) || (typeof seed === 'string' && !/^\d+$/.test(seed))) throw new TypeError('Seed must be an unsigned integer');
      seed = BigInt(seed);
      if (seed < 0n || seed > 18446744073709551615n) throw new RangeError('Seed must fit an unsigned 64-bit integer');
      if (typeof prompt !== 'string' || !prompt.trim()) throw new TypeError('Prompt must be a nonempty string');
    } catch (error) { return Promise.reject(error); }
    return this._request('generate', { prompt, seed }, signal);
  }

  _reset(error) {
    // Terminating this dedicated worker also terminates its descendant Rayon
    // workers. A cancel message cannot interrupt synchronous WASM execution.
    this.worker?.terminate();
    this.worker = null;
    for (const { reject, cleanup } of this.pending.values()) { cleanup(); reject(error); }
    this.pending.clear();
  }
  cancel() { this._reset(new DOMException('Generation cancelled; worker restarted on next request', 'AbortError')); }
  dispose() { this.closed = true; this._reset(new Error('Generator has been disposed')); }
}
