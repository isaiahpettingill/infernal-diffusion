# Aquatic animation correction (V1 and V2)

Water-capable creatures use true swimming clips rather than ground slithering,
beached flopping or planted feet. This is an intentional compatibility correction
for aquatic outputs in both profiles. Terrestrial V1 generation is unchanged.
The bottom mount determines propulsion; rider-only fins cannot turn a land mount
into a swimmer. Anchoring takes precedence over swimming.

## Engine contract

For obligate swimmers the existing `move` and `fast_move` clips bind movement
modes `SWIM` and `FAST_SWIM`. Amphibious frogs/toads, fishfolk and limbed land
creatures with aquatic traits retain their ground clips and receive additional
`swim` and `fast_swim` clips bound to those modes. Additional water modes bind
clips of the same lowercase name:

- `WATER_HOVER`: stationary treading/hovering, zero travel
- `WATER_RISE`: upward travel, negative `root_dy`
- `WATER_DIVE`: downward travel, positive `root_dy`
- `WATER_TURN`: in-place turning, zero travel; the engine chooses heading

Every mode declares category `SWIMMING`. Propulsion modifiers are `UNDULATING`,
`KICKING` (frogs), `PULSING` (jellyfish), or `JET_PROPULSION` (kraken). Fast and
mounted modifiers remain independent. Each mode links a real baked clip.

The `water_primary` attack links `attack_water`, retaining the primary attack's
origin, element, damage, telegraph and gameplay steps. V2 may independently upgrade
its primary attack to a projectile pattern; both attack links remain valid.

Root deltas describe reference travel in generator units per frame, consistently
with mode speed and frame duration. Rendered poses remain in-place. The consuming
engine chooses whether to integrate these deltas, use the reference speed, or
apply its own water controller. It owns water-volume detection, transitions,
turning direction, navigation, targets, buoyancy, and collision response. No path
planning or AI decisions are embedded in the generator.

## Motion guarantees

Fish use a traveling lateral tail wave in the 3D renderer. Mermaids drive their
connected fish tail; cetaceans use a vertical tail stroke. Frogs use synchronized
hind-leg kicks and recovery. Jellyfish trail tentacles during bell propulsion;
kraken use phase-delayed tentacle strokes. Hover uses reduced-amplitude propulsion,
and rise/dive add a pitch while the engine applies vertical travel.

Swimming poses never run foot-contact IK, emit footfall events, or snap their
corpse to a ground plane. Joint lengths stay fixed; attachment details inherit
the parent transform. Loop phases omit duplicate endpoints but match at the
mathematical seam. One-shot attack strokes recover to neutral.

Water affinity includes fish/eels/sharks, cetaceans, sea serpents/dragons,
mermaids/mermen/fishfolk, frogs/toads, jellyfish and kraken/cephalopods. Explicit FIN, SWIM or
AQUATIC traits also enable water locomotion. Frog/toad vocabulary normalization is
available in V2; a programmatic FROG/TOAD spec also receives swimming in V1.
