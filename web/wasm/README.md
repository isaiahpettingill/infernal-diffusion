# Browser WebAssembly generator

This standalone static-site example runs Rust generation and vocabulary parsing in a dedicated module Web Worker. It initializes a real Rayon worker pool (2–4 threads, based on hardware concurrency). No generation HTTP endpoint, native BERT, or WASI runtime is used. SharedArrayBuffer and cross-origin isolation are required. Use a current browser supporting the modern WASM features selected by the build: threads/atomics, SIMD and relaxed SIMD, tail calls, bulk memory, multivalue, nontrapping float-to-int, sign extension, extended constants, and typed function references when supported by the Rust toolchain. The client instantiates the actual binary to detect unavailable features, rather than relying on an MVP-only probe.

## Run

1. Build the browser WASM package using the repository's WASM build instructions. The wasm-bindgen `--target web` output must live in `web/wasm/pkg/`, including `infernal_diffusion.js` and `infernal_diffusion_bg.wasm`.
2. From `web/wasm`, run `python3 server.py`.
3. Open <http://127.0.0.1:8080/>. Do not use `file://`; module workers and WASM fetches require HTTP(S). Production servers must use HTTPS, serve `.wasm` as `application/wasm`, and send `Cross-Origin-Opener-Policy: same-origin` plus `Cross-Origin-Embedder-Policy: require-corp`. The supplied local server sets these headers on every response, including worker scripts, and serves localhost as a secure context. A plain `python -m http.server` is insufficient for this threaded build. Keep the generated `pkg/snippets/` directory; it contains Rayon worker modules.

Enter a prompt and unsigned 64-bit decimal seed, then Generate. The live heartbeat shows the main thread remains responsive while Rust runs. Sprite, emission, and optional projectile atlases render to canvases, followed by expandable full metadata. Large atlases can briefly cost main-thread time while being painted; generation itself stays in the worker.

## Integrate

```js
import { MonsterGenerator } from './client.js';
const generator = new MonsterGenerator();
const { threadCount } = await generator.ready(); // actual binary + Rayon pool ready
const controller = new AbortController();
const result = await generator.generate('stormtrooper with a laser blaster', 42n, {
  signal: controller.signal,
});
// result.packages: [{path, monster, sprites, emission, projectiles}]
// atlas: {width, height, rgba: Uint8Array}; projectiles may be null
// monster: complete serialized proto metadata, not an abbreviated demo schema
// Use bigint or decimal strings for seeds larger than Number.MAX_SAFE_INTEGER.
generator.dispose();
```

Each request has a unique numeric ID. Errors reject the matching promise; initialization errors carry `code: 'WASM_UNAVAILABLE'`. Invalid generator requests carry `code: 'GENERATION_FAILED'`. Unexpected worker crashes reject all pending work, and a later request starts a fresh worker. RGBA ArrayBuffers are transferred rather than copied; the main thread owns returned buffers. Metadata is structured-cloned without trimming fields. uint64 fields may be BigInt; stringify with a replacer when needed.

`generator.cancel()` or aborting a pending request terminates the worker and rejects **all** outstanding requests with `AbortError`. Termination also shuts down descendant Rayon workers; no shared pool outlives its owning generator worker. The next request lazily starts a new worker, reloads WASM, and initializes a fresh pool. Synchronous WASM cannot receive cancellation messages while generating. Use separate generator instances if jobs need independent cancellation. Requests on one instance execute sequentially. `dispose()` is permanent; create another instance afterward.

The low-level worker protocol accepts `{id, type: 'ready'}` or `{id, type: 'generate', prompt, seed: bigint}`. Replies are `{id, type: 'ready', threadCount}`, `{id, type: 'result', result}`, or `{id, type: 'error', error: {name, message, code}}`. Generation errors do not intentionally terminate the worker. Extremely large/complex prompts may exhaust browser memory; clients should constrain untrusted workloads and use termination for time limits.

## Browser tests

Build `pkg/` first, then from this directory:

```sh
npm ci
npx playwright install chromium
npm test
```

