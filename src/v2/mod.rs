//! Opt-in versioned recipe refinement. V1 never enters this module's stages.
pub mod anatomy;
pub mod combat;
use crate::{parser::MonsterSpec, proto, Error};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u32)]
pub enum GenerationProfile {
    #[default]
    V1 = 1,
    V2 = 2,
}
impl TryFrom<u32> for GenerationProfile {
    type Error = Error;
    fn try_from(value: u32) -> Result<Self, Error> {
        match value {
            1 => Ok(Self::V1),
            2 => Ok(Self::V2),
            _ => Err(Error::InvalidMonster(format!(
                "unknown generation profile {value}"
            ))),
        }
    }
}
impl std::str::FromStr for GenerationProfile {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Error> {
        match value {
            "v1" => Ok(Self::V1),
            "v2" => Ok(Self::V2),
            _ => Err(Error::InvalidMonster(format!(
                "unknown generation profile {value}; expected v1 or v2"
            ))),
        }
    }
}

/// V2-only vocabulary extensions leave both legacy parsers untouched.
pub fn interpret(mut spec: MonsterSpec, prompt: &str) -> MonsterSpec {
    let lower = prompt.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let has = |word: &str| words.contains(&word);
    let explicit_legs = words
        .windows(2)
        .any(|pair| ["legs", "limbs"].contains(&pair[1]));
    if has("frog") || has("toad") {
        spec.affinity = "FROG".into();
        spec.body_plan = "QUADRUPED".into();
        if !explicit_legs {
            spec.limb_count = 4;
        }
        if !has("arms") {
            spec.arm_count = 0;
        }
        if spec.confidence == 0.0 {
            spec.confidence = 1.0;
        }
        if spec.materials == ["FLESH"] {
            spec.materials = vec!["LEAF".into()];
        }
    }
    if has("tree") && spec.affinity == "UNKNOWN" {
        spec.affinity = "TREE_MONSTER".into();
        spec.body_plan = "FUNGUS".into();
        spec.materials = vec!["WOOD".into()];
    }
    if [
        "friendly",
        "cute",
        "adorable",
        "welcoming",
        "smiling",
        "lure",
        "fake",
    ]
    .iter()
    .any(|w| has(w))
    {
        spec.features.push("FRIENDLY_LURE".into());
    }
    if words.windows(2).any(|pair| {
        ["legs", "limbs"].contains(&pair[1])
            && (pair[0].parse::<u8>().is_ok()
                || [
                    "zero", "one", "two", "three", "four", "five", "six", "eight", "ten", "twelve",
                    "no",
                ]
                .contains(&pair[0]))
    }) {
        spec.features.push("EXPLICIT_LEGS".into());
    }
    if [
        "shoots",
        "sprays",
        "spits",
        "acid",
        "fire",
        "ice",
        "poison",
        "lightning",
    ]
    .iter()
    .any(|w| has(w))
    {
        spec.features.push("EXPLICIT_ATTACK".into());
    }
    if [
        "sprays", "spray", "spits", "spit", "shoots", "shoot", "breathes",
    ]
    .iter()
    .any(|w| has(w))
    {
        spec.attack.delivery = "PROJECTILE".into();
        if ["sprays", "spray", "spits", "spit"].iter().any(|w| has(w)) {
            spec.attack.concept = "SPRAY".into();
        }
    }
    spec
}

pub fn prepare(spec: &mut MonsterSpec) {
    if !spec.features.iter().any(|f| f == "PROFILE_V2") {
        spec.features.push("PROFILE_V2".into());
    }
    anatomy::prepare_spec(spec);
}

