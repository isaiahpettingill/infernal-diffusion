# Player-relative world scaling (V1 and V2)

Both profiles now publish one stable `size.world_scale` for a **96px reference
player**, centered within the requested 70–128px player-height range. Atlas tile
size is texture resolution, not monster size. V1 is still the default, and V2
still owns its refined anatomy/encounters.

## Size policy

These are gameplay-readable neutral-body heights, not literal zoological sizes:

| Creature | Default height |
| --- | ---: |
| Beetle / insect | 24px |
| Spider / frog / snail | 28px |
| Rabbit / bat / fish | 32px |
| Wolf / goblin / slime | 56px |
| Humanoid / horse | 96px |
| Orc | 112px |
| Bear / griffin | 128px |
| Elephant / dragon | 160px |
| Mammoth / tree monster | 192px |
| T. rex / kraken | 224px |
| Long-neck dinosaur | 288px |
| Any cute or chibi creature | 64px |

Other species use explicit affinity defaults or a body-plan fallback in
`src/world_scale.rs`. Animal-headed humanoids retain humanoid stature. Long,
horizontal creatures retain their authored aspect ratios; wings and long tails
can extend beyond the reference body. This deliberately does not force every
creature to player height.

Global prompt modifiers multiply that baseline **after species/recipe selection**:

- `giant`, `huge`, `colossal`, `massive`: 1.65–2.50×
- `large`, `big`: 1.15–1.50×
- `small`, `little`: 0.55–0.80×
- `mini`, `miniature`, `tiny`: 0.25–0.45×

A bounded, seed-derived factor is selected once for the package, independently
of the animation, parser, profile and geometry random stream. Same seed and
modifier reproduce the same multiplier; different seeds vary it. Thus a giant
cute elephant is 64px times the giant factor, while a mini elephant shrinks from
its ordinary 160px baseline. The last whole-creature modifier wins if several
conflict. Modifiers bound to body parts or equipment, such as `giant sword`, keep
the existing part-scale behavior and do not change whole-creature stature.

For explicit Rust/JSON specs, `world_height_px` (4–1024, finite) overrides the
species, cute/chibi baseline and all text size modifiers. The optional
`world_size_modifier` enum is `normal`, `giant`, `large`, `small` or `mini`.
Existing `size`/`morphology_scale` still describe already-baked morphology; do not
multiply them a second time. Mounted composites combine rider/mount stature and
retain clause-local modifier intent; spawned minions receive a smaller explicit
height instead of blindly inheriting a giant parent's size.

## Engine contract

New additive `SizeInfo` fields (protobuf tags 9–12, identical CBOR/JSON names):

- `world_scale`: baked sprite pixels → world pixels
- `reference_height_px`: neutral core mesh height at the atlas pixel density
- `target_height_px`: final requested neutral-body world height
- `reference_player_height_px`: 96

`reference_height_px * world_scale == target_height_px` within floating-point
precision. Measure the neutral core mesh once, not transparent padding, first
emergence frame, largest attack pose, raised weapon or wingspan. Lower-body
merfolk tails count as anatomy; ordinary accessory tails do not. Frame fitting
still covers all animated geometry without shrinking the character. No runtime
frame-dependent auto-fit or scale jitter is needed.

Apply the single scalar coherently:

- Atlas/collider/posed-attack-view pixel offsets: subtract the package anchor,
  mirror when required, then multiply by `world_scale`
- Collider-view radii and other baked pixel lengths: `world_scale`
- Generator-space lengths, root motion, attack reach/radii and speed:
  `size.pixels_per_unit * world_scale`
- Keep angles, durations, damage, counts and dimensionless factors unchanged
- Keep physics roots unit-scaled if the engine already converts dimensions
  explicitly; never scale the same distance again via a parent transform

A host with a different reference player can multiply **all** spatial conversions
by `actual_player_height / 96`, or provide an explicit absolute height. The
playground uses 96px. Destructos applies the contract once to art, physical bodies,
hurtboxes and combat anchors. Existing external consumers must adopt the scalar;
opening the atlas alone displays its baked resolution, not its world size.

All four new fields equal to zero identify a legacy package. Consumers may keep
their old display policy for those assets. Invalid/nonfinite or inconsistent new
metadata is rejected by package validation. Regenerate cached packages to obtain
the new scale policy. The legacy flat-2D renderer is separate from the default
V1 mesh3D pipeline and retains its older output contract.

## Compatibility and verification

Ordinary V1 baked PNGs and previous metadata fields are preserved; the package ID,
scale recipe marker and four additive size fields intentionally change. The
same-host compatibility checker permits only these specific metadata changes and
continues to require byte-identical PNGs and previous metadata values. Both native
BERT and vocabulary/WASM parsing retain the size intent. Godot protobuf and all
four CBOR bindings are regenerated from the schema.

Reproduce the actual renderer evidence (no image-generation model):

```sh
cargo run --release --no-default-features --example world_scale_catalog
python3 tools/world_scale_evidence.py
cargo test --release --no-default-features --test world_scaling
```

[Rendered comparisons and measurements](evidence/world-scaling/README.md)
