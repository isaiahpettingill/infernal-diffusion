//! Seeded morphology for prompts with no recognizable creature affinity.

use crate::{anatomy, parser::MonsterSpec};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

const ARCHETYPES: &[(&str, &str, u8, u8)] = &[
    ("HUMANOID", "ORC", 2, 2),
    ("HUMANOID", "TROLL", 2, 2),
    ("HUMANOID", "CYCLOPS", 2, 2),
    ("QUADRUPED", "WOLF", 4, 0),
    ("QUADRUPED", "BULL", 4, 0),
    ("QUADRUPED", "LION", 4, 0),
    ("QUADRUPED", "RHINO", 4, 0),
    ("ARACHNID", "SPIDER", 8, 0),
    ("ARACHNID", "SCORPION", 8, 0),
    ("ARACHNID", "CENTIPEDE", 12, 0),
    ("INSECT", "HORNET", 6, 0),
    ("INSECT", "SCARAB", 6, 0),
    ("SERPENT", "COBRA", 0, 0),
    ("SERPENT", "SAND_WORM", 0, 0),
    ("GASTROPOD", "SLUG", 0, 0),
    ("GASTROPOD", "SNAIL", 0, 0),
    ("WINGED", "DRAGON", 4, 0),
    ("WINGED", "HARPY", 2, 2),
    ("FLOATING", "GHOST", 0, 0),
    ("AMORPHOUS", "BLOB", 0, 0),
    ("FUNGUS", "MUSHROOM", 0, 0),
];

const MATERIALS: &[&str] = &[
    "FLESH", "FUR", "SCALES", "CHITIN", "BONE", "STONE", "SLIME", "TUMOR", "SPECTRAL", "FEATHER",
    "WRAPPING",
];
const FEATURES: &[&str] = &[
    "HORN",
    "TAIL",
    "CLAW",
    "FANG",
    "MANY_EYES",
    "TENTACLE",
    "SHELL",
    "PUSTULE",
    "ANTLER",
    "TUSK",
    "SMOKE",
    "FLAME",
    "WING",
];
const ELEMENTS: &[&str] = &["FIRE", "ICE", "POISON", "ACID", "SHADOW", "LIGHTNING"];

pub fn fill_unknown(spec: &mut MonsterSpec, rng: &mut ChaCha8Rng) -> bool {
    if spec.affinity != "UNKNOWN" {
        return false;
    }
    let bare = spec.size == 0.55
        && spec.bulk == 0.5
        && spec.materials == ["FLESH"]
        && spec.features.is_empty()
        && spec.secondary_affinities.is_empty()
        && spec.attack.concept == "MELEE"
        && spec.attack.element == "PHYSICAL";
    let &(plan, affinity, legs, arms) = &ARCHETYPES[rng.random_range(0..ARCHETYPES.len())];
    spec.body_plan = plan.into();
    spec.affinity = affinity.into();
    spec.limb_count = legs;
    spec.arm_count = arms;
    spec.heads = if rng.random_bool(if bare { 0.42 } else { 0.22 }) {
        rng.random_range(2..=4)
    } else {
        1
    };
    if arms > 0 && rng.random_bool(0.38) {
        spec.arm_count = [2, 4, 6][rng.random_range(0..3)];
    }
    if legs > 0 && plan != "ARACHNID" && rng.random_bool(0.18) {
        spec.limb_count = (legs + 2).min(12);
    }
    if spec.size == 0.55 {
        spec.size = rng.random_range(0.18..0.96);
    }
    if spec.bulk == 0.5 {
        spec.bulk = rng.random_range(0.2..0.88);
    }
    if spec.materials == ["FLESH"] {
        spec.materials[0] = if rng.random_bool(0.58) {
            anatomy::material_for_affinity(affinity).into()
        } else {
            MATERIALS[rng.random_range(0..MATERIALS.len())].into()
        };
    }
    if rng.random_bool(if bare { 0.68 } else { 0.35 }) {
        let candidate = ARCHETYPES[rng.random_range(0..ARCHETYPES.len())].1;
        if candidate != affinity {
            spec.secondary_affinities.push(candidate.into());
        }
    }
    let additions = if bare {
        rng.random_range(1..=3)
    } else {
        rng.random_range(0..=2)
    };
    for _ in 0..additions {
        let feature = FEATURES[rng.random_range(0..FEATURES.len())];
        if !spec.features.iter().any(|existing| existing == feature) {
            spec.features.push(feature.into());
        }
    }
    if spec.attack.concept == "MELEE" && spec.attack.element == "PHYSICAL" {
        if rng.random_bool(if bare { 0.48 } else { 0.28 }) {
            spec.attack.delivery = "PROJECTILE".into();
            spec.attack.concept = "PROJECTILE".into();
            spec.attack.element = ELEMENTS[rng.random_range(0..ELEMENTS.len())].into();
        } else if rng.random_bool(0.28) {
            spec.attack.element = ELEMENTS[rng.random_range(0..ELEMENTS.len())].into();
        }
    }
    spec.parser.push_str("+seeded-mystery-v1");
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use std::collections::BTreeSet;

    #[test]
    fn unmatched_prompts_vary_across_seed_but_repeat_exactly() {
        let original = crate::parser::parse_vocabulary("purple existential paperwork");
        let mut plans = BTreeSet::new();
        let mut signatures = BTreeSet::new();
        for seed in 0..96 {
            let mut first = original.clone();
            let mut second = original.clone();
            assert!(fill_unknown(
                &mut first,
                &mut ChaCha8Rng::seed_from_u64(seed)
            ));
            assert!(fill_unknown(
                &mut second,
                &mut ChaCha8Rng::seed_from_u64(seed)
            ));
            assert_eq!(
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            );
            assert_ne!(first.affinity, "UNKNOWN");
            assert!(first.confidence < 0.5);
            plans.insert(first.body_plan.clone());
            signatures.insert(serde_json::to_string(&first).unwrap());
        }
        assert!(plans.len() >= 8);
        assert!(signatures.len() >= 90);
    }

    #[test]
    fn semantic_constraints_survive_random_body_selection() {
        let mut partial = crate::parser::parse_vocabulary("huge bone thing that shoots fire");
        assert_eq!(partial.affinity, "UNKNOWN");
        assert!(fill_unknown(
            &mut partial,
            &mut ChaCha8Rng::seed_from_u64(3)
        ));
        assert_eq!(partial.size, 0.9);
        assert_eq!(partial.materials[0], "BONE");
        assert_eq!(partial.attack.delivery, "PROJECTILE");
        assert_eq!(partial.attack.element, "FIRE");
        let mut known = crate::parser::parse_vocabulary("wolf");
        assert!(!fill_unknown(&mut known, &mut ChaCha8Rng::seed_from_u64(3)));
        assert_eq!(known.affinity, "WOLF");
    }
}
