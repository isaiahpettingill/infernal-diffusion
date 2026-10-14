# V2 visual and contract review

These images are actual generator output, not AI image mockups.

- [V1/V2 sprite comparison](v2-comparison.png): seed 42; native pixels and 3×
  nearest-neighbor details. Friendly V2 subjects show their actual lure clip.
  Generated with `profile_catalog` and `tools/profile_evidence.py`.
- [Intermediate mermaid geometry](v2-mermaid-intermediate.png): exact triangles
  exported from the V2 lure pose and rendered with the existing Blender inspection
  tool, using direct anatomy seed 42. The final sprite renderer is CPU palette
  rasterization; Blender lighting intentionally differs. Direct intermediate seeds
  exclude package hashing, so this is not the identical seeded package specimen.
- [Water navigation](v2-water-navigation.png): actual V2 mermaid sprite frames for
  swimming, hovering, rising, diving, turning and underwater attack. Runtime world
  travel comes from advisory root deltas/controller decisions, not sprite motion
  alone. These water capabilities also exist in V1; amphibians retain land clips.

Local checks on Linux:

- Lightweight parser: 125 library tests, one CLI test and four profile integration
  tests pass, including nine terrestrial V1 golden specimens from untouched main
  `6f0c9b31eabfdc9e1b593cf1c3621f75d63dd9f3` (RGBA and protobuf byte hashes).
- Embedded BERT: 127 library tests, one CLI test and three profile integration
  tests pass.
- Strict Clippy (`--no-default-features --all-targets -- -D warnings`) passes.
- Godot extension unit tests and real headless generation/save/load smoke pass.
- Seven JS client/worker contract tests pass; optimized threaded WASM build passes.
- Real Chromium tests could not start in this workspace because IPC creation was
  sandbox-blocked. The new browser tests are included for CI; no local browser
  assertion is reported as passed.

Visual review found clearer frog, mermaid and tree silhouettes and readable lure
hearts/waving. Generic animals still use shared procedural shapes; the wolf's
small head/ear silhouette and coarse low-poly surfaces remain art-direction
limitations. This is an incremental procedural V2, not a hand-authored roster or
trained image/mesh model. The selected catalog is not exhaustive visual coverage
of every species, seed or combination.

Animated output (same real catalog, timing-aware, 2× nearest-neighbor):

- [V1/V2 water navigation](water-navigation.gif)
- [Friendly lure → reveal → attack](friendly-ambush.gif)

Reproduce with `python tools/profile_animation_evidence.py output/profile-catalog output/animations`.
The heart is baked only in LURE frames; it is absent during REVEAL and ATTACK.
GIFs show sprite motion in place, not an engine simulation of world travel or
projectile spawning. Generated event/attack data supplies those engine actions.
