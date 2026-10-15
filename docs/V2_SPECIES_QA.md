# V2 species, modifiers, and weapons QA

Requested October 15, 2026. Scope is the opt-in V2 procedural pipeline; V1 remains the default and its visuals remain unchanged. These are procedural improvements, not a newly trained model.

## Status convention

Checked items are implemented and covered by the automated or rendered evidence below. Visual review is a recognizability review, not a claim of photorealism. The final CI item remains open until the published commit finishes all workflows.

## Anatomy and species

- [x] Orc: more believable anatomy, recognizable jaw/tusks and humanoid silhouette
- [x] Troll: distinct from orc, more believable mass, face and proportions
- [x] Deliberate seeded AI-style anomalies: recognizable duplicated/misplaced features or unusual joints/limbs instead of shapeless noise; viable connected animation rigs
- [x] Wolf: distinct canine head, muzzle, ears, body, paws and tail
- [x] Lion: clearly feline, recognizable mane/head and body distinct from wolf
- [x] Tiger: feline build with readable stripes, distinct from lion
- [x] Bear: heavy body/paws, rounded ears and short tail
- [x] Other animal recipes: review family coverage and explicitly list any untreated families
- [x] Crab: broad carapace, claws and lateral leg arrangement
- [x] Lobster: actual recipe selection, long segmented body, claws and tail fan; no humanoid fallback
- [x] Hydra: recognizable multi-headed creature with coherent necks
- [x] Cyclops: single prominent eye
- [x] Minotaur: recognizable bull-headed humanoid
- [x] Cerberus: recognizable three-headed dog
- [x] Historical figure: improve existing recipe's face/clothing identity; document supported interpretations
- [x] Anime woman: clearly adult, clothed, busty manga proportions and exaggerated hair/profile-picture aesthetic; remove schoolgirl/youth cues
- [x] Rhino: prominent nose horn and heavy distinctive body
- [x] Dinosaurs: distinctive dinosaur builds rather than generic quadrupeds
- [x] Dragons: recognizable head, wings, tail and scale cues
- [x] Elephant: trunk and clear distinctive tusks
- [x] Mammoth: large curled tusks and recognizable mammoth head/shaggy silhouette
- [x] Snake: coherent serpentine shape and movement
- [x] Gorgon: distinctive gorgon traits
- [x] Worm: distinct worm shape and motion
- [x] Sand worm: distinct large sand-worm shape and motion
- [x] Grendel: large generic humanoid monster with exactly one arm
- [x] Potato monster: potato-bodied character with face/limbs that throws potato projectiles
- [x] Cacodaemon / floating head demon: aliases produce a floating demonic head
- [x] Devils and demons: recognizable horns
- [x] Reaper: hood and scythe
- [x] Alien: distinctly sci-fi visual features
- [x] Robot: recognizable geometric mechanical parts, deliberately jumbled assembly to convey fictional limited training data
- [x] Coconut / sphere / ball / dot / circle: small round projectile-throwing creatures; token-aware aliases

## Composable prompt modifiers

- [x] Cute and chibi are aliases for huge head, compact body and short limbs
- [x] Cute elephant preserves elephant identity and ordinary melee damage
- [x] Cute/chibi alone does not imply fake-friendly lure behavior or change combat type
- [x] Zombie applies gray coloration and missing limbs across applicable creature anatomies, with viable locomotion and attacks; document limbless case
- [x] Ninja applies a mask and faster movement while preserving base creature identity
- [x] Hot / flaming / fire applies fire damage and appropriate flaming melee weapon/strike or fire projectiles
- [x] Water adds swimming idle, movement, turn and attack animations to the chosen creature
- [x] Explicit rapper / dancer replaces active fighting with hip-hop dancing and retains passive contact damage
- [x] Dancing has a deterministic approximately 1/1000 random chance across V2 monsters; explicit request always enables it
- [x] Dances have coordinated rhythmic poses, weight shifts and clean loops adapted to biped, quadruped, limbless and floating bodies
- [x] Dancers have dollar-sign necklaces and sunglasses, including randomly selected dancers
- [x] Sunglasses in the dancer context do not accidentally trigger spectral shade behavior

## Weapons and attacks

