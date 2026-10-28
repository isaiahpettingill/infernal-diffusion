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

Sound is opt-in. The browser backend uses native Web Audio filtering, rate
transposition, envelope, compression, bounded modulated delays and short echoes.
It processes vetted local Wikimedia recordings, not remote streamed assets.
V1 occasionally chooses mismatched comic bases and fewer variants; V2 favors
matching families, more variations and subtler spatial effects. Both share the
same peak, voice, duration and repetition limits. This is sample-based browser
audio, not a native Rust/Godot synthesis backend or voice cloning.

**No attribution required** mode filters every source before selection/decode,
invalidates processed and decoded caches, and stops pending/current voices when
switched. It permits only verified CC0/worldwide PD sources. Normal mode also
permits vetted commercial-use CC BY/CC BY-SA recordings, with visible source
credits and downloadable `.audio-provenance.json` for Destructos. Each output
uses one source; no incompatible ShareAlike licenses are blended. Source notices
and modifications remain in `audio/NOTICE.md` and the manifest. No-attribution
mode does not claim all files in the full distributable are public domain.

Wilhelm and exact Star Wars name pronunciations are not bundled while source
rights/identity remain unresolved; cleared grunts/ordinary words are fallbacks.

## Verification

`node --test web/wasm/play/*.test.mjs`

Browser generation additionally requires the real generated `web/wasm/pkg/`
bundle. Missing BERT/WASM reports an error, never a fake generated monster.
The existing WASM workflow packages this route and its real-browser screenshot.
Pages deployment is manual, main-only, and defaults to publish=false. Do not
select publish=true without the final user go-ahead; no automatic deployment.

Known limitations: no native Rust/Godot audio backend; complex combat primitives
above; no subjective listening certification. CI verifies real decoding,
nonzero processed PCM, bounded peaks/voices and actual playback scheduling.
Native/WASM float identity is not promised. Generated audio is not exported as
Vorbis by this browser harness; provenance covers the played PCM outputs.
