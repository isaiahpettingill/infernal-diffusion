# Anatomy-aware animation baking

The generator bakes articulated poses once, then shares them between the 2D and
3D renderers. The public clip names, semantic states and Pose layout remain stable.

- Anatomical parent transforms carry facial details, claws and held weapons.
- Walking feet have a linear support phase and eased, lifted swing. A fixed-length
  FABRIK solve keeps hips, knees, shins and feet connected. Gait phasing distinguishes
  bipeds, quadrupeds, alternating arthropod groups, centipedes, hops and limps.
  Ground speed is bounded by each limb's actual stance workspace; root deltas use
  the same speed so a constrained foot cannot silently slide against declared travel.
- Idle breath and secondary motion are small, while grounded feet retain support.
  Jump/takeoff anticipates with compression; landing absorbs weight before recovery.
- Attacks follow their actual origin chain. Weapon attacks raise the arm, strike,
  then recover. Bite, horn and tail attacks articulate their relevant anatomy.
  The strike key is selected from actual AttackStep timing rather than a generic
  sine wave. Long movement attacks include their full travel and recovery.
- Front/back impacts use opposite recoil, then damp to rest. Stun eases back to rest.
- Death is a staged loss of support, collapse, contact and settle. The final two
  sampled poses hold a grounded corpse. The whole connected body is resolved to the
  floor, rather than independently clamping nodes and tearing joints. Arthropods
  lose leg support and fold their feet inward/up through a second IK solve. Spectral
  opacity can dissipate separately; physical corpses retain their material alpha.
- One-shot clips include phase 1.0; loops omit a duplicated endpoint. FOOT_CONTACT
  events use each actual foot's touchdown phase and name the foot in reference_id.

`src/motion.rs` owns the deterministic pose solver. `src/physics.rs` still assesses
stance with Rapier; its raw reaction API is retained for callers but is no longer
mixed into generated attack clips. Simulation joint anchors now respect anatomical
rest rotations, avoiding spurious first-step impulses.

## Visual regression inspection

Bake the same prompt and seed to CBOR packages before and after a change:

```sh
infernal bake3d-cbor 'orc with a sword' 42 /tmp/before/orc
infernal bake3d-cbor 'orc with a sword' 42 /tmp/after/orc
UV_CACHE_DIR=/tmp/uv uv run tools/animation_evidence.py /tmp/before /tmp/after /tmp/comparison
```

The tool uses package metadata instead of assuming frame offsets. It emits every
clip as a labelled before/after contact sheet and a timing-correct GIF, plus a
family-wide sheet and a machine-readable coverage manifest. Both versions use the
same pixel scale. Inspect all states and all directional atlas borders, especially
weapon extremes, long tails, wing spans and collapsed poses, before accepting a
visual change. The clip generator does not invent missing movement capabilities.

Tests cover finite poses, attachment distances, fixed limb lengths, loop sampling,
stance/swing contact, final grounded/held corpses, and complete attack travel.