Playwright starts the static server automatically. Tests use the real WASM worker, covering deterministic metadata and atlas bytes, transferable typed pixels, complete stormtrooper metadata including `RANDOM_DIRECTION`, main-thread heartbeat, validation, queued request cancellation/restart, disposal, invalid-binary and missing-isolation error reporting, pool initialization, and demo canvas rendering. Tests have a generous timeout because full procedural atlases are generated, including pool startup. They do not silently skip when `pkg/` is absent: build it first.

To use an already-installed modern Chromium, set `PLAYWRIGHT_CHROMIUM_EXECUTABLE=/path/to/chromium` when running `npm test`.

## Reproducible optimized build

Install Rust nightly-2026-10-12 with rust-src, wasm-bindgen-cli **0.2.129** (matching Cargo.lock), and Binaryen **130** from their official releases, then run:

```sh
rustup toolchain install nightly-2026-10-12 --profile minimal --component rust-src
./tools/build_wasm.sh
```

The `wasm-release` profile uses opt-level 3, fat LTO, one codegen unit, aborting panics and stripped debug information. The script rebuilds `std` with atomics instead of linking a non-thread-safe precompiled standard library. It targets **wasm32-unknown-unknown**, enables **atomics, bulk-memory, simd128, relaxed-simd, tail-call, multivalue, mutable-globals, nontrapping-fptoint, sign-ext, extended-const and reference-types**, and finishes with **wasm-opt --all-features -O3**. `pkg/build-info.json` records the build configuration. The maximum shared linear memory is 2 GiB; this is a ceiling, not an up-front allocation.

These are enabled compiler/optimizer capabilities, not a promise that every proposal appears in every binary. Threads are real: wasm-bindgen-rayon starts a shared-memory pool and the unchanged directional renderer uses its parallel iterator. Rust/LLVM chooses whether to emit tail calls, multi-value operations, relaxed SIMD, etc. Typed function references have **no separate supported rustc target-feature** in this pinned toolchain; reference-types is enabled but is not the same proposal. We do not pass a fictional `typed-function-references` flag. Binaryen accepts all supported proposals for validation/optimization; that flag does not turn a wasm32 Rust program into memory64, a GC-language runtime or an exception-unwinding build.

The vocabulary parser is the existing deterministic parser, preserving compound species, materials, anatomy modifiers, attacks, mounts, minions and unknown-creature handling. Native builds retain default BERT. Geometry recipes (including stormtrooper armor/helmet and random-direction firing), 3D rasterization, palettes, directional animation, physics, colliders, gameplay and complete companion packages are shared with native generation. No atlas-size, animation-count or geometry shortcuts are introduced for the browser. Browser images are returned as RGBA rather than PNG to avoid unnecessary encode/decode work.

Seeded rendering does not use fast-math or explicit nondeterministic relaxed floating-point intrinsics. Same-build/same-browser reproducibility is checked by hashing every atlas and comparing full metadata. Relaxed SIMD is enabled as requested, so cross-browser/cross-architecture bit equality is not promised; Rust floating-point behavior and future optimizer changes also require revalidation. Pin the binary for replay systems.

The GitHub Actions WASM job runs parser tests, compiles/optimizes this target, runs real Chromium worker tests under isolation headers, and uploads the runnable site plus WASM as `infernal-wasm32-threaded`.

For a manual run in an existing browser, open `/tests/browser-check.html` on the supplied server and click **Run browser checks**. The page reports real pool initialization, stormtrooper metadata, atlas SHA-256 determinism, heartbeat progress, busy/queued cancellation, and successful generation after restart. Pool initialization times out after 45 seconds with a diagnostic rather than leaving the UI loading indefinitely; failed initialization terminates the worker tree.

### Opt-in V2

The client defaults to V1. Select V2 per call with
`generator.generate(prompt, seed, { profile: 'v2' })`. The direct WASM export is
`generate_monster_with_profile(prompt, seed, 'v2')`. Rebuild the WASM package to
use V2; older packages keep working for V1 and reject V2 explicitly.
See [generation profiles](../../docs/generation-profiles.md).

Run the dependency-free client and worker contract checks with
`node --test web/wasm/tests/profiles.node.mjs` from the repository root.
