# Structured V2 hybrid recipes

Hybrid generation is an explicit V2 operation. It assembles anatomical recipes;
it does not ask a language model to infer omitted parts. Prompt aliases remain
available on existing V2 generation APIs, while structured callers use the same
strict JSON schema across Rust, C, WASM and Godot.

## Schema version 1

A machine-readable [JSON Schema](../schemas/hybrid-recipe.schema.json) is supplied
for editors and integrations. Rust validation remains authoritative at generation.

Every field is required. Unknown fields, unknown enum values, unsupported schema
versions, invalid limb counts and incompatible part combinations return errors.

```json
{
  "schema_version": 1,
  "upper_body": "HUMANOID",
  "lower_body": "HORSE",
  "head": "HUMANOID",
  "lower_limb_count": 6,
  "upper_arm_count": 2
}
```

Part names are case-sensitive:

- Upper body: `HUMANOID`, `STORMTROOPER`, `HARPY`, `TREE`
- Lower body: `HUMANOID`, `HORSE`, `SPIDER`, `MERMAID`, `TREE`, `SERPENT`, `WOLF`
- Head: `HUMANOID`, `FISH`, `WOLF`, `BULL`, `BIRD`, `SPIDER`, `STORMTROOPER`, `TREE`, `CAT`

Upper arms are bounded at 8 and lower limbs at 12. The recipe validator also
checks compatibility; these maximums do not mean every count fits every part.
Use zero lower limbs for a mermaid or serpent lower body. Explicit counts remain
authoritative rather than being replaced by seeded random topology mutations.

## CLI

Save the example as `centaur.json`, then run:

```sh
infernal --hybrid-json centaur.json 42 out/centaur
```

This route always uses V2 and the deterministic structured parser. An explicit
`--profile v1` is rejected. It writes the standard protobuf and atlas package.

## Rust and C

Deserialize `v2::hybrid::HybridRecipe` with serde, then call
`generate_hybrid(&recipe, seed, output_dir, format)` or
`generate_hybrid_in_memory(&recipe, seed)`. Both validate the recipe.

C callers pass the same JSON to
`infernal_generate_hybrid_json(recipe_json, seed, &error)`. A null result means
failure. Successful handles use the existing object access, save and free APIs;
allocated errors use `infernal_free_string`. The new symbol is optional for
consumers dynamically loading older ABI-1 libraries.

## Browser / WASM

```js
const monster = await generator.generateHybrid(recipeObject, 42n);
```

The client accepts a JSON object or JSON text. The worker calls the direct WASM
export `generate_hybrid_json(recipeJson, seedBigInt)`. Validation is performed by
the same Rust schema; cancellation and result ownership are unchanged. An older
WASM binary reports that structured hybrid support requires rebuilding.

## Godot

```gdscript
var recipe := {
    "schema_version": 1,
    "upper_body": "STORMTROOPER",
    "lower_body": "MERMAID",
    "head": "STORMTROOPER",
    "lower_limb_count": 0,
    "upper_arm_count": 2,
}
var job := generator.generate_hybrid_async(recipe, 42)
```

Poll using `poll_result()`, then use the existing in-memory save/release methods.
The native extension exposes `generate_hybrid_json_async(recipe_json, seed,
absolute_library_dir)`; the addon converts the dictionary and library path.
Schema errors arrive as failed jobs. Invalid seeds or queue backpressure return
`-1`. An older core produces a clear unsupported-hybrid job error, never a silent
fallback to ordinary prompt generation.

## Mounted and deliberately unusual prompts

V2 also normalizes mount-first wording through the public prompt route:
`Ghost lion with orc rider with huge axe` becomes an orc rider carrying a 1.8× axe
on a ghost lion. The ghost modifier belongs to the lion, not automatically to the
rider. This uses the existing mounted-body and survivor-package contract rather
than treating the rider as a second animal head. Spectral appearance follows
mounted node ownership; consumers must apply the appropriate part/companion
interaction data rather than make every mounted part share the rider's rules.

`Skeleton mermaid with eight arms` retains eight explicit arms, a fish-tail lower
body, and skeletal skull/rib/limb materials. Explicit counts win over occasional
seeded topology variation. Tests exercise both exact strings through embedded
BERT and vocabulary-only builds, plus rejected contradictory/invalid requests.
