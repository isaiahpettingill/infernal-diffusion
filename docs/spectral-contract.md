# V2 spectral interaction contract

`Monster.v2_profile.spectral` is optional additive metadata (protobuf tag 7).
Spectral creatures advertise `PHASING`; their descriptor defines damage responses
and collision behavior separately from visual opacity. Engines must not infer
immunity from alpha or make every undead creature intangible.

## Damage rules

Evaluate `damage_rules` in descending `priority`; the first matching rule wins.
Every nonempty condition must match:

- `elements`: the incoming element must match any listed element
- `delivery`: the incoming delivery must match exactly
- `required_tag`: the incoming attack must include this tag

Empty conditions impose no restriction. Apply the matching rule's
`damage_multiplier` and `projectile_pass_through`. If no rule matches, use
`default_damage_multiplier` and `default_projectile_pass_through`. Pass-through
applies to projectile delivery only. These rules are distinct from solid-world
collision and do not tell the engine which actor to target.

The Rust reference evaluator is
`v2::spectral::damage_response(descriptor, element, delivery, tags)`.

### Shade

A shade is opaque black regardless of source anatomy. Its attacks are only
`v2_shade_tendril_lash` and `v2_shade_tendril_thrust`. The entire surface, including
head, torso and limbs, continuously unwinds into three connected tapered strands.
The same baked frames are replayed in reverse to reform the original creature;
recovery does not deal a second hit. Coherent seeded deformation and bounded 1.8×
strand growth keep the strike readable. `WHOLE_BODY_UNWIND_REVERSE` and
`BOUNDED_SHADE_TENDRIL_EXTENSION_MAX_1_8` explicitly identify this motion.
Original gun/bite/summon attacks and unused offensive frames are removed.

The ordinary clip lasts 1400 ms, with its outward hit at 650 ms and recovery
starting at 750 ms. Its solid-phasing window is [150, 1350) ms. Mutant timing may
add bounded holds; consumers must use each package's exported events and windows.
[Posed damage receivers](attack-origin-tracks.md#whole-body-damage-receivers)
follow the disappearing body and visible strands, including during recovery.

The priority-100 FIRE/WATER exception takes precedence over the priority-10
projectile rule. FIRE and WATER projectiles deal ordinary damage and do not pass
through. Other projectiles pass through and deal zero damage. Non-projectile
attacks use ordinary damage unless another applicable rule says otherwise.

### Ghost

A MAGIC tag (priority 100) or MAGIC element (priority 90) permits ordinary damage
and stops projectile pass-through, regardless of visual appearance. Other
attacks deal zero damage; nonmagical projectiles pass through.

### Ghoul

Ordinary damage applies by default. Being a ghoul does not confer blanket damage
immunity or projectile pass-through. Its phasing is limited to declared windows.

## Solid-world phasing

`collision_mode` identifies always-intangible behavior or bounded phase windows.
Each `PhaseWindow` references an animation ID and a half-open clip-relative
interval `[start_ms, end_ms)`. When `solid_pass_through` is true, an engine may
allow passage through solid obstacles during that interval; at `end_ms` the
window has ended. The Rust reference helper is
`v2::spectral::passes_solids(descriptor, animation_id, time_ms)`. Ghosts use `ALWAYS_INTANGIBLE`; shades and ghouls use
`PHASE_WINDOWS`.

`PHASE_BEGIN` and `PHASE_END` animation events reference the actual attack ID
bound to the clip. Import validation requires a single paired interval, valid
attack references and times within the clip. Visual transparency does not replace
this collision contract. The engine remains responsible for safe placement when
solid collision resumes.

## Serialization and older packages

Descriptors and rules survive protobuf, CBOR and JSON roundtrips. Packages
without the extension omit `spectral` when serialized as JSON and retain their
existing behavior. Consumers should inspect the optional descriptor rather than
assuming every V2 package is spectral.
