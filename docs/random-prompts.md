# Seeded summoner prompt profiles

Random suggestions use the same Rust generator in the CLI, C ABI, Godot, native
playground, and browser worker. A prompt profile selects vocabulary; it does **not**
select the rendering/generation profile. Existing generation remains V1 by default,
and V2 is an independent opt-in. The resulting text is always editable.

- **Default** samples 12 equally weighted families, then a canonical recipe within
  each. It spans humanoids, predators, large animals, birds, invertebrates, ocean
  life, fantasy hybrids, folklore, undead, constructs/plants/slimes, prehistoric
  creatures and serpents/cryptids. All families remain available at every difficulty.
- **Necromancer** applies skeletal, zombie or ghost traits to the broad bestiary,
  including animals and hybrids. Any mount is also skeletal.
- **Shaman** mixes orcs, trolls and 13 large beasts. Most suggestions combine two
  distinct species using parser-supported secondary affinities; large body size is
  explicit. This is the ordinary species-blend system, not the structured V2 hybrid
  schema.
- **Lunatic** independently picks three creature names, materials, anatomy, movement,
  size and elemental attacks, then scrambles their order. Compound species and count
  phrases stay intact so both parsers can recognize them. Contradictory ingredients
  use the parsers' normal precedence rules; this profile intentionally ignores the
  difficulty's usual size/attack budget.
- **Boss** always supplies hard-tier size, armored bulk, stone/metal/chitin armor,
  an elemental ranged attack and extra anatomy. Armor and magic produce real health,
  defense and attack metadata independently of visible height. Host games still own
  balancing and behavior. Boss does not claim an unsupported boss-tier field.
- **Ocean** samples 13 aquatic recipes and preserves SWIM in both V1 and V2. It
  never adds a land mount or anchoring that would disable water movement.

The same seed, difficulty and prompt profile reproduce exactly the same suggestion
within this generator version. Different versions may intentionally improve the
vocabulary and change results; save the generated prompt for cross-version replay.
Difficulty must be 1, 2 or 3, including for Boss and Lunatic. Most profiles progress
from small/physical to ordinary/mixed to huge/armored/elemental. Shaman stays large;
Boss always uses hard modifiers and Lunatic is deliberately unconstrained.

## CLI and Rust

```sh
infernal suggest 42 2
infernal suggest 42 2 necromancer
infernal suggest 42 1 boss
infernal suggest 18446744073709551615 2 ocean
```

The optional final argument is case-insensitive. `--profile v2` is reserved for
monster generation, not suggestion. Arena rounds keep the existing 1–3 progression
and rotate through the expanded recipe catalog.

```rust
use infernal_diffusion::prompt::{random_prompt_with_profile, PromptProfile};
let text = random_prompt_with_profile(42, 2, PromptProfile::Shaman)?;
```

For native HTTP and browser-worker APIs, see [browser suggestion controls](browser-prompts.md).

## C and Godot

`infernal_random_prompt_profile(seed, difficulty, prompt_profile, &error)` is an
additive optional ABI-1 symbol. Profiles are Default=0, Necromancer=1, Shaman=2,
Lunatic=3, Boss=4 and Ocean=5. Unknown IDs and invalid difficulty return a null
pointer and optional error. Free returned strings with `infernal_free_string`.
The original `infernal_random_prompt` remains equivalent to Default.

```gdscript
var job = generator.suggest_prompt_async_profile(42, 2, generator.PROMPT_SHAMAN)
# Poll the usual result; result.prompt is editable text for generate_*_async.
# result.difficulty is the effective tier (always 3 for Boss).
```

Rebuild both the extension and native core for the new method. The extension loads
the core symbol optionally: the legacy `suggest_prompt_async` remains compatible,
while any explicit `suggest_prompt_async_profile` request (including Default)
against an older core reports a rebuild error rather than using an old catalog. Godot seeds retain its existing signed, non-negative int64
range. Rust, C and browser paths support the full unsigned 64-bit seed range.

The sample-distribution tests measure actual parsed affinities/body plans/materials,
not just distinct strings, and check profile constraints across both interpreters.
Integration tests generate real V1/V2 packages for all six profiles and verify
non-size Boss gameplay bonuses plus Ocean swimming metadata.
