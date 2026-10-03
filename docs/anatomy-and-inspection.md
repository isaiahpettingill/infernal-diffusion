# Anatomical generation and visual inspection

The generator is deterministic procedural geometry, with an optional embedded semantic parser. These improvements do not train or replace the semantic model.

## Structure before ornament

- Insects have a head, thorax and abdomen; six walking legs attach to the thorax. Compound eyes and paired antennae are separate from spider eyes/chelicerae. Beetles have paired wing covers; hornets have a narrow petiole, four membranous wings and abdominal sting.
- Spiders have a fused anterior body and a distinct abdomen, with eight laterally splayed legs. The small head node is a face/chelicera attachment within the anterior body, not a third free body region. There are no antennae unless explicitly requested.
- Scorpions retain eight walking legs, two pedipalps and a segmented raised tail. Centipedes attach leg pairs to successive trunk segments. Centipede leg count remains deliberately bounded by the generator's 12-limb budget; this is a stylized monster, not a taxonomic specimen.
- Opposite limbs use adjacent even/odd IDs. Explicit lateral position and radius replace the previous inconsistent ID-derived depth for these anatomies.
- Requested nonstandard leg counts and extra features survive specialization. Species identity supplies defaults, rather than overriding an explicit monster design.
- A handful of seeded shape parameters vary length, width, abdomen proportion and leg spread coherently, preserving symmetry and connectivity. Other families receive species-conditioned length/girth/height variation. This avoids independently jittered joints and disconnected appendages.

References used for the anatomy constraints: [Smithsonian insect anatomy](https://naturalhistory.si.edu/education/teaching-resources/life-science/what-insect), [Natural History Museum spider anatomy](https://nhm.org/community-science-nhm/spider-survey).

Custom Rust anatomy stages constructing `Node` literals must now supply `z: None, rz: None` for the legacy depth heuristic, or `Some(...)` values for explicit 3D lateral geometry. These are intermediate fields, not a change to the package skeleton wire format.

## Inspect the exact intermediate scene

```
cargo run --release --no-default-features --example export_intermediate -- "beetle" 42 /tmp/beetle
blender -b --python tools/inspect_intermediate_blender.py -- /tmp/beetle/idle_00.json /tmp/beetle.png /tmp/beetle.blend
```

The JSON contains actual triangles passed to the sprite rasterizer, per-face colors, node parents and posed positions. Blender imports those triangles; it does not substitute a prettier model. Coordinates are converted from X-forward/Y-up/Z-lateral into Blender's Z-up frame. Render lighting is neutral inspection lighting; final game sprites still use the existing CPU palette rasterizer.

Frames expand to the complete animated triangle envelope across all four views, with a three-pixel border. This changes atlas framing, never the shared pixels-per-unit scale. Extreme requests needing more than 512-pixel frames fail with an explicit validation error rather than silently clipping. Atlases are packed near-square to avoid tall textures exceeding common hardware limits. Consumers must use the returned atlas dimensions, column count and anchor.

The inspection example uses the same anatomy/animation/scene functions with a direct `ChaCha8Rng` anatomy seed. This intentionally excludes package recipe-hash seeding, so before/after geometry comparisons can use identical input anatomy seeds even when a recipe version changes. Game generation continues to hash the prompt, recipe version and seed. Compare production packages separately with the same prompt and public generation seed.

An optional fourth argument selects an animation clip (for example `death` or `attack_primary`) and exports every pose of it. Files are named `<clip>_<index>.json`. The `spec.json` sidecar records parser output.

## Regression checks

Run `cargo test --release` for the complete default-feature suite and `cargo test --release --no-default-features` for the vocabulary-only build. Anatomy tests check multiple families across 16 seeds, repeatability, body/appendage counts, foot-side symmetry, valid parent ordering, explicit feature preservation, and varied body proportions.

The authored source for the three new meshes is `tools/author_meshes.py`; regenerate checked-in mesh JSON with `uv run python tools/author_meshes.py`. Intermediate exports and Blender renders are developer inspection artifacts, not assets required by the runtime.
