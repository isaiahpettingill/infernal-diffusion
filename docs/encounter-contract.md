# V2 encounter contracts

V1 is unchanged. Recipe identity `profile.v2@4` invalidates earlier V2 caches.
V2 adds opt-in recipes `splitting slime`, `bush mimic`,
`large banana mimic`, `vase mimic`, `bronze shield sentinel`, and `boulder pusher`.
`amphora`, `amorpha`, `hoplite`, `Talos`, and `Sisyphus` are supported aliases.
The first batch has fixed anatomy; unsupported explicit head/limb counts or
mounted variants fail clearly before rendering instead of silently ignoring them.
Mimics have only bush, large-banana, and vase disguises. A generic mimic uses a vase.

## Transport and ownership

`monster.v2_profile.encounter` is optional. Its `contract_version` is **1**;
`archetype` selects exactly one typed payload: `split`, `mimic`, `shield`, or
`pushed_prop`. Protobuf field 10, CBOR, JSON, Godot and generated language bindings
carry identical field names. Older packages omit it and keep existing behavior.
Importers must reject unknown versions/archetypes and invalid/unbounded values,
or safely ignore the whole unsupported optional capability. Never guess from the
prompt text or sprite colors. The Rust validator checks bounds and clip references.

This is capability/asset metadata, not an embedded AI program. The game owns
navigation, timing policy, world collision, targeting, health, rewards, save IDs,
and safe spawning. Lengths are native generated-sprite/world pixels before the
consumer's actor scale; durations are milliseconds, angles degrees. The game
can adapt advisory physics values to its difficulty and world scale.

## Splitting slime

`archetype: SPLITTING_SLIME`; `split` declares two children, at most two splitting
generations and six descendants per original slime. Child scale is 0.6, maximum
health is 0.3 of the parent's maximum, and speed is 1.3 times the parent speed.
Each wave's total child health is below its parent's. A child inherits its
ancestor's remaining family budget; it must not start a new root budget.

`v2_split_burst` is visual death separation. `v2_emerge` lasts 720ms and is a
non-damaging entrance. `separation_px: 28` is minimum intended separation; the
engine must find safe terrain positions and avoid overlapping actors, or skip a
child when blocked. Neither clip contains summon steps: the game creates children
once on death, with stable child IDs. Persist the consumed split/reward marker,
child IDs, generation and remaining budget before respawn/reload can run death
logic again. Rendered child geometry/atlas may be reused at the declared scale.

## Object mimics

`archetype: OBJECT_MIMIC`; `mimic.disguise` is `BUSH`, `LARGE_BANANA`, or `VASE`.
The existing `lure` descriptor references `v2_lure` and `v2_reveal` with a 110px
trigger radius and 720ms harmless telegraph. Here `effect: OBJECT_DISGUISE`
replaces hearts, and `reset_policy: STAY_REVEALED` avoids magically redisguising a
known attacker. The `OBJECT_DISGUISE` event marks the closed loop at time zero.
Only `v2_lure` is closed; normal idle/move clips show the revealed mouth.
The engine persists whether the mimic has revealed. Reveal completion permits
ordinary close-range attacks; closed/reveal frames do not deal contact damage.

## Bronze shield sentinel

`archetype: SHIELD_SENTINEL`; `shield.front_arc_degrees: 120` describes the full
frontal guard cone, centered on the actor's facing. Eligible incoming melee and
projectiles block during guard, not during the bash/recovery vulnerability.
The engine evaluates the source/velocity direction; rear hits bypass the shield.
`v2_guard` is the held guard pose. `v2_shield_bash` reaches impact at 400ms;
`v2_shield_recovery` exposes the core for 720ms. The typed animation references
avoid identifying combat phases by image appearance.

## Boulder pusher and independent pushed entity

`archetype: BOULDER_PUSHER`; `pushed_prop.kind: BOULDER` is a **separate world
entity**, never an attachment or baked rock in the pusher atlas. The pusher
sprite contains only the actor. `id_suffix: boulder` is a role key, not a globally
unique ID: the game derives/stores its own stable link from the pusher UUID.

`coordinate_space: LOCAL_PIXELS_X_FORWARD_Y_DOWN` means `anchor_x: 42` points
forward from the actor's feet and `anchor_y: -24` points above them. Mirror X
with facing and scale both coordinates with actor scale. The rock has radius
24px, advisory maximum health 45 and push speed 150px/s. It has independent
position, velocity, collision, health, destruction, rendering and persistence.
Moving/destroying it cannot move/destroy the pusher as a side effect.

`v2_push` emits one `PUSH_RELEASE` event at 720ms with `reference_id: boulder`.
This event is an impulse opportunity for the engine, not a projectile/spawn
primitive. The engine uses the linked rock if it is close enough, and handles
retrieve/reset/respawn policy and safe placement itself. `v2_push_recovery`
provides a 720ms vulnerable visual. Ordinary short-range shove clips remain as
fallback attacks. Do not both launch from the event and independently spawn a
second rock from the attack list. Persist rock state and the owner/link state;
a destroyed/absent linked rock must not duplicate after room reload.

## Compatibility and testing

`tests/encounter_contracts.rs` covers identity, strict disguise allowlist,
Protobuf/CBOR/JSON roundtrips, absent extensions, invalid growth budgets,
nonfinite values, missing clips and missing/duplicate/late rock release events.
The consuming Destructos changes separately test real Godot physics, defense,
death splitting and room save/restore. Visual clips alone do not implement those
behaviors in other engines.

## Damage categories

`v2_profile.damage_traits` (Protobuf field11) is a bounded, deduplicated list of
`SKELETAL`, `GELATINOUS`, `ARMORED`, `UNDEAD`, `WOODEN`, `SPECTRAL`, and
`FIRE_ATTUNED`, derived from interpreted semantic anatomy/materials/features.
It has no damage multipliers: the engine owns balance and authored overrides.
Existing `spectral.damage_rules` and phase windows retain their priority for
shade/ghost interactions. An omitted list means no declared trait; importers
should not infer defenses from sprite palette or names.
