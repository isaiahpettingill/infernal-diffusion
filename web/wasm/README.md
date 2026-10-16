# Browser WebAssembly generator

The default build runs the real Candle BERT encoder and full Rust monster generator in **one dedicated module Web Worker**. It embeds `google/bert_uncased_L-2_H-128_A-2` from `models/bert-mini/` inside the WASM binary, including weights, tokenizer and configuration. There is no model download, generation service, JavaScript replacement parser, WASI runtime or native executable at runtime. After loading the local bundle, generation works offline.

The portable build needs Web Workers, WebAssembly SIMD and bulk memory in a current browser. **SharedArrayBuffer and cross-origin isolation are not required.** An optional threaded build retains the real 2–4-worker Rayon pool for hosts that support isolation.

## Build and run

From the repository root, install the `wasm32-unknown-unknown` Rust target, wasm-bindgen-cli **0.2.129** (matching Cargo.lock), and Binaryen **130**, then build:

```sh
rustup target add wasm32-unknown-unknown
./tools/build_wasm.sh
cd web/wasm
python3 server.py
```

Open <http://127.0.0.1:8080/>. The script writes `pkg/infernal_diffusion.js`, `pkg/infernal_diffusion_bg.wasm`, model licensing notices, `pkg/build-info.json`, and `bundle-manifest.json` with artifact sizes and SHA-256 hashes. Deploy `client.js`, `worker.js`, the complete `pkg/` directory, and any application UI together. Keep relative URLs intact. The model weights and tokenizer do not need separate hosting. The bundled model is Apache-2.0 licensed; retain its supplied license and notice.

Use HTTP(S), not `file://`, for module workers and WASM fetches. Production hosts should use HTTPS and serve `.wasm` as `application/wasm`. An ordinary static server, including `python3 -m http.server`, can serve the default portable build. The supplied server additionally sets COOP/COEP headers so it can serve either distribution. If your site has a content security policy, allow its same-origin worker/module/WASM resources as appropriate.

Enter a prompt and unsigned 64-bit decimal seed, select V1 (default) or V2, then Generate. The live heartbeat demonstrates that generation and BERT inference stay off the main thread. Sprite, emission and optional projectile atlases render to canvases with full expandable metadata. Large atlases may still cost main-thread time when painted.

## Integrate

```js
import { MonsterGenerator } from './client.js';

const generator = new MonsterGenerator();
const onProgress = ({ phase, completed, total }) => {
  console.log(phase, completed, total);
};
const info = await generator.ready({ onProgress });
// Portable: threadCount: 1, threaded: false, bundledModel: true
// model: 'google/bert_uncased_L-2_H-128_A-2'
// parser: 'candle-bert-prototypes-v2'

const controller = new AbortController();
const result = await generator.generate('stormtrooper with a laser blaster', 42n, {
  profile: 'v1', // default; explicitly choose 'v2' for V2
  signal: controller.signal,
  onProgress,
});
// result.packages: [{ path, monster, sprites, emission, projectiles }]
// atlas: { width, height, rgba: Uint8Array }; projectiles may be null
// monster: complete serialized proto metadata, including generation.parser
// Use bigint or decimal strings for seeds above Number.MAX_SAFE_INTEGER.

generator.dispose();
```

`ready({ signal, onProgress })` resolves after the actual WASM binary is instantiated and the bundled BERT encoder completes a real forward pass. The worker caches the model; repeated readiness and generation requests reuse it. Its response includes `threadCount`, `threaded`, `bundledModel`, `model`, `parser`, and the model SHA-256. There is no silent vocabulary-only fallback if BERT initialization fails. An older vocabulary-only WASM binary must be rebuilt.

