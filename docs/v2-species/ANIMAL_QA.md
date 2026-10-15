# V2 animal surface review

Procedural mesh changes, October 15, 2026. These surfaces are selected only by
V2's render override. They retain the native node IDs, parent links, attachment
slots, material roles, animation tracks and metadata schema. No generated bitmap
assets or edits to V1 mesh assets are involved.

## Coverage

`src/v2/animal_meshes.rs` provides 67 cached surfaces: canine and feline skulls,
a distinct lion mane, bear, horse, bovine/minotaur, goat, boar, rabbit, rodent,
bird head groups, crocodile, dragon/hydra, theropod and herbivore dinosaur heads,
elephant/mammoth/rhino heads, species body and tail profiles, paws and hooves,
trunk, curved elephant and curled mammoth tusks, rhino/bull/goat/ram horns,
crab/lobster shells and pincers, lobster tail fan, snake/cobra heads and segments,
gorgon snake tips, separate worm and sand-worm heads, annulated worm tubes,
broad stegosaurus plates, and a bowed triceratops frill and solid brow horns.
Cerberus/Fenrir reuse the canine surface on each native head; hydra reuses the
reptilian surface. Native feature nodes own horns, tusks and trunks; these are
not duplicated inside their corresponding head meshes.

V2 anatomy refinements additionally adjust head proportions, thick elephant
trunks and continuous low worm dimensions. Body placement, limb arrangements,
crustacean recipes, tiger stripes and other semantic changes are documented in
the parent species QA report.

This does not claim a full redesign of every animal recipe. Fish/cetaceans,
amphibians, bats and non-crustacean insects/arachnids retain their existing
family-specific geometry. Some shared bird/ungulate/rodent families receive
head/body profiles without an individually redesigned skeleton for each name.

## Automated checks

`cargo test --no-default-features v2_animals -- --nocapture`: 7 tests pass.
Checks cover finite vertices and valid indices; distinct canine/feline landmarks;
tucked bodies and bear withers; composite head reuse; separate snake/worm/maw
heads; native feature-node ownership; and volumetric tube/plate/frill/horn forms.
The all-target no-default Clippy run also passes with warnings denied.
A final atlas-boundary audit checked 3756 idle, move and attack frames across
13 recipes, three seeds and four directions, with zero nontransparent edge hits.
These tests establish mesh contracts and bounds, not visual quality. Build/profile and V1
compatibility coverage are recorded by the overall species QA owner.

## Actual renderer evidence

The species catalog renders seeds 0, 7 and 42 with the real V2 atlas generator.
The comparisons use the same names and seeds as the earlier captured baseline.
Mammoth has no captured before image; lobster's before image demonstrates the
old humanoid recipe fallback. Contact sheets enlarge original atlas pixels with
nearest-neighbor sampling. GIFs preserve native atlas scale and frame position.
Atlas frame dimensions vary by species (96–176 pixels). The native128 sheet
crops only transparent padding and centers each unscaled sprite in a 128px
viewport; the GIFs retain the original full frame and motion placement.

- `animals-native128.png`: original sprite pixels in 128px viewports, seed 0
- `animals-before-after.png`: nine representative animal recipes, seed 0, 3x
- `animals-three-seeds.png`: those nine recipes at seeds 0, 7 and 42, 2x
- `animals-walk-native.gif`: fixed native-scale viewports, real locomotion frames
- `animals-attack-native.gif`: fixed native-scale viewports, real attack frames
- `animals-worm-dinosaur-details.png`: worm and dinosaur silhouette review

The review prioritizes visible muzzle/ear, mane, trunk/tusk, shell/claw, plate and
worm silhouette differences. Motion evidence checks that these attached forms
remain coherent through the existing clips; it does not establish game-engine
collision, behavior, or damage implementation.
