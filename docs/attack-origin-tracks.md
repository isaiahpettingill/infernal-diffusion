# V2 posed attack-origin tracks

Every V2 attack descriptor includes `motion_samples`: baked attack-origin tracks
from the actual posed geometry. Use these when locating contact or emission
origins on deformed limbs. Static rest-pose colliders cannot describe an extended
tree arm or a phasing creature's moving attack tip.

Each `AttackOriginSample` contains:

- `time_ms`: clip-relative start time of the sampled frame
- `sprite_frame_id`: the direction-zero sprite frame for this sample
- `node_id`: the anatomical origin node
- `views`: projected `ColliderView` positions and radii for each view
- `active`: whether this frame interval overlaps a damaging/emission step

Samples cover every frame of the attack clip. More than one origin may be present
at the same time: the bilateral tree smash has two hand origins per frame, while
a shade's attack tracks its actual leading tip.

## Coordinates and frame lookup

View `x`, `y`, and `radius` are frame-local sprite pixels, with the same upper-left
origin and projection as ordinary `ColliderView`. Select the directional atlas
frame using:

```text
sample.sprite_frame_id + view.direction_index * monster.sprites.direction_stride
```

Do not treat these coordinates as world units. Apply the engine's sprite scale,
placement and orientation when mapping them into the game world.

## Active intervals

For melee, `active` is true when the sprite-frame interval overlaps a damaging
step's `[at_ms, at_ms + duration_ms)` window. For `SPAWN_PROJECTILE`, it identifies
the emission pulse, not the projectile's lifetime: a projectile step's duration
does not keep the origin active for that entire duration.

An active hit radius uses the positive attack-step radius converted with
`PIXELS_PER_UNIT`; otherwise the sample uses the local node contact radius.
Multiple active samples describe spatial coverage, not instructions to apply
multiple hits. The engine chooses its own hit deduplication and damage policy.

These are baked geometric descriptors. They do not select targets, move an actor,
or implement autonomous behavior. Import validation checks that references,
frame times, active states and projected values match the package contract.

## Whole-body damage receivers

Shade `V2Profile.body_motion_samples` contains one `BodyMotionSample` for every
attack frame, including recovery. Each record names the clip, clip-relative
`time_ms`, direction-zero `sprite_frame_id`, and every damage-receiving node's
`PosedHurtbox`. The four projected views use the coordinates described above.

These receiver envelopes use the same whole-surface deformation field as the
renderer. Replace rest-pose hurtboxes with the current frame's receivers while
unwinding; otherwise fire and water could hit the absent torso instead of the
visible ink. Mirrored recovery frames reuse exactly the same geometry and views.
The circles are conservative sampled envelopes, not exact triangle collision.
Offensive `motion_samples.active` remains separate from receiving damage, so
recovery stays vulnerable without creating another outgoing hit.

Non-shade profiles omit this array. Import validation rejects missing frames,
unknown or duplicate nodes, inconsistent frame IDs/times and invalid projections.
