# V2 motion, equipment and face refinement evidence

Captured October 15, 2026 from the native, no-default-features V2 generator. All eleven packages passed generation/package validation. Native frames are **96 × 96 pixels**; GIFs enlarge those baked pixels with nearest-neighbor sampling. The four GIF panels are the renderer's actual 0°, 90°, 180° and 270° views, with original clip timings. These are procedural meshes and poses, not image-generation mockups.

## Review files

- [All motion keyframes](motion-keyframes.png)
- [Orc rapper, shotgun prop, seed 42](dancer_orc.gif): alternating arm raises/chest accents, head nods and stepping weight shifts
- [Wolf dancer, seed 7](dancer_wolf.gif): foreleg lifts, head bob, alternating leg phases and tail motion
- [Floating jellyfish dancer, seed 0](dancer_jelly.gif): body roll with linked appendage movement
- [Shade dancer, seed 42](dancer_shade.gif): retains the spectral dark silhouette; passive contact only
- [Spear throw, seed 43](spear_throw.gif) and [thrust, seed 42](spear_thrust.gif)
- [Spear projectile playback](spear-projectile-playback.gif): exported actor and projectile art composed using the exported release time, tracked origin and speed; this is an isolated playback harness, not Destructos runtime footage
- [Laser cannon, seed 42](laser.gif), [sword](sword.gif), [troll axe](axe.gif), [robot chainsaw](chainsaw.gif), [reaper scythe](scythe.gif)
- [Machine-readable exported metadata and movement audit](motion-metadata.json)

## Implemented contracts

### Dancer

Explicit dancers and the seeded rare modifier share one behavior. Active attack clips/events/projectile exports are removed. Exactly one `v2_passive_contact` descriptor remains: `CONTACT`, `OVERLAPPING_ACTOR`, damage 6, cooldown 500 ms, no spawned count or projectile speed. Its visual animation is the normal dance idle. Runtime consumers must implement overlapping-body damage from this descriptor; export validation alone does not prove external engine support.

Loops use sixteen 80 ms frames (1.28 seconds). Existing root travel per cycle is retained and each movement speed is recalculated from that distance and the new duration. A Ninja compound shortens the movement frames and rescales speed by the exact duration ratio. Every looped dancer movement frame in this capture agrees with its mode's advertised speed to less than 0.0001 world units.

The joint choreography preserves all anatomical edge lengths and keeps children attached. Grounded dances intentionally step and bounce; there is no world-space planted-foot IK lock. Floating/limbless recipes use whole-body roll and linked appendage phases. The existing generic jellyfish body is retained; this evidence verifies its motion rather than asserting a newly realistic jellyfish model.

A dancer's gun is an inactive prop; the combat two-hand grip solver is bypassed so it does not suppress the arm choreography.

### Weapons and grip

V2-authored meshes cover laser, railgun, shotgun, cannon, rocket/missile, sword, axe, spear, chainsaw and scythe. Guns retain local muzzle X=1.3 and the established primary/secondary grip anchors. Grip solving resolves actual hand/arm/shoulder chains belonging to the local weapon owner, including supported hybrids. It never invents hands for turrets, mounted weapons or one-armed creatures. Both grips contact their rigid weapon through the tested idle/attack/recoil poses without stretching either arm.

The scythe has a long exposed shaft, wrapped grip, collar and hooked upper blade. It is attached at the hand's actual handle position. Laser details include the receiver, stock, cooling collars, capacitor strip and emitter; the front view naturally foreshortens the barrel. Chainsaw teeth and small gun details remain only a few pixels at the game resolution.

### Spear variant

Eligible hand-held spears select a throw when `seed % 3 == 1`; other seeds retain thrust. Dancers and shades are excluded. The throw contains twenty-five 50 ms frames, a 450 ms release, one velocity-aligned SPEAR projectile at speed 200, 1200 ms projectile lifetime and a 1500 ms attack cooldown. The carried mesh is hidden from release through recovery, then returns at 1100 ms. The final pose returns to the original held placement. Rig modifiers run after spear generation so their event and attachment contracts remain intact. Awkward-mutant variants delay release/recovery together; for example seed 7 releases at 570 ms while seed 43 uses the base 450 ms.

### Faces

Orc and troll have separate heavy-skull meshes with broad jaws, recessed sockets, brow planes, rooted tusks and folded ears. Troll adds the longer hooked nose and narrower facial proportions. Upper crown rings were broadened to remove pointed cranial silhouettes. Cyclops has one integrated central orbit; adult manga and historical recipes have separate face/hair silhouettes. The adult manga face uses mature midface/chin proportions and no school-age styling. Existing ape and V1 face paths are retained.

## Automated checks

- Full native no-default library suite passed at the integration checkpoint: **190 passed, 0 failed, 1 ignored**. Subsequent edits were stronger dance choreography, scythe readability, rounded skull crowns and the dancer speed fix; focused checks below were rerun after these edits.
- Latest focused dancer checks: 3 passed, including active-attack suppression, rigid anatomical edges and per-frame speed parity for orc/wolf/cobra/jellyfish with and without Ninja
- Latest grip checks: 3 passed, 1 intentionally ignored visual-package helper
- Latest V2 weapon geometry/stable-muzzle check: 1 passed
- Latest V2 face geometry/identity checks: 3 passed
- Spear seeded release/recovery/thrust/rig checks passed for ordinary and mutant seeds
- All eleven final native render packages validate, including shade dancer and the actual spear projectile export

The parent integration report owns the final exact-head full suite, BERT/WASM/browser checks and V1 compatibility report.

## Reproduce

The case prompts/seeds and exported metadata are recorded in `motion-metadata.json`. `capture_motion.rs` is the small native generator used for each case and can be compiled against the crate's release library. It takes `OUTPUT_DIR PROMPT SEED` and writes the real package plus `metadata.json`. `make_motion_previews.py PACKAGE_ROOT` rebuilds the four-view GIFs/keyframe sheet from those packages. `make_spear_playback.py SPEAR_PACKAGE_DIR` rebuilds the isolated exported-projectile playback. The preview scripts require Pillow; they crop existing rendered assets and do not generate replacement art.