pub fn describe(spec: &MonsterSpec, attacks: &[proto::Attack]) -> proto::V2Profile {
    let friendly = spec.features.iter().any(|f| f == "FRIENDLY_LURE");
    let patterns = attacks
        .iter()
        .map(|attack| {
            let projectiles: Vec<_> = attack
                .steps
                .iter()
                .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                .collect();
            proto::AttackPatternDescriptor {
                attack_id: attack.id.clone(),
                pattern: attack
                    .id
                    .strip_prefix("v2_pattern_")
                    .unwrap_or(if projectiles.is_empty() {
                        "anatomical_strike"
                    } else {
                        "aimed"
                    })
                    .to_uppercase(),
                projectile_count: projectiles.iter().map(|s| s.count).sum(),
                volley_count: projectiles.len() as u32,
                interval_ms: if projectiles.len() > 1 {
                    projectiles[1].at_ms.saturating_sub(projectiles[0].at_ms)
                } else {
                    0
                },
                spread_deg: projectiles.iter().map(|s| s.spread_deg).fold(0.0, f32::max),
                aim_mode: projectiles
                    .first()
                    .map_or("ANATOMY_ORIGIN", |s| s.target.as_str())
                    .into(),
            }
        })
        .collect();
    let primary = attacks.first();
    proto::V2Profile {
        profile_version: 1,
        categories: vec![
            spec.body_plan.clone(),
            if friendly {
                "PROXIMITY_AMBUSH"
            } else {
                "HOSTILE"
            }
            .into(),
        ],
        capabilities: if friendly {
            vec!["LURE".into(), "REVEAL".into(), "ATTACK".into()]
        } else {
            vec!["ATTACK".into()]
        },
        modifiers: vec!["ORGANIC_VARIATION".into(), "SEEDED_PALETTE".into()],
        attack_patterns: patterns,
        lure: if friendly {
            Some(proto::LureDescriptor {
                lure_animation_id: combat::LURE_CLIP.into(),
                reveal_animation_id: combat::REVEAL_CLIP.into(),
                trigger: "TARGET_WITHIN_RADIUS".into(),
                trigger_radius: combat::LURE_RADIUS,
                attack_id: primary.map_or_else(String::new, |a| a.id.clone()),
                effect: "HEARTS".into(),
                harmless: true,
                telegraph_ms: combat::REVEAL_DURATION_MS,
                distant_state: "LURE".into(),
                revealed_state: "ATTACK".into(),
                reset_policy: "RETURN_TO_LURE".into(),
            })
        } else {
            None
        },
    }
}

pub fn validate_profile(monster: &proto::Monster) -> Result<(), Error> {
    let Some(profile) = &monster.v2_profile else {
        return Ok(());
    };
    let invalid =
        || Error::InvalidMonster("invalid V2 profile or dangling capability references".into());
    if profile.profile_version != 1
        || profile.attack_patterns.len() != monster.attacks.len()
        || profile.attack_patterns.is_empty()
        || profile.attack_patterns.len() > 32
    {
        return Err(invalid());
    }
    let mut seen = std::collections::BTreeSet::new();
    for pattern in &profile.attack_patterns {
        if !seen.insert(pattern.attack_id.as_str()) {
            return Err(invalid());
        }
        let Some(attack) = monster.attacks.iter().find(|a| a.id == pattern.attack_id) else {
            return Err(invalid());
        };
        if attack.steps.len() > 32 {
            return Err(invalid());
        }
        let shots: Vec<_> = attack
            .steps
            .iter()
            .filter(|s| s.primitive == "SPAWN_PROJECTILE")
            .collect();
        let count = shots
            .iter()
            .try_fold(0u32, |n, s| n.checked_add(s.count))
            .ok_or_else(invalid)?;
        if shots
            .iter()
            .any(|s| s.count == 0 || s.count > 16 || !s.spread_deg.is_finite())
        {
            return Err(invalid());
        }
        let interval = if shots.len() > 1 {
            shots[1]
                .at_ms
                .checked_sub(shots[0].at_ms)
                .ok_or_else(invalid)?
        } else {
            0
        };
        let spread = shots.iter().map(|s| s.spread_deg).fold(0.0, f32::max);
        let expected = attack
            .id
            .strip_prefix("v2_pattern_")
            .unwrap_or(if shots.is_empty() {
                "anatomical_strike"
            } else {
                "aimed"
            })
            .to_uppercase();
        if pattern.pattern != expected
            || pattern.volley_count != shots.len() as u32
            || pattern.interval_ms != interval
            || pattern.spread_deg != spread
            || pattern.aim_mode
                != shots
                    .first()
                    .map_or("ANATOMY_ORIGIN", |s| s.target.as_str())
            || count != pattern.projectile_count
            || !pattern.spread_deg.is_finite()
            || pattern.spread_deg < 0.0
            || pattern.spread_deg > 120.0
        {
            return Err(invalid());
        }
    }
    // Same recipe/event checks apply to generated and untrusted imported packages.
    combat::validate_patterns(&monster.animations, &monster.attacks).map_err(|_| invalid())?;
    let required: &[&str] = if profile.lure.is_some() {
        &["LURE", "REVEAL", "ATTACK"]
    } else {
        &["ATTACK"]
    };
    if profile.capabilities.len() != required.len()
        || required.iter().any(|c| {
            profile
                .capabilities
                .iter()
                .filter(|v| v.as_str() == *c)
                .count()
                != 1
        })
    {
        return Err(invalid());
    }
    if let Some(lure) = &profile.lure {
        let lure_clip = monster
            .animations
            .iter()
            .find(|a| a.id == lure.lure_animation_id)
            .ok_or_else(invalid)?;
        let reveal = monster
            .animations
            .iter()
            .find(|a| a.id == lure.reveal_animation_id)
            .ok_or_else(invalid)?;
        let reveal_duration = reveal
            .frames
            .iter()
            .try_fold(0u32, |n, f| n.checked_add(f.duration_ms))
            .ok_or_else(invalid)?;
        if !lure.harmless
            || lure.trigger != "TARGET_WITHIN_RADIUS"
            || !lure.trigger_radius.is_finite()
            || !(1.0..=1000.0).contains(&lure.trigger_radius)
            || lure.effect != "HEARTS"
            || lure.distant_state != "LURE"
            || lure.revealed_state != "ATTACK"
            || lure.reset_policy != "RETURN_TO_LURE"
            || !(120..=5000).contains(&lure.telegraph_ms)
            || reveal_duration != lure.telegraph_ms
            || !monster.attacks.iter().any(|a| a.id == lure.attack_id)
            || lure_clip.id == reveal.id
            || !lure_clip.looped
            || reveal.looped
            || lure_clip.semantic_state != "LURE"
            || reveal.semantic_state != "REVEAL"
            || !lure_clip
                .events
                .iter()
                .any(|e| e.kind == "LURE_HEARTS" && e.reference_id == lure_clip.id)
            || !reveal
                .events
                .iter()
                .any(|e| e.kind == "REVEAL" && e.time_ms == 0 && e.reference_id == reveal.id)
            || !reveal.events.iter().any(|e| {
                e.kind == "REVEAL_COMPLETE"
                    && e.time_ms == lure.telegraph_ms
                    && e.reference_id == reveal.id
            })
            || monster
                .attacks
                .iter()
                .any(|a| a.animation_id == lure_clip.id || a.animation_id == reveal.id)
            || [lure_clip, reveal].iter().any(|clip| {
                clip.events.iter().any(|e| {
                    !matches!(
                        e.kind.as_str(),
                        "LURE_HEARTS" | "REVEAL" | "REVEAL_COMPLETE"
                    )
                })
            })
        {
            return Err(invalid());
        }
    }
    Ok(())
}

