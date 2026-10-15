//! V2-only semantic recipes; shared by BERT and vocabulary entry points.
use crate::parser::MonsterSpec;

pub(super) fn interpret(mut spec: MonsterSpec, prompt: &str) -> MonsterSpec {
    let lower = prompt.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |w: &str| words.contains(&w);
    let explicit_legs = spec.features.iter().any(|f| f == "EXPLICIT_LEGS");
    let explicit_arms = spec.features.iter().any(|f| f == "EXPLICIT_ARMS");
    let mut humanoid = None;
    if has("grendel") {
        humanoid = Some("GRENDEL");
    }
    if has("reaper") {
        humanoid = Some("REAPER");
    }
    if has("devil") {
        humanoid = Some("DEMONIC");
    }
    if let Some(affinity) = humanoid {
        spec.affinity = affinity.into();
        spec.body_plan = "HUMANOID".into();
        if !explicit_legs {
            spec.limb_count = 2;
        }
        if !explicit_arms {
            spec.arm_count = 2;
        }
        spec.confidence = 1.0;
    }
    if has("grendel") {
        // Grendel's missing arm is recipe identity, not random damage.
        spec.arm_count = 1;
        spec.features.push("EXPLICIT_ARMS".into());
        spec.bulk = 0.85;
        spec.size = 0.8;
    }
    if has("reaper") {
        if !spec.features.iter().any(|f| f == "EXPLICIT_ATTACK") {
            spec.attack.concept = "SCYTHE".into();
            spec.attack.delivery = "MELEE".into();
            spec.attack.element = "PHYSICAL".into();
        }
        spec.features.push("HOODED".into());
    }
    if has("cacodaemon") || has("cacodemon") || (has("floating") && has("head") && has("demon")) {
        spec.affinity = "CACODAEMON".into();
        if !["metal", "wood", "stone", "ice", "ghost", "spectral"]
            .iter()
            .any(|w| has(w))
        {
            spec.materials = vec!["FLESH_RUST".into()];
        }
        spec.body_plan = "FLOATING".into();
        if !explicit_legs {
            spec.limb_count = 0;
        }
        if !explicit_arms {
            spec.arm_count = 0;
        }
        spec.features.push("HORN".into());
        spec.confidence = 1.0;
    }
    if has("worm") && spec.affinity != "SAND_WORM" && !has("sand") {
        spec.affinity = "WORM".into();
        spec.body_plan = "SERPENT".into();
        if !explicit_legs {
            spec.limb_count = 0;
        }
        if !explicit_arms {
            spec.arm_count = 0;
        }
        spec.confidence = 1.0;
    }
    if ["coconut", "sphere", "ball", "dot", "circle"]
        .iter()
        .any(|w| has(w))
    {
        spec.affinity = "ROUNDLING".into();
        spec.body_plan = "HUMANOID".into();
        spec.size = 0.2;
        if !explicit_legs {
            spec.limb_count = 2;
        }
        if !explicit_arms {
            spec.arm_count = 2;
        }
        spec.confidence = 1.0;
        if !spec.features.iter().any(|f| f == "EXPLICIT_ATTACK") {
            spec.attack.delivery = "PROJECTILE".into();
            spec.attack.concept = "ROUND".into();
            spec.attack.element = "PHYSICAL".into();
        }
    }
    if has("rapper") || has("dancer") {
        spec.features.push("DANCER".into());
    }
    if has("ninja") {
        spec.features.push("NINJA".into());
    }
    if ["hot", "flaming", "fire", "fiery", "burning"]
        .iter()
        .any(|w| has(w))
    {
        spec.attack.element = "FIRE".into();
        spec.features.push("FLAMING".into());
    }
    if has("water") {
        spec.features.push("SWIM".into());
    }
    if has("potato") {
        spec.affinity = "POTATO".into();
        spec.body_plan = "HUMANOID".into();
        if !explicit_legs {
            spec.limb_count = 2;
        }
        if !explicit_arms {
            spec.arm_count = 2;
        }
        spec.materials = vec!["FLESH_OCHRE".into()];
        spec.confidence = 1.0;
        if !spec.features.iter().any(|f| f == "EXPLICIT_ATTACK") {
            spec.attack.delivery = "PROJECTILE".into();
            spec.attack.concept = "POTATO".into();
            spec.attack.element = "PHYSICAL".into();
        }
    }
    if has("zombie") || has("zombified") {
        if spec.affinity == "ZOMBIE" {
            let stripped = words
                .iter()
                .filter(|w| !matches!(**w, "zombie" | "zombified"))
                .copied()
                .collect::<Vec<_>>()
                .join(" ");
            let base = crate::parser::parse_vocabulary(&stripped);
            if base.confidence >= 0.68 && base.affinity != "UNKNOWN" {
                spec.affinity = base.affinity;
                spec.body_plan = base.body_plan;
                if !explicit_legs {
                    spec.limb_count = base.limb_count;
                }
                if !explicit_arms {
                    spec.arm_count = base.arm_count;
                }
                spec.heads = base.heads;
            }
        }
        spec.features.push("ZOMBIFIED".into());
    }
    if matches!(spec.affinity.as_str(), "DEMONIC" | "CACODAEMON")
        && !spec.features.iter().any(|f| f == "HORN")
    {
        spec.features.push("HORN".into());
    }
    if ["hot", "flaming", "fire", "fiery", "burning"]
        .iter()
        .any(|w| has(w))
    {
        spec.attack.element = "FIRE".into();
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(p: &str) -> MonsterSpec {
        crate::v2::interpret(crate::parser::parse_vocabulary(p), p)
    }
    #[test]
    fn aliases_are_v2_only_and_species_preserving() {
        for (p, a) in [
            ("lobster", "LOBSTER"),
            ("grendel", "GRENDEL"),
            ("potato monster", "POTATO"),
            ("sphere", "ROUNDLING"),
            ("cacodaemon", "CACODAEMON"),
            ("floating head demon", "CACODAEMON"),
            ("reaper", "REAPER"),
            ("worm", "WORM"),
        ] {
            assert_eq!(parse(p).affinity, a, "{p}");
        }
        assert_ne!(parse("ballista").affinity, "ROUNDLING");
        for p in ["cute elephant", "chibi elephant"] {
            let s = parse(p);
            assert_eq!(s.affinity, "ELEPHANT");
            assert!(s.features.contains(&"CHIBI".into()));
            assert!(!s.features.contains(&"FRIENDLY_LURE".into()));
            assert_eq!(s.attack.delivery, "MELEE");
        }
        assert_eq!(parse("zombie wolf").affinity, "WOLF");
        assert!(!parse("rapper orc with shades")
            .features
            .contains(&"SHADE".into()));
        for p in ["water bear", "ninja wolf", "dancer elephant", "flaming orc"] {
            let s = parse(p);
            assert_ne!(s.affinity, "UNKNOWN");
        }
        assert_eq!(parse("hot potato monster").attack.element, "FIRE");
    }
    #[test]
    fn every_round_fire_and_dance_alias_is_token_aware() {
        for word in ["coconut", "sphere", "ball", "dot", "circle"] {
            let s = parse(word);
            assert_eq!(s.affinity, "ROUNDLING");
            assert_eq!(s.attack.delivery, "PROJECTILE");
        }
        for word in ["hot", "flaming", "fire", "fiery", "burning"] {
            let s = parse(&format!("{word} wolf"));
            assert_eq!(s.affinity, "WOLF");
            assert_eq!(s.attack.element, "FIRE");
        }
        for word in ["rapper", "dancer"] {
            assert!(parse(&format!("{word} wolf"))
                .features
                .contains(&"DANCER".into()));
        }
        for p in ["ballista", "firefly", "watermelon"] {
            let s = parse(p);
            assert_ne!(s.affinity, "ROUNDLING");
            assert!(!s.features.contains(&"SWIM".into()));
            assert!(!s.features.contains(&"FLAMING".into()));
        }
    }
    #[test]
    fn grendel_and_explicit_counts_survive_seeded_mutation() {
        for seed in 0..256 {
            let mut s = parse("grendel");
            crate::v2::prepare(&mut s);
            crate::v2::anatomy::prepare_seeded(&mut s, seed);
            assert_eq!(s.arm_count, 1);
        }
        let mut s = parse("zombie wolf with four legs");
        crate::v2::prepare(&mut s);
        crate::v2::anatomy::prepare_seeded(&mut s, 0);
        assert_eq!(s.limb_count, 4);
        let mut z = parse("zombie wolf");
        crate::v2::prepare(&mut z);
        crate::v2::anatomy::prepare_seeded(&mut z, 0);
        assert_eq!(z.limb_count, 3);
    }
    #[test]
    fn dancer_rarity_is_deterministic_and_near_one_in_a_thousand() {
        let mut base = parse("wolf");
        crate::v2::prepare(&mut base);
        let mut count = 0;
        for seed in 0..100_000 {
            let mut s = base.clone();
            crate::v2::anatomy::prepare_seeded(&mut s, seed);
            if s.features.contains(&"RARE_DANCER".into()) {
                count += 1;
                let mut replay = base.clone();
                crate::v2::anatomy::prepare_seeded(&mut replay, seed);
                assert_eq!(s.features, replay.features);
            }
        }
        assert!((65..=140).contains(&count), "observed {count}/100000");
    }
    #[cfg(feature = "bert")]
    #[test]
    fn bert_and_vocabulary_share_new_modifiers() {
        for (p, affinity) in [
            ("coconut", "ROUNDLING"),
            ("sphere", "ROUNDLING"),
            ("ball", "ROUNDLING"),
            ("dot", "ROUNDLING"),
            ("circle", "ROUNDLING"),
            ("lobster", "LOBSTER"),
            ("grendel", "GRENDEL"),
            ("potato monster", "POTATO"),
            ("cacodaemon", "CACODAEMON"),
            ("floating head demon", "CACODAEMON"),
            ("reaper", "REAPER"),
            ("worm", "WORM"),
        ] {
            let b = crate::parser::parse_with_embedded_bert(p).unwrap();
            assert_eq!(crate::v2::interpret(b, p).affinity, affinity, "{p}");
        }
        for (p, feature) in [
            ("cute elephant", "CHIBI"),
            ("chibi wolf", "CHIBI"),
            ("zombie wolf", "ZOMBIFIED"),
            ("ninja bear", "NINJA"),
            ("water bear", "SWIM"),
            ("flaming orc", "FLAMING"),
            ("rapper orc", "DANCER"),
        ] {
            let b = crate::parser::parse_with_embedded_bert(p).unwrap();
            let b = crate::v2::interpret(b, p);
            assert!(b.features.iter().any(|f| f == feature), "{p}");
        }
    }
}
