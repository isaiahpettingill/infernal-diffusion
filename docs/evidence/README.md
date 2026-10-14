# V2 visual and contract review

These are actual generator pixels and exact baker triangles, not image-generation
mockups. Native views and nearest-neighbor enlargements are labeled; full-frame
128px previews fit the complete returned frame rather than cropping out limbs.

## Current review

- [V1/V2 recipe comparison](v2-comparison.png)
- [V1/V2 humanoid comparison](v2-humanoid-comparison.png): orc, trooper, humanoid,
  harpy and mermaid; V1 keeps its original heads
- [Explicit hybrid assemblies](v2-hybrid-compositions.png): six-legged centaur,
  spider-centaur, fish-headed humanoid, trooper/mermaid, walking tree
- [Mermaid seeded variants](mermaid-seeded-variants.png): ordinary, extra tail,
  four arms, both; extra appendages remain connected and animate
- [Skeleton variants](v2-skeleton-variants.png): recognizable skull cavities,
  ribs, pelvis and long bones with bounded seeded irregularities
- [Exact intermediate head geometry](v2-head-geometry.png)
- [Intermediate mermaid head](v2-mermaid-intermediate.png): actual 3D baker
  triangles with fixed Blender inspection lighting, not the sprite shader

## Animated samples

- [V1/V2 water navigation](water-navigation.gif)
- [Friendly lure → reveal → declared ambush attack](friendly-ambush.gif)
- [Full-body mermaid / bilateral growing-tree attacks](v2-specialty-attacks.gif)
- [Whole-body shade unwind and exact reverse (humanoid)](v2-shade-tendrils.gif)
- [Ghost and ghoul phase/reappear attacks](v2-spectral-phase.gif)

The shade contact sheet covers humanoid, wolf, tree and mermaid whole-body
unwind/reversal. Spectral samples use a checker background to reveal actual alpha.
Shade atlases have occupied pixels exactly `(0,0,0,255)`; ghost base/phase alpha
is 97/64; ordinary ghoul base/phase alpha is 255/64. Damage and solid collision
rules are separate exported data, not inferred from these pixels. Hearts occur
only in LURE frames and are absent during REVEAL/ATTACK.

GIFs show baked sprite motion in place, not a game-engine simulation of world
travel, projectiles or collision. Returned attack events, damage rules and posed
contact tracks supply those engine actions. Consumers retain targeting,
pathfinding, safe reappearance placement and world-unit scaling.

## Verification and reproduction

Release tests cover both the vocabulary-only and embedded-BERT configurations.
Strict no-default-feature Clippy, JS client/worker contracts, native Godot
integration smoke and the optimized threaded WASM build have been run. Actual
browser generation is exercised by the PR's WASM CI workflow; local browser IPC
was sandbox-blocked and is not reported as a local success.

Terrestrial V1 is compared byte-for-byte against untouched commit
`6f0c9b31eabfdc9e1b593cf1c3621f75d63dd9f3` on the same host/compiler for eleven
specimens, including every PNG and serialized protobuf field. Original frozen
hashes are retained as cross-host diagnostics, not regenerated to hide changes.
See [the portable regression explanation](../V2.md#portable-v1-regression-check).

```sh
cargo run --release --no-default-features --example profile_catalog -- output/profile-catalog
cargo run --release --no-default-features --example v2_variants -- output/v2-variants
cargo run --release --no-default-features --example v2_composites -- output/v2-composites
python tools/profile_evidence.py output/profile-catalog output/v2-comparison.png
python tools/profile_evidence.py output/profile-catalog output/v2-humanoid-comparison.png orc stormtrooper humanoid harpy mermaid
python tools/profile_animation_evidence.py output/profile-catalog output/animations
python tools/v2_variant_evidence.py output/v2-variants output/variants
python tools/v2_composite_evidence.py output/v2-composites output/composites
```

The profile catalog uses seed 42 except its ordinary mermaid baseline uses seed0.
The variant/composite examples record their explicit seeds. The
[66-affinity coverage manifest](../../assets/v2/humanoid_face_coverage.json)
enumerates refined humanoid faces and deliberate animal-headed exceptions; the
rendered catalog samples representative families rather than every combination.

Remaining art-direction limits: low-poly hair is formed from curved clumps rather
than fine strands; facial detail is tiny at native resolution; large tree canopies
can make tendril motion less prominent; and generic animals still share body-family
shapes. Additional clips can enlarge atlases, so use the returned dimensions and
consider offline baking/caching for large rosters. This is a procedural V2, not a
hand-authored roster or a trained image/mesh model.
