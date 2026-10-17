# Lossless runtime atlas storage

Compact atlases are an **opt-in storage representation**, independent of V1/V2
monster generation. Legacy disk/in-memory/C/WASM APIs continue returning the
uniform-grid format. No resolution, directions, animation frames, durations,
attack events, special attacks, or physical coordinates are removed or changed.

Use `generate_in_memory_compact_with_profile` / `generate_hybrid_in_memory_compact`
in Rust, the matching `_compact` C generation exports, or
`client.generate(prompt, seed, {profile: 'v2', compact: true})` and
`client.generateHybrid(recipe, seed, {compact: true})` in JavaScript. These pack
each companion as it is completed, avoiding accumulated dense companion copies.
`atlas::compact` / `infernal_object_compact` can also compact an existing object;
the C operation invalidates borrowed image pointers and must run before reading
image data. Compaction is idempotent.

## Consumer contract

Format 8 adds `sprites.frame_regions`, one rectangle per **logical frame ID**:
`x, y, width, height` locate the packed pixels, and `offset_x, offset_y` place them
inside the original virtual frame. `frame_width` and `frame_height` stay unchanged.
`columns` is retained for backwards schema compatibility but must not be used to
slice a compact texture. Repeated frames may reference the same packed rectangle.
Rectangles include no rotation. One transparent pixel surrounds each allocation.

Render the region at its trim offset inside the original full frame, then apply
the same anchor and world scale as before. Godot `AtlasTexture.margin` provides
this virtual-size behavior without rebuilding a dense texture. All collider,
attack-origin and motion-track coordinates remain in the original frame system.
Reject malformed/overflowing or missing rectangles; never guess grid positions
when compact metadata is present. Older packages without regions use the grid.

Emission shares the color atlas layout. When it is entirely empty,
`emission_file` is empty and the raw image API returns a 1×1 transparent sentinel
for ABI compatibility; consumers should skip allocating an emission texture.
Nonempty emission and projectile pixels are preserved. Persisted compact packages
round-trip through protobuf/CBOR validation and the updated Godot readers.

## What this saves

Transparent frame margins are cropped, identical cropped pixel blocks are shared,
and unused emission textures are eliminated. The raw RGBA payload and GPU texture
allocation both shrink. This is different from PNG compression, which only reduces
encoded disk/network bytes. The existing bundled BERT/WASM download is a separate
cost and is not reduced by atlas packing.

The CPU rasterizer still produces complete virtual frames at the original
resolution. A dense sheet exists while each package is generated; compact storage
does not claim to eliminate that temporary allocation. Packing at the sink avoids
the old extra color-sheet clone and avoids creating a full-size emission image
before compaction. Peak process RSS also includes model/WASM memory and allocator
high-water marks, so it is measured separately from retained atlas bytes.

NES-style component sprites could avoid some repeated artwork, but these monsters
are articulated 3D meshes with depth-dependent occlusion, deformation, shading,
translucency and effects. Assembling separately rendered parts would change that
visual contract or require a more complicated renderer. Complete-frame trimming
and sharing provide substantial savings while preserving every output pixel.

## Reproducible checks

- `cargo test --release --test compact_atlas`: exact reconstructed color/emission,
  all gameplay metadata and specials, hybrid parity, save/load, bad rectangles
- `node --expose-gc web/wasm/tests/atlas-cost.node.mjs PATH_TO_BASELINE_PKG`
- `node --expose-gc web/wasm/tests/atlas-cost.node.mjs web/wasm/pkg compact`

Compare each sample's `logicalPixelsSha256` and `gameplaySha256`; all must match.
The benchmark uses the actual bundled BERT/WASM and separately reports generation
wall time, transferred decoded atlas bytes, unique frames and process peak RSS.
Node results are not a claim of browser/GPU timing or deployment verification.

## Measured browser-runtime results (2026-10-16)

Real bundled BERT/WASM executed in Node on the same host, seed 42, identical
ordered fixture workloads, explicit GC between samples. One timed run per case;
these timings are illustrative, not a latency distribution. First case includes
first-generation warm-up. RSS is the complete process high-water mark.

