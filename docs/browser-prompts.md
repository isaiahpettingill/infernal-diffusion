# Browser prompts and local BERT generation

Both the native Arena (`infernal serve`) and standalone WASM demo offer:

- An editable monster prompt, including arbitrary manually written descriptions
- A **Summoner profile** selector: Default, Necromancer, Shaman, Lunatic, Boss, Ocean
- Difficulty **1 / Easy**, **2 / Medium**, or **3 / Hard**
- A **Random prompt** button that fills text without rendering a monster
- An independent **Generation profile** selector, defaulting to V1, with V2 opt-in

**Boss always uses hard modifiers; Lunatic mixes all tiers.** Their difficulty input still participates in the deterministic seed, but does not constrain those profile policies. The controls stay enabled so the same input combination can be replayed.

Suggestions come from Rust's `random_prompt_with_profile`, using exactly the same vocabulary and policies as the native API. JavaScript contains no alternative prompt vocabulary or composition tables. Summoner profiles influence suggested text; V1/V2 determines how the chosen text is generated. Changing either selector does not rewrite a hand-written prompt.

The first Random prompt click uses the seed shown in the form. Another click without changing the seed, difficulty, or summoner profile advances the seed by one and shows the new value, wrapping at `18446744073709551615`. Explicitly editing any of those options resets reroll history, so the next click uses exactly the displayed seed. Re-enter the same seed to replay its suggestion. This preserves reproducibility without repeating the same suggestion on every click. Decimal text/BigInt preserves all 64 bits. Editing any suggestion input, choosing an example, or starting generation discards stale pending suggestions. Errors preserve existing text and leave retry available.

## Native HTTP

`POST /api/suggest`, with `Content-Type: application/json`:

```json
{"seed":"18446744073709551615","difficulty":2,"prompt_profile":"ocean"}
```

A successful response has `prompt` (editable text), `seed` (decimal string), and `difficulty` (integer). The `prompt_profile` field defaults to `default` and is case-insensitive. Difficulty must be an integer from 1 to 3. Invalid JSON, seeds, difficulty, or profiles return HTTP 400 with an `error` string. Suggestions do not write packages or generate atlases.

Both `/api/suggest` and `/api/generate` accept decimal-string u64 seeds. Numeric integer JSON seeds remain supported for existing native callers; browser clients should use decimal strings to avoid JavaScript Number rounding. Generation's `profile` field stays independent (`v1` by default, `v2` explicitly). The `prompt_profile` field applies only to suggestions.

## WASM client

The default browser distribution runs the actual Candle BERT encoder and the full Rust generator in one Web Worker. The bundled tiny model, tokenizer and configuration are embedded in WASM; no generation endpoint or third-party model request is needed. It runs on ordinary static hosts without SharedArrayBuffer or cross-origin-isolation headers. Once the local bundle is loaded, generation can continue offline. Optional threaded builds retain Rayon and require COOP/COEP.

```js
const generator = new MonsterGenerator();
const runtime = await generator.ready({
  onProgress: ({ phase }) => console.log(phase),
});
// runtime.model === 'google/bert_uncased_L-2_H-128_A-2'
// runtime.parser === 'candle-bert-prototypes-v2'
// runtime.bundledModel === true; portable runtime.threadCount === 1
```

Readiness includes a real encoder forward pass. The model is initialized once per worker and reused for later calls. Prompt-generated metadata records `generation.parser: 'candle-bert-prototypes-v2'`; model loading/inference errors are reported instead of silently substituting the vocabulary-only parser. Prompt suggestions remain the shared Rust random-prompt function, independent of BERT inference and V1/V2 selection.

```js
const text = await generator.suggestPrompt('18446744073709551615', 2, {
  promptProfile: 'necromancer',
  signal: controller.signal,
});
const monster = await generator.generate(text, '18446744073709551615'); // V1
const refined = await generator.generate(text, '18446744073709551615', {
  profile: 'v2', signal: controller.signal,
  onProgress: ({ phase, completed, total }) => console.log(phase, completed, total),
});
```

`MonsterGenerator.suggestPrompt` returns a promise of a string, not a generated package. It accepts bigint, decimal-string, or safe-integer seeds; invalid inputs reject before worker startup. Suggestions and generation run in one request-correlated queue. Aborting any pending request or calling `cancel()` rejects all outstanding requests and terminates the worker; the next request starts a fresh worker. `dispose()` is permanent.

`generate` and `ready` accept optional progress callbacks with `{ phase, completed?, total? }`. Phases are `loading-wasm`, `initializing-model`, `parsing`, `building`, `rendering`, `packaging`, and `complete`; initialization only occurs on a fresh worker, and stage counts are not a global percentage. The worker emits progress messages only for requests with `progress: true`. Full-resolution RGBA atlases, world-size metadata, companion packages and exact u64 seeds are retained. Prompts are limited to 4096 UTF-8 bytes. `generateHybrid(recipe, seed, options)` still accepts explicit structured V2 recipes through a separate export, without interpreting them as prose.

The low-level WASM export is `suggest_prompt(seed, difficulty, prompt_profile)`. The seed is a JS BigInt/u64 argument; prefer the client wrapper because it validates range before wasm-bindgen's integer conversion. Difficulty is validated before narrowing, including rejection of fractional, non-finite, and wrapped-u8 inputs such as 257. A BERT-enabled binary lacking this suggestion export yields an explicit rebuild error, with no fallback JavaScript generator. Older vocabulary-only WASM distributions must be rebuilt for the current BERT worker.

See [WASM build and browser checks](../web/wasm/README.md).