`onProgress` receives `{ phase, completed?, total? }`. Initialization phases are `loading-wasm` and `initializing-model`. Prompt generation emits `parsing`, `building`, `rendering`, `packaging`, and `complete`. Counts describe work in the current phase, not a global percentage; phases can repeat for companion packages. Structured hybrid recipes bypass prompt parsing and retain their dedicated generation API. Initialization events occur only when that worker first initializes. Progress is optional; omitting the callback does not change generation results. Exceptions in a UI progress callback do not orphan its request.

Each request has a unique numeric ID and runs through one ordered queue. Generation preserves full metadata and transfers atlas ArrayBuffers to the main thread without copying. uint64 metadata fields may be BigInt; use a replacer when JSON-stringifying. Both generation profiles preserve the complete geometry, directional renderer, animations, physics, attacks, colliders and companion packages. Browser output is raw RGBA rather than PNG to avoid encoding and decoding overhead.

Client validation rejects invalid seeds, empty prompts, prompts over 4096 UTF-8 bytes, invalid difficulties and unknown profiles before starting a worker. Initialization errors use `code: 'WASM_UNAVAILABLE'`; generation errors use `GENERATION_FAILED`; suggestion errors use `SUGGESTION_FAILED`. A bad generation request does not poison the queue. Worker crashes or initialization failures reject pending work and allow a later call to start a fresh worker. The default request timeout is ten minutes; pass `{ timeoutMs }` to the constructor to change it. A timeout uses `code: 'TIMEOUT'` and terminates the worker and pending work.

### Cancellation

`generator.cancel()` or aborting a pending request terminates the worker and rejects **all** outstanding requests with `AbortError`, including readiness and suggestions. Optional descendant Rayon workers also terminate. The next request lazily creates a new worker, reloads WASM, and initializes a fresh model. Synchronous WASM cannot consume a cancellation message while running, so cancellation uses worker termination rather than polling. Use separate generator instances for independent cancellation. `dispose()` is permanent.

### Prompt suggestions and hybrids

```js
const prompt = await generator.suggestPrompt('18446744073709551615', 2, {
  promptProfile: 'ocean', // case-insensitive; defaults to 'default'
});
// Editable text only: no monster is rendered yet.
const monster = await generator.generate(prompt, '18446744073709551615', { profile: 'v2' });

const hybrid = await generator.generateHybrid({
  schema_version: 1,
  upper_body: 'HUMANOID', lower_body: 'HORSE', head: 'HUMANOID',
  lower_limb_count: 4, upper_arm_count: 2,
}, 42n, { onProgress });
// Structured hybrids always use V2 and do not reinterpret the recipe as prose.
```

Summoner profiles are Default, Necromancer, Shaman, Lunatic, Boss and Ocean. Boss always uses hard modifiers; Lunatic mixes tiers. Difficulty remains part of the deterministic input for both. Suggestions use Rust's shared vocabulary; summoner selection is independent of generation V1/V2. Arbitrary hand-written prompts remain supported. See [browser prompt APIs](../../docs/browser-prompts.md) and [generation profiles](../../docs/generation-profiles.md).

In the demo, the first Random prompt click uses the displayed seed. Repeated clicks with unchanged options advance the visible seed, wrapping from u64::MAX to zero. Editing the seed, difficulty or summoner profile resets reroll history, including re-entering the same seed. Random prompt never renders automatically. Manual prompt edits, presets, option changes and generation invalidate outstanding suggestions; stale responses cannot overwrite newer text. Failures leave the text intact and allow retry.

### Low-level worker protocol

Requests are `{ id, type, ...payload, progress?: true }`:

- `ready`: no payload
- `generate`: `prompt`, `seed: bigint`, optional `profile: 'v1' | 'v2'`
- `generate_hybrid`: JSON-text `prompt`, `seed: bigint`, `profile: 'v2'`
- `suggest`: `seed: bigint`, `difficulty: 1 | 2 | 3`, optional `prompt_profile`