| Fixture | Profile | Raw atlases before → after (MiB) | Reduction | Generation before → after |
|---|---|---:|---:|---:|
| slime | v1 | 19.14 → 0.92 | 95.2% | 1.340 → 1.288 s |
| slime | v2 | 19.14 → 0.84 | 95.6% | 0.395 → 0.319 s |
| orc | v1 | 81.28 → 5.42 | 93.3% | 1.075 → 1.030 s |
| orc | v2 | 47.53 → 4.08 | 91.4% | 1.424 → 1.360 s |
| complex | v1 | 243.37 → 57.25 | 76.5% | 5.597 → 5.125 s |
| complex | v2 | 280.34 → 27.99 | 90.0% | 7.808 → 7.037 s |
| shade | v1 | 40.50 → 4.36 | 89.2% | 0.857 → 0.804 s |
| shade | v2 | 43.65 → 4.43 | 89.9% | 1.711 → 1.717 s |
| hybrid | v2 | 80.10 → 9.03 | 88.7% | 2.503 → 2.200 s |

The complex fixture is “giant orc riding a giant spider that shoots fire”; values
include the root and both survivor companion packages. Shade is “shade humanoid”.
The structured hybrid is a stormtrooper upper body, spider lower body, fish head,
eight lower limbs and two upper arms. Every logical pixel and all non-storage
gameplay metadata hashes matched in all nine cases.

Peak RSS: **1044.1 → 531.6 MiB** (49.1% lower).
This is not a guarantee for every monster or browser. Atlas bytes are also the
uncompressed texture payload, excluding GPU/driver overhead and any mipmaps;
no GPU allocation timing is claimed. Transfer is the worker’s decoded pixel
buffers; the bundled model/runtime download remains approximately 23.4 MiB.

Full machine-readable measurements: [before](benchmarks/atlas-wasm-before.json)
and [after](benchmarks/atlas-wasm-after.json). The baseline source is
`26d129bc4004438adbaf06ecffe29d1a692cd6dc` (merged unchanged in PR #9).

Baseline WASM SHA-256: `0c003a0a8f452823b31d38dac6effeed8816f9f9b66ceaac725f4435d704e856`.
Measured candidate WASM SHA-256: `841f7fcd5a8636894d2c041e083890e8ab3d560e1ea710ab4e9b8fb05d974409`.

Local verification: 294 Rust tests passed, one pre-existing test ignored;
42 JavaScript contract tests passed (including compact routing). The native
Godot shim smoke and packed/grid gameplay regressions also passed. Chromium
could not launch in this executor because Unix sockets are restricted and the
elevated executor failed during setup. The real-browser regression is included
in the existing WASM CI suite; local Node results are not presented as browser QA.

## Native compute separately from packing

A quiet, interleaved native comparison of 13 fixture/profile cases (17 root and
companion package pairs) used three runs per case, discarded warm-up, and averaged
the remaining two. The APIs deliberately returned **dense** atlases in both runs
to isolate renderer/ownership improvements from compact-storage effects.
All raw sprite, emission, projectile and protobuf bytes matched exactly. JSON
metadata differed only by the additive empty `frame_regions` field.

Ordinary cases were 8–26% faster: wolf V2 0.938→0.835 s, slime V2
0.251→0.187 s, sword orc V2 0.817→0.722 s, mounted V2 4.328→3.872 s,
and shade V2 1.247→1.148 s. The four-armed structured hybrid was effectively
unchanged at 1.582→1.577 s. See [all results](benchmarks/native-render-comparison.json).
This is host-specific; it is not a lower-end-device latency guarantee.

The renderer reuses only bit-identical scenes, verified after hashing all raster
inputs and pose-dependent alpha state. Every logical frame still exists. This
avoids 13–32% of directional rasterizations in the measured fixtures, and reuses
per-direction scratch buffers and palette lookups. Downsampling keeps the same
coverage, averaging and alpha rules. No approximate pose merging is used.

Reproduce with `cargo run --release --example bench_generation -- OUTPUT 3`,
optionally `INFERNAL_PROFILE=1` for phase timings and reused-frame counts. Run both
revisions without concurrent compilers or other CPU-heavy tests.
