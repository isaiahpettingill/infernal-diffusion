# Playable monster harness (draft)

Open `play/` beneath the existing portable WASM demo server after running the
repository's `tools/build_wasm.sh`. The generator, BERT, worker, seed handling,
and package schemas are unchanged. Nothing here calls a generation service.

Generate with a prompt, exact decimal seed and V1/V2 profile. Every generated
animation gets a button and deterministic keyboard binding. Additional shortcut
banks cover unusually large animation sets. A/D or arrows move; Space jumps.
Typing in a field never controls the monster. Death/hurt poses can be exercised
without destroying the generated package; Idle resets the pose. Reset dummy
clears health, damage counters and projectiles.

The dummy receives damage from timed generated attack steps and spatial contact,
not from pressing an animation button. Projectile simulation is a bounded test
adapter. Complex field/summon primitives currently announce **animation only**;
they are not implemented combat effects. This is not Destructos' game controller.

Audio is a versioned recipe declaration only, not a finished voice synthesizer.
The user-supplied Wilhelm death fallback is planned, with V1 commonly using it
and V2 usually transforming it; no asset is bundled yet. Curated bases, licensing
provenance and audition approval remain pending. Do not publish this draft until
the user gives the final go-ahead.

## Verification

`node --test web/wasm/play/engine.test.mjs`

Browser generation additionally requires the real generated `web/wasm/pkg/`
bundle. Missing BERT/WASM reports an error, never a fake generated monster.
CI builds a portable artifact but does not deploy automatically. A separate
manual Pages deployment may be added after review; preserve existing site routes.

Known limitations: synthesized audio/Rust audio core not yet implemented; no
local real-WASM/browser run yet; special combat primitives above; no final
artistic/audio tuning. Native/WASM float identity is not promised.
