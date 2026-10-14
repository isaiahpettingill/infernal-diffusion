# Opt-in V2 generation

All existing entry points and invocations still generate V1. V2 is an explicit
choice per generation request; it is not a process-global setting. V3 is not
implemented. Profile strings are exactly `v1` and `v2`; numeric C/Godot profiles
are `1` and `2`. Other values fail rather than falling back.

## CLI

```sh
infernal "armored ogre" 42 out/v1
infernal "armored ogre" 42 out/v2 --profile v2
infernal bake3d-cbor "armored ogre" 42 out/v2-cbor --profile v2
```

The optional model directory retains its original position. `--profile` applies
to the default monster-generation command, `bake3d`, and `bake3d-cbor`. V2 is
rejected for `bake2d` and `bake2d-cbor`; props and other non-generation commands
reject the profile option. A simple-parser build works without neural dependencies:

```sh
cargo run --no-default-features -- "armored ogre" 42 out/v2 --profile v2
```

## Rust

Use `generate_in_memory_with_profile(prompt, seed, GenerationProfile::V2)` or
`generate_with_profile(prompt, seed, output_dir, model_dir, format,
GenerationProfile::V2)`. The existing APIs retain V1 defaults.

## C ABI

`infernal_generate_in_memory_profile(prompt, seed, INFERNAL_PROFILE_V2, &error)`
returns the same opaque handle as `infernal_generate_object`. Read it using the
existing object accessors, optionally save with `infernal_object_save_with_format`,
and free it exactly once with `infernal_object_free`. A null handle indicates an
error; free the error string using `infernal_free_string`.

This is an additive ABI-1 symbol. Dynamic consumers must check its presence when
loading older cores. Existing V1 calls do not require it.

## Godot

```gdscript
var generator := InfernalDiffusion.new()
var job := generator.generate_in_memory_async_profile("armored ogre", 42, InfernalDiffusion.PROFILE_V2)
# Or generate directly to a package directory (optional fifth argument: format).
var disk_job := generator.generate_async_profile("armored ogre", 42, "user://ogre", InfernalDiffusion.PROFILE_V2)
```

Poll and release in-memory results exactly as before. Invalid profile values
return `-1` before queuing. V2 against an old native core returns a failed job
explaining that the library must be rebuilt; it never silently produces V1.
The native `InfernalGenerator.generate_async_profile` method takes
`(prompt, seed, absolute_output_dir, format, profile, absolute_library_dir)`;
the addon wrapper handles paths and defaults to protobuf.

## Browser / WASM

```js
const result = await generator.generate('armored ogre', 42n, { profile: 'v2' });
```

Omitting the option uses V1. The direct WASM export is
`generate_monster_with_profile(prompt, seedBigInt, 'v2')`; the old
`generate_monster(prompt, seedBigInt)` export remains V1. Rebuild the WASM package
to use V2. An older binary remains usable for V1 and rejects V2 with an explicit
rebuild message. Cancellation and transfer ownership are unchanged.

Check the JS client contract without a browser or WASM build:

```sh
node --test web/wasm/tests/profiles.node.mjs
```

## Friendly-lure lifetime

The V2 lure descriptor uses `reset_policy: "RETURN_TO_LURE"`: the provided
visuals support a rearming ambush. The engine chooses when to play the harmless
`LURE` loop, the complete `REVEAL` telegraph, and the referenced attack. After
the attack, returning to idle or movement displays the friendly appearance again.
This is not a persistent mutation flag or a built-in AI state machine. An engine
may keep combat active and choose its own clips; persistent hostile idle/movement
would require additional assets and engine-maintained state.

For explicit cross-species anatomy assembly, see [structured hybrid recipes](hybrid-recipes.md).
Spectral interactions are specified separately in the [spectral damage and phasing contract](spectral-contract.md).
Animated hit and emission origins use [posed attack-origin tracks](attack-origin-tracks.md).