- [x] Laser cannon: improved readable geometry
- [x] All anatomically applicable gun users have coherent two-handed grip through idle, firing and recoil; retain hybrid support
- [x] Swords: improved silhouette, grip and attack placement
- [x] Axes: improved silhouette, grip and attack placement
- [x] Chainsaws: recognizable body, handle, bar and teeth
- [x] Spears: recognizable shaft/head and coherent held placement
- [x] Some spear users throw: readable wind-up, release and recovery; others retain thrust attacks
- [x] Thrown spear held-mesh visibility and projectile release timing/origin are consistent
- [x] Reaper scythe is recognizable and properly held
- [x] Dancer behavior suppresses active attacks, including projectile and spear-throw attacks

## Verification and compatibility

- [x] Public embedded-BERT and vocabulary/WASM paths exercise every new alias/modifier
- [x] Combined modifier examples tested (cute elephant, water bear, fire orc, ninja variants, zombie variants, dancing variants)
- [x] Explicit requested limb counts and one-armed Grendel are not overwritten by random anomalies
- [x] Hybrids, moving attack origins, shades and reverse-unwind animations remain valid
- [x] V1 byte-compatibility tests pass
- [x] Deterministic seed replay passes; rare dance frequency tested on a sufficiently large cheap semantic sample
- [x] Before/after actual renders across multiple seeds, including 96–128px game-sized previews
- [x] Idle, locomotion and attacks reviewed in motion, not only stills
- [x] Dance GIFs reviewed for foot sliding, disconnected limbs and loop seams
- [ ] Default/native BERT, no-default, WASM/browser and applicable Godot build checks pass on exact published head (CI pending publication)
- [x] Engine-owned behavior is clearly documented: exported damage, movement, projectile and passive-contact descriptors do not by themselves prove Destructos runtime implementation
- [x] Final PR contains precise completed/pending scope and known limitations

## Evidence log

Baseline: 22 recipes across three seeds; lobster fell back to a humanoid and wolf/lion silhouettes were nearly identical. Final native catalog: 45 named prompts × seeds 0, 7 and 42. The checked-in sheets are cropped from actual baked atlases, using 128px presentation canvases plus nearest-neighbor detail views. Native frame sizes vary by recipe.

- [Animal coverage, before/after, three-seed and movement evidence](v2-species/ANIMAL_QA.md)
- [Dance, weapon, spear release and face evidence](v2-species/MOTION_QA.md)
- [Humanoid stills](v2-species/humanoids.png), [animal stills](v2-species/animals.png), [fantasy stills](v2-species/fantasy.png), [modifier stills](v2-species/modifiers.png)
- Public generated-package integration: `tests/v2_species_contracts.rs`; shared vocabulary/BERT alias and modifier tests: `src/v2/recipe_extensions.rs`
- Rare dance: deterministic 100,000-seed semantic sample, bounded 65–140 occurrences; explicit dancers bypass rarity
- Chibi dimensions: compact torso at 42% forward extent, skull at approximately 136% forward extent and 242% height; attached trunk shortened separately so the skull, rather than only its appendage, dominates
- Local V1 compatibility: 11 representative packages byte-identical to pinned baseline, including every PNG and protobuf export
- Web combat/profile tests: 27 passed; passive CONTACT damage is handled by the included Arena simulation

## Known limits and interpretation

- Historical figure uses the existing `adolph hitler` recipe. This is not a general historical-person identification system.
- Orc/troll remain stylized low-poly pixel sprites. Seeded anomalies include a coherent oversized six-finger hand, not independent random vertex destruction. Existing explicit limb-count constraints remain authoritative; Grendel always has one arm.
- Zombie removes one available limb unless an explicit count overrides it; naturally limbless creatures lose a terminal appendage where present. It does not invent an expendable limb.
- Red is the default V2 crab/lobster shell, with deterministic red variation. Explicit nondefault material selections remain available.
- Fish/cetaceans, amphibians, bats and non-crustacean insects/arachnids keep existing family geometry. Shared bird/ungulate/rodent surface improvements are not individually redesigned skeletons for every animal name.
- Water reuses swimming clip generation; it does not imply a fluid simulation. Dances preserve connected rigid rig edges, but do not have world-space planted-foot IK.
- Damage, Ninja movement speeds and projectiles are exported runtime contracts. The included Arena implements passive contact, but this PR does not modify or establish external Destructos engine support. Spear playback evidence is an exported-metadata harness.
- V1 is still the default. All new semantic recipes and rendering overrides are gated on V2.

