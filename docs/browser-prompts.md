# Random prompts in the browser

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

```js
const text = await generator.suggestPrompt('18446744073709551615', 2, {
  promptProfile: 'necromancer',
  signal: controller.signal,
});
const monster = await generator.generate(text, '18446744073709551615'); // V1
const refined = await generator.generate(text, '18446744073709551615', { profile: 'v2' });
```

`MonsterGenerator.suggestPrompt` returns a promise of a string, not a generated package. It accepts bigint, decimal-string, or safe-integer seeds; invalid inputs reject before worker startup. Suggestions and generation run in one request-correlated queue. Aborting any pending request or calling `cancel()` rejects all outstanding requests and terminates the worker; the next request starts a fresh worker. `dispose()` is permanent.

The low-level WASM export is `suggest_prompt(seed, difficulty, prompt_profile)`. The seed is a JS BigInt/u64 argument; prefer the client wrapper because it validates range before wasm-bindgen's integer conversion. Difficulty is validated before narrowing, including rejection of fractional, non-finite, and wrapped-u8 inputs such as 257. A previously built binary lacking this export yields an explicit rebuild error, with no fallback JavaScript generator.

See [WASM build and browser checks](../web/wasm/README.md).