Replies are `{ id, type: 'ready', ...info }`, `{ id, type: 'result', result }`, or `{ id, type: 'error', error: { name, message, code } }`. A request with **exactly** `progress: true` can also receive `{ id, type: 'progress', progress: { phase, completed, total } }` before its terminal reply. Other requests receive no progress messages. Text suggestions have no atlas transfer list.

Direct WASM exports include `initialize_model()`, `generator_info()`, `generate_monster(prompt, seed)`, `generate_monster_with_profile(prompt, seed, profile)`, `generate_monster_with_profile_progress(prompt, seed, profile, callback)`, `generate_hybrid_json(recipe, seed)`, `generate_hybrid_json_progress(recipe, seed, callback)`, and `suggest_prompt(seed, difficulty, prompt_profile)`. A direct progress callback receives `(phase, completed, total)`. Use the client for u64 validation, cancellation, request correlation and off-main-thread execution.

## Optional threaded build

```sh
rustup toolchain install nightly-2026-10-12 --profile minimal --component rust-src
WASM_THREADED=1 ./tools/build_wasm.sh
```

This rebuilds the standard library with atomics, enables `wasm-threads,bert`, and uses shared linear memory with a 2 GiB maximum. Serve it in a secure context with `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp`. Keep generated `pkg/snippets/` modules. Threaded readiness initializes a real Rayon pool with 2–4 threads, based on hardware concurrency; it reports `threaded: true`. Missing isolation produces an actionable error instead of hanging. Pool initialization times out after 45 seconds. Choose the default portable build for hosts that cannot supply isolation headers.

Both distributions use the `wasm-release` profile: opt-level 3, fat LTO, one codegen unit, aborting panics and stripped debug information, followed by `wasm-opt --all-features -O3`. The default Rust target features are `simd128,bulk-memory`; optional threading also enables atomics and the advanced proposal flags recorded in build-info.json. `--all-features` allows the optimizer to process supported proposals; it is not a promise that every proposal appears in the result. `pkg/build-info.json` records the actual build settings.

No fast-math or explicit nondeterministic relaxed floating-point intrinsics are used. Tests verify same-build/same-browser metadata and every RGBA atlas hash. Cross-browser, cross-architecture, compiler-version and portable-versus-threaded bit equality are not promised; pin and validate the binary for replay systems.

## Tests

Dependency-free client, worker and controls checks, from the repository root:

```sh
node --test web/wasm/tests/profiles.node.mjs web/wasm/tests/suggestions.node.mjs \
  web/wasm/tests/prompt-controls.node.mjs web/wasm/tests/worker-contract.node.mjs
```

After building `pkg/`, run the real browser suite:

```sh
cd web/wasm
npm ci
npx playwright install chromium
npm test
# When testing the opt-in threaded package:
WASM_THREADED=1 npm test
```

Playwright starts the server automatically. `PLAYWRIGHT_CHROMIUM_EXECUTABLE=/path/to/chromium` selects an installed browser. Tests do not silently skip a missing build. The portable suite removes isolation headers, verifies SharedArrayBuffer is unavailable, blocks off-origin requests, initializes BERT, then switches the browser offline and generates a real monster. Other tests cover exact u64 seeds, V1/V2 world-size metadata, all-atlas SHA-256 replay, real parser metadata, progress, main-thread heartbeat, queued and model-loading cancellation/restart, recoverable errors, suggestions, hybrids and demo rendering. The threaded matrix preserves isolation-error and pool checks.

For an additional real-WASM Node regression, run `node web/wasm/tests/explicit-counts.actual-wasm.node.mjs` from the repository root. It checks BERT parser metadata and explicit head/arm counts through full V2 rendering; portable builds run directly, and threaded builds use a Node worker_threads bridge for Rayon. This does not replace browser tests.

For manual browser checks, open `/tests/browser-check.html` on the supplied server and click **Run browser checks**. It reports runtime/model readiness, full stormtrooper metadata, progress phases, atlas hashes, heartbeat activity and cancellation/restart.
