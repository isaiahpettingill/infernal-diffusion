//! Opt-in versioned recipe refinement. V1 never enters this module's stages.
pub mod anatomy;
pub mod attack_tracks;
pub mod combat;
pub mod grip;
pub mod hybrid;
pub mod prompt_normalization;
mod recipe_extensions;
pub mod spear_throw;
pub mod spectral;
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

/// V2 semantic extensions share the exact count intent from both parser paths.
pub fn interpret(mut spec: MonsterSpec, prompt: &str) -> MonsterSpec {
    if let Some(normalized) = prompt_normalization::normalize_mount(prompt) {
        return normalized;
    }
    // Keep every modifier on its owning clause in ordinary riding grammar as
    // well as mount-first requests. The legacy parser already supplies the tree.
    let lower = prompt.to_lowercase();
    if spec.mount.is_some() {
        if let Some((rider_clause, mount_clause)) = lower.split_once(" riding ") {
            let mut mount = spec.mount.take().unwrap();
            let lower = crate::parser::mount_spec(&spec, &mount);
            let lower = interpret(lower, mount_clause);
            mount.body_plan = lower.body_plan.clone();
            mount.affinity = lower.affinity.clone();
            mount.spec = Some(Box::new(lower));
            let mut rider = interpret(spec, rider_clause);
            rider.mount = Some(mount);
            return rider;
        }
    }
    let counts = crate::parser::ExplicitCounts::from_prompt(prompt);
    counts.apply_v2(&mut spec);
    let lower = prompt.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    let has = |word: &str| words.contains(&word);
    if has("human") && spec.affinity == "UNKNOWN" {
        spec.affinity = "HUMAN".into();
        spec.body_plan = "HUMANOID".into();
        spec.limb_count = 2;
        spec.arm_count = 2;
        spec.confidence = 1.0;
    }
    let explicit_legs = counts.legs.is_some();
    if has("lobster") {
        spec.affinity = "LOBSTER".into();
        spec.body_plan = "ARACHNID".into();
        if !explicit_legs {
            spec.limb_count = 8;
        }
        spec.arm_count = 0;
        spec.materials = vec!["CHITIN".into()];
        spec.confidence = 1.0;
    }
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
    if has("cute") || has("chibi") {
        spec.features.push("CHIBI".into());
    }
    if [
        "friendly",
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
    if let Some(recipe) = hybrid::parse_prompt(prompt) {
        hybrid::apply_spec(&recipe, &mut spec);
    }
    for (word, feature) in [("shade", "SHADE"), ("ghost", "GHOST")] {
        if (has(word) || has(&format!("{word}s")))
            && !(word == "shade" && !has("shade") && (has("rapper") || has("dancer")))
        {
            spec.features.push(feature.into());
            if spec.affinity == "UNKNOWN" {
                spec.affinity = feature.into();
                spec.body_plan = "HUMANOID".into();
                spec.limb_count = 2;
                spec.arm_count = 2;
                spec.confidence = 1.0;
            }
        }
    }
    if ["mutant", "mutated"].iter().any(|word| has(word)) {
        spec.features.push("MUTANT_UNSTABLE".into());
        if spec.arm_count == 4 {
            spec.features.push("FOUR_ARMED".into());
        }
    }
    if has("ghoul") || has("ghouls") {
        spec.affinity = "GHOUL".into();
        spec.body_plan = "HUMANOID".into();
        spec.limb_count = 2;
        spec.arm_count = 2;
        spec.confidence = 1.0;
    }
    let mut spec = recipe_extensions::interpret(spec, prompt);
    // Explicit user counts win over species, hybrid and modifier defaults.
    // Reapplying also protects intent if a future recipe changes a count.
    counts.apply_v2(&mut spec);
    spec
}

pub fn prepare(spec: &mut MonsterSpec) {
    if !spec.features.iter().any(|f| f == "PROFILE_V2") {
        spec.features.push("PROFILE_V2".into());
    }
    anatomy::prepare_spec(spec);
    if spec.features.iter().any(|feature| feature == "SHADE") {
        spec.spawn = None;
    }
}

fn describe_hybrid(spec: &MonsterSpec) -> Option<proto::HybridRecipeDescriptor> {
    let recipe = hybrid::from_spec(spec)?;
    // Both schemas use the same strict named fields; wire counts widen u8 to u32.
    serde_json::from_value(serde_json::to_value(recipe).ok()?).ok()
}

fn validate_hybrid(monster: &proto::Monster, profile: &proto::V2Profile) -> Result<(), String> {
    let marked = monster.tags.iter().any(|tag| tag.starts_with("HYBRID_"));
    let Some(descriptor) = &profile.hybrid else {
        return if marked {
            Err("hybrid tags require structured provenance".into())
        } else {
            Ok(())
        };
    };
    let recipe: hybrid::HybridRecipe =
        serde_json::from_value(serde_json::to_value(descriptor).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    recipe.validate()?;
    let mut spec = crate::parser::parse_vocabulary("humanoid");
    hybrid::apply_spec(&recipe, &mut spec);
    let expected: std::collections::BTreeSet<_> = spec
        .features
        .iter()
        .filter(|f| f.starts_with("HYBRID_"))
        .collect();
    let actual: std::collections::BTreeSet<_> = monster
        .tags
        .iter()
        .filter(|f| f.starts_with("HYBRID_"))
        .collect();
    if actual != expected
        || actual.len()
            != monster
                .tags
                .iter()
                .filter(|f| f.starts_with("HYBRID_"))
                .count()
    {
        return Err("hybrid provenance disagrees with explicit part tags".into());
    }
    Ok(())
}

fn descriptor_pattern(attack: &proto::Attack, is_projectile: bool) -> String {
    attack
        .id
        .strip_prefix("v2_pattern_")
        .unwrap_or({
            match attack.id.as_str() {
                "v2_passive_contact" => "passive_contact",
                "v2_spear_throw" => "spear_throw",
                "v2_mermaid_lunge" => "full_body_lunge",
                "v2_mermaid_tail_whip" => "tail_whip",
                "v2_tree_bilateral_smash" => "bilateral_extending_smash",
                "v2_spectral_phase_lunge" => "phase_reappear_lunge",
                "v2_shade_tendril_lash" => "shade_tendril_lash",
                "v2_shade_tendril_thrust" => "shade_tendril_thrust",
                _ if is_projectile => "aimed",
                _ => "anatomical_strike",
            }
        })
        .to_uppercase()
}

pub fn describe(
    spec: &MonsterSpec,
    attacks: &[proto::Attack],
    animations: &[proto::Animation],
) -> proto::V2Profile {
    let friendly = spec.features.iter().any(|f| f == "FRIENDLY_LURE")
        && !spec.features.iter().any(|f| f == "DANCER");
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
                motion_samples: vec![],
                pattern: descriptor_pattern(attack, !projectiles.is_empty()),
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
    let primary = if !spec
        .features
        .iter()
        .any(|feature| feature == "EXPLICIT_ATTACK")
    {
        attacks
            .iter()
            .find(|attack| attack.id == "v2_tree_bilateral_smash")
            .or_else(|| {
                attacks
                    .iter()
                    .find(|attack| attack.id == "v2_mermaid_lunge")
            })
            .or_else(|| attacks.first())
    } else {
        attacks.first()
    };
    proto::V2Profile {
        profile_version: 1,
        spectral: spectral::describe(spec, animations),
        hybrid: describe_hybrid(spec),
        body_motion_samples: vec![],
        categories: vec![
            spec.body_plan.clone(),
            if friendly {
                "PROXIMITY_AMBUSH"
            } else {
                "HOSTILE"
            }
            .into(),
        ],
        capabilities: {
            let mut capabilities = if friendly {
                vec!["LURE".into(), "REVEAL".into(), "ATTACK".into()]
            } else {
                vec!["ATTACK".into()]
            };
            if spectral::describe(spec, animations).is_some() {
                capabilities.push("PHASING".into());
            }
            capabilities
        },
        modifiers: {
            let mut modifiers = vec!["ORGANIC_VARIATION".into(), "SEEDED_PALETTE".into()];
            for feature in [
                "FOUR_ARMED",
                "EXTRA_TAIL",
                "MUTANT_UNSTABLE",
                "AI_ANATOMY_ECHO",
                "ZOMBIFIED",
                "NINJA",
                "DANCER",
                "RARE_DANCER",
                "FLAMING",
                "SWIM",
                "CHIBI",
            ] {
                if spec.features.iter().any(|f| f == feature) {
                    modifiers.push(feature.into());
                }
            }
            if spec.features.iter().any(|f| f == "FOUR_ARMED") {
                modifiers.push("FOLDED_EXTRA_ARMS_UNFOLD".into());
            }
            if spec.features.iter().any(|f| f == "MUTANT_UNSTABLE") {
                modifiers.push("AWKWARD_COORDINATION".into());
            }
            if attacks.iter().any(|a| a.id == "v2_tree_bilateral_smash") {
                modifiers.push("BOUNDED_ARM_EXTENSION_MAX_1_8".into());
            }
            if attacks
                .iter()
                .any(|a| a.id.starts_with("v2_shade_tendril_"))
            {
                modifiers.push("BOUNDED_SHADE_TENDRIL_EXTENSION_MAX_1_8".into());
                modifiers.push("WHOLE_BODY_UNWIND_REVERSE".into());
            }
            modifiers
        },
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
        let marked_v2 = monster.tags.iter().any(|tag| tag == "PROFILE_V2")
            || monster.generation.as_ref().is_some_and(|generation| {
                generation
                    .recipe_ids
                    .iter()
                    .any(|id| id.starts_with("profile.v2@"))
            })
            || monster.animations.iter().any(|clip| {
                clip.id.starts_with("v2_")
                    || clip
                        .events
                        .iter()
                        .any(|event| event.kind.starts_with("V2_"))
            });
        return if marked_v2 {
            Err(Error::InvalidMonster(
                "V2-marked package requires its profile descriptor".into(),
            ))
        } else {
            Ok(())
        };
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
    let unique_modifiers: std::collections::BTreeSet<_> =
        profile.modifiers.iter().map(String::as_str).collect();
    let has = |modifier: &str| unique_modifiers.contains(modifier);
    if unique_modifiers.len() != profile.modifiers.len()
        || !has("ORGANIC_VARIATION")
        || !has("SEEDED_PALETTE")
        || unique_modifiers.iter().any(|m| {
            ![
                "ORGANIC_VARIATION",
                "SEEDED_PALETTE",
                "FOUR_ARMED",
                "EXTRA_TAIL",
                "MUTANT_UNSTABLE",
                "AWKWARD_COORDINATION",
                "FOLDED_EXTRA_ARMS_UNFOLD",
                "BOUNDED_ARM_EXTENSION_MAX_1_8",
                "BOUNDED_SHADE_TENDRIL_EXTENSION_MAX_1_8",
                "WHOLE_BODY_UNWIND_REVERSE",
                "AI_ANATOMY_ECHO",
                "ZOMBIFIED",
                "CHIBI",
                "NINJA",
                "DANCER",
                "RARE_DANCER",
                "FLAMING",
                "SWIM",
            ]
            .contains(m)
        })
        || has("MUTANT_UNSTABLE") != has("AWKWARD_COORDINATION")
        || has("FOUR_ARMED") != has("FOLDED_EXTRA_ARMS_UNFOLD")
        || has("WHOLE_BODY_UNWIND_REVERSE") != has("BOUNDED_SHADE_TENDRIL_EXTENSION_MAX_1_8")
        || ((has("FOUR_ARMED") || has("EXTRA_TAIL")) && !has("MUTANT_UNSTABLE"))
        || has("BOUNDED_ARM_EXTENSION_MAX_1_8")
            != monster
                .attacks
                .iter()
                .any(|a| a.id == "v2_tree_bilateral_smash")
        || has("BOUNDED_SHADE_TENDRIL_EXTENSION_MAX_1_8")
            != monster
                .attacks
                .iter()
                .any(|a| a.id.starts_with("v2_shade_tendril_"))
        || profile.categories.len() != 2
        || profile.categories[1]
            != if profile.lure.is_some() {
                "PROXIMITY_AMBUSH"
            } else {
                "HOSTILE"
            }
        || ![
            "ARACHNID",
            "INSECT",
            "QUADRUPED",
            "THEROPOD",
            "SERPENT",
            "GASTROPOD",
            "WINGED",
            "FLOATING",
            "AMORPHOUS",
            "HUMANOID",
            "FUNGUS",
            "TANK",
        ]
        .contains(&profile.categories[0].as_str())
    {
        return Err(invalid());
    }
    for (modifier, event_kind) in [
        (
            "AWKWARD_COORDINATION",
            "V2_AWKWARD_COORDINATION_MAX_0_18_RAD",
        ),
        ("FOLDED_EXTRA_ARMS_UNFOLD", "V2_FOLDED_EXTRA_ARMS_UNFOLD"),
    ] {
        for attack in &monster.attacks {
            let Some(clip) = monster
                .animations
                .iter()
                .find(|c| c.id == attack.animation_id)
            else {
                return Err(invalid());
            };
            let event_count = clip
                .events
                .iter()
                .filter(|e| e.kind == event_kind && e.reference_id == attack.id)
                .count();
            if event_count != usize::from(has(modifier) && attack.id != "v2_passive_contact") {
                return Err(invalid());
            }
        }
    }
    if has("DANCER")
        && (monster.attacks.len() != 1
            || monster.attacks[0].id != "v2_passive_contact"
            || monster.attacks[0].delivery != "CONTACT"
            || !monster.projectiles.is_empty()
            || monster.animations.iter().flat_map(|a| &a.events).any(|e| {
                [
                    "ATTACK",
                    "ATTACK_HIT",
                    "PROJECTILE_SPAWN",
                    "FIELD_SPAWN",
                    "MINION_SPAWN",
                ]
                .contains(&e.kind.as_str())
            }))
    {
        return Err(invalid());
    }
    if has("RARE_DANCER") && !has("DANCER") {
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
        let expected = descriptor_pattern(attack, !shots.is_empty());
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
    spectral::validate(monster).map_err(|_| invalid())?;
    validate_hybrid(monster, profile).map_err(|_| invalid())?;
    attack_tracks::validate(monster).map_err(|_| invalid())?;
    // Same recipe/event checks apply to generated and untrusted imported packages.
    combat::validate_patterns(&monster.animations, &monster.attacks).map_err(|_| invalid())?;
    let mut required = if profile.lure.is_some() {
        vec!["LURE", "REVEAL", "ATTACK"]
    } else {
        vec!["ATTACK"]
    };
    if profile.spectral.is_some() {
        required.push("PHASING");
    }
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
            || lure_clip.events.len() != 1
            || reveal.events.len() != 2
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
                    e.reference_id != clip.id
                        || match (clip.semantic_state.as_str(), e.kind.as_str()) {
                            ("LURE", "LURE_HEARTS") => e.time_ms != 240,
                            ("REVEAL", "REVEAL") => e.time_ms != 0,
                            ("REVEAL", "REVEAL_COMPLETE") => e.time_ms != lure.telegraph_ms,
                            _ => true,
                        }
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
        let profile = describe(&spec, &attacks, &animations);
        proto::Monster {
            animations,
            attacks,
            v2_profile: Some(profile),
            ..Default::default()
        }
    }
    #[test]
    fn ambush_uses_species_strike_unless_attack_was_explicit() {
        for (prompt, expected) in [
            ("friendly tree", "v2_tree_bilateral_smash"),
            ("friendly mermaid", "v2_mermaid_lunge"),
            ("friendly mermaid that sprays fire", "v2_pattern_fire_spray"),
        ] {
            let mut spec = interpret(crate::parser::parse_vocabulary(prompt), prompt);
            prepare(&mut spec);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
            anatomy::refine(&spec, &mut body, 0);
            let (animations, _, attacks, _) = combat::animate(&spec, &body, 0);
            assert_eq!(
                describe(&spec, &attacks, &animations)
                    .lure
                    .unwrap()
                    .attack_id,
                expected
            );
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