#[cfg(test)]
mod profile_validation_tests {
    use super::*;
    use rand::SeedableRng;
    fn fixture() -> proto::Monster {
        let mut spec = interpret(
            crate::parser::parse_vocabulary("friendly goblin"),
            "friendly goblin",
        );
        prepare(&mut spec);
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
        let (animations, _, attacks, _) = combat::animate(&spec, &body, 0);
        let profile = describe(&spec, &attacks);
        proto::Monster {
            animations,
            attacks,
            v2_profile: Some(profile),
            ..Default::default()
        }
    }
    #[test]
    fn rejects_duplicate_or_fabricated_pattern_metadata() {
        let valid = fixture();
        validate_profile(&valid).unwrap();
        let mut bad = valid.clone();
        let profile = bad.v2_profile.as_mut().unwrap();
        assert!(profile.attack_patterns.len() > 1);
        profile.attack_patterns[1] = profile.attack_patterns[0].clone();
        assert!(validate_profile(&bad).is_err());
        let mut bad = valid.clone();
        bad.attacks[0].id = "v2_pattern_invented".into();
        let p = &mut bad.v2_profile.as_mut().unwrap().attack_patterns[0];
        p.attack_id = "v2_pattern_invented".into();
        p.pattern = "INVENTED".into();
        assert!(validate_profile(&bad).is_err());
        let mut bad = valid;
        bad.attacks[0]
            .steps
            .iter_mut()
            .filter(|s| s.primitive == "SPAWN_PROJECTILE")
            .for_each(|s| s.count = u32::MAX);
        assert!(validate_profile(&bad).is_err());
    }
    #[test]
    fn rejects_misleading_lure_states_timing_and_damage() {
        let valid = fixture();
        for change in 0..5 {
            let mut bad = valid.clone();
            let lure = bad.v2_profile.as_mut().unwrap().lure.as_mut().unwrap();
            match change {
                0 => lure.telegraph_ms += 1,
                1 => lure.reset_policy = "PERSISTENT_MUTATION".into(),
                2 => lure.revealed_state = "IDLE".into(),
                3 => bad
                    .animations
                    .iter_mut()
                    .find(|a| a.id == lure.reveal_animation_id)
                    .unwrap()
                    .events
                    .clear(),
                _ => bad.attacks[0].animation_id = lure.reveal_animation_id.clone(),
            }
            assert!(validate_profile(&bad).is_err());
        }
    }
}
