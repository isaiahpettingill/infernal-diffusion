# Engine-owned locomotion

The generator supplies capabilities and matching animation assets. Your engine
owns navigation, steering, target selection, collision response, movement state
transitions, jump trajectories, flight altitude and combat decisions. A mode's
position in an array does **not** mean “approach”, “escape” or a required order.

## Recommended Godot integration

The existing asynchronous GDExtension already generates complete monsters in
memory. Copy the addon and host binaries into your game, queue generation, then
consume the descriptor-first profile supplied by default on each returned package:

```gdscript
var generator := InfernalDiffusion.new()
var job := generator.generate_in_memory_async("limping goblin", 42)
# In your normal poll_result() handling, once result.ok is true:
var root: Dictionary = result.packages.back()
var locomotion: Dictionary = root.locomotion # contract_version == 1
for capability: Dictionary in locomotion.modes:
    # Choose one of YOUR controllers using category/modifiers.
    print(capability.category, capability.modifiers, capability.animation_id)
```

Contract version 1 returns `modes`, locomotion-only `clips`, and explicit unit labels.
It excludes `behavior`, attacks, tactical targets, and attack movement recipes.
Each mode includes `id`, `category`, `modifiers`, `animation_id`, and
`reference_speed`. Each clip keeps sprite frame IDs, durations, loop flags and
foot-contact events, with advisory root deltas named `reference_root_dx` and
`reference_root_dy`. Transitions such as takeoff, landing, jump and emergence
are available clips, not a prescribed state machine. Sprite atlas metadata and
images remain in `root.monster.sprites` and `root.sprites`.

Use `animation_id` to look up the clip. For a direction, the actual atlas frame
is `sprite_frame_id + direction_index * sprites.direction_stride`; do not treat
the animation ID or the mode's array index as a sprite frame index.

## Capability vocabulary

`MovementMode` adds only two protobuf fields: `category = 4` and
`modifiers = 5`. They are strings/arrays in CBOR and native Godot dictionaries.

- `GROUNDED`: surface-supported displacement
- `AERIAL`: airborne displacement, including hovering/levitation
- `SWIMMING`: aquatic displacement
- `CLIMBING`: surface-attached vertical traversal
- `BURROWING`: underground traversal
- `STATIONARY`: no locomotion; reference speed is zero

Modifiers are an unordered, deduplicated set of physical/style hints. Current
values include `BIPEDAL`, `QUADRUPEDAL`, `MULTILEGGED`, `SLITHERING`, `CRAWLING`,
`OOZING`, `FLOPPING`, `HOPPING`, `LIMPING`, `TRACKED`, `ROOTED_GAIT`, `UNDULATING`,
`WINGED`, `POWERED_FLIGHT`, `LEVITATING`, `MOUNTED`, `JUMP_CAPABLE`, and `FAST`.
`WINGED` versus `POWERED_FLIGHT` versus `LEVITATING` distinguishes flight style
without specifying how a game must implement it. `JUMP_CAPABLE` advertises jump
clips, not a jump height, landing target or ballistic model. `FAST` identifies a
faster gait variant, not a command to flee. No modifier makes unsupported terrain
traversable; the engine decides where and when a capability can be used.

An aquatic creature's grounded beached-flop mode remains `GROUNDED`; its fins do
not turn that particular animation into flight or swimming. Fin-bearing swimming
undulation is `SWIMMING`, never levitation. Anchored creatures expose only
`STATIONARY` modes and do not receive jump, flight, climbing, burrowing or mobility
attack clips. A mounted creature's movement clips describe the composite, using the deepest
mount's support/propulsion and gait in a stack. Rider-only fins, wings, limping or
anchoring do not transfer to the mount. `MOUNTED` is a descriptor qualifier, not a
replacement gait: a mounted spider still uses its arthropod phases.

## Units and root motion

- `MovementMode.speed` and profile `reference_speed`: generator length units per
  second. They are tuning references, not measured maximum physical speeds
- Serialized `AnimationFrame.root_dx/root_dy` and profile `reference_root_*`:
  generator length units over that frame's duration. Positive X is local forward;
  negative Y is up in the generator's 2D convention
- Frame durations and event timestamps: milliseconds
- `size.pixels_per_unit`: the baked 3D atlas's pixels per generator length unit,
  before view projection/foreshortening. A profile value of zero means this old
  or 2D package does not provide that conversion; it does not mean zero speed

Choose one explicit engine length scale: `engine_speed = reference_speed *
engine_units_per_generator_unit`. Godot 2D pixels and Godot 3D meters are not
interchangeable. Sprite framing dimensions are not creature size, and the 3D
atlas already includes morphology scaling. Do not multiply the sprite by
`morphology_scale` again. Controller-driven movement may ignore root deltas or
use them to adjust animation playback; root-motion-driven movement may consume
them after scaling/rotation. **Do not apply both velocity and root displacement**
for the same motion or the creature moves twice.

## Compatibility and migration

The on-disk format remains version 7 for mesh output and version 2 for flat
output. This is an additive change: legacy fields and protobuf numbers 1–3 are
unchanged, old protobuf readers skip fields 4–5, and CBOR readers should ignore
unknown keys. The C ABI remains version 1. No native shim API change is needed;
its recursive record visitor automatically includes new fields.

Older packages have empty/missing `category` and `modifiers`. Validation still
accepts them and the Godot profile labels their category `UNKNOWN`. Handle an
unknown category with an engine-selected fallback or skip it; ignore unknown
modifiers. Do not infer authoritative physical abilities from legacy mode IDs.

`Monster.behavior`, including `approach_mode`, `escape_mode`, aggression and
ranges, and mobility `AttackStep` targets are deprecated **integration** inputs.
They remain emitted for existing demo clients; they are not removed or executed
by the descriptor profile. `behavior` may be omitted entirely, and validation
checks it only when supplied. Existing attack/animation references remain valid.
Use the whole record only if you deliberately want those legacy demo hints;
new movement code should consume the default `package.locomotion` profile.
For metadata obtained outside the wrapper, the equivalent explicit adapter is
`InfernalDiffusion.locomotion_profile(monster_dictionary)`.

The typed Rust, Go, Odin and C# CBOR models and generated Godobuf reader include
the additive fields. Rust callers constructing `MovementMode` with a struct
literal must supply the two fields or add `..Default::default()`; wire backward
compatibility does not remove that source-level requirement. Regenerate readers after future schema changes using the
commands in [the addon README](README.md).
