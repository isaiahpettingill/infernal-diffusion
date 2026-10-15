//! Declarative spectral interaction rules. Engines apply these independently of
//! rendered alpha. Higher-priority matching rules win; visual opacity is never
//! treated as immunity or a collision response.
use crate::{parser::MonsterSpec, proto::*};

pub fn validate_spec(spec: &MonsterSpec) -> Result<(), String> {
    if spec.features.iter().any(|f| f == "SHADE") && spec.features.iter().any(|f| f == "GHOST") {
        return Err(
            "SHADE and GHOST have incompatible defenses; select one spectral modifier".into(),
        );
    }
    Ok(())
}

fn kind(spec: &MonsterSpec) -> Option<&'static str> {
    if spec.features.iter().any(|f| f == "SHADE") {
        Some("SHADE")
    } else if spec.features.iter().any(|f| f == "GHOST") {
        Some("GHOST")
    } else if spec.affinity == "GHOUL" {
        Some("GHOUL")
    } else {
        None
    }
}

fn base(kind: &str) -> SpectralDescriptor {
    let rule = |priority,
                elements: Vec<String>,
                delivery: &str,
                required_tag: &str,
                damage_multiplier,
                projectile_pass_through| DamageRule {
        priority,
        elements,
        delivery: delivery.into(),
        required_tag: required_tag.into(),
        damage_multiplier,
        projectile_pass_through,
    };
    match kind {
        "SHADE" => SpectralDescriptor {
            kind: kind.into(),
            visual_style: "INK_BLACK".into(),
            opacity: 1.0,
            collision_mode: "PHASE_WINDOWS".into(),
            damage_rules: vec![
                rule(100, vec!["FIRE".into(), "WATER".into()], "", "", 1.0, false),
                rule(10, vec![], "PROJECTILE", "", 0.0, true),
            ],
            default_damage_multiplier: 1.0,
            default_projectile_pass_through: false,
            phase_windows: vec![],
        },
        "GHOST" => SpectralDescriptor {
            kind: kind.into(),
            visual_style: "TRANSLUCENT".into(),
            opacity: 0.38,
            collision_mode: "ALWAYS_INTANGIBLE".into(),
            damage_rules: vec![
                rule(100, vec![], "", "MAGIC", 1.0, false),
                rule(90, vec!["MAGIC".into()], "", "", 1.0, false),
            ],
            default_damage_multiplier: 0.0,
            default_projectile_pass_through: true,
            phase_windows: vec![],
        },
        _ => SpectralDescriptor {
            kind: "GHOUL".into(),
            visual_style: "DECAYED".into(),
            opacity: 1.0,
            collision_mode: "PHASE_WINDOWS".into(),
            damage_rules: vec![],
            default_damage_multiplier: 1.0,
            default_projectile_pass_through: false,
            phase_windows: vec![],
        },
    }
}

fn windows(animations: &[Animation]) -> Vec<PhaseWindow> {
    animations
        .iter()
        .filter_map(|clip| {
            let start = clip.events.iter().find(|e| e.kind == "PHASE_BEGIN")?;
            let end = clip.events.iter().find(|e| e.kind == "PHASE_END")?;
            Some(PhaseWindow {
                animation_id: clip.id.clone(),
                start_ms: start.time_ms,
                end_ms: end.time_ms,
                solid_pass_through: true,
            })
        })
        .collect()
}

pub fn describe(spec: &MonsterSpec, animations: &[Animation]) -> Option<SpectralDescriptor> {
    let mut profile = base(kind(spec)?);
    profile.phase_windows = windows(animations);
    Some(profile)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageResponse {
    pub damage_multiplier: f32,
    pub projectile_pass_through: bool,
}

/// Reference implementation for consumers: every nonempty rule criterion must
/// match. A FIRE/WATER projectile hits a shade before its delivery immunity can
/// match. A magical attack hits a ghost regardless of its elemental appearance.
pub fn damage_response(
    profile: &SpectralDescriptor,
    element: &str,
    delivery: &str,
    tags: &[&str],
) -> DamageResponse {
    let matching = profile
        .damage_rules
        .iter()
        .filter(|rule| {
            (rule.elements.is_empty() || rule.elements.iter().any(|e| e == element))
                && (rule.delivery.is_empty() || rule.delivery == delivery)
                && (rule.required_tag.is_empty() || tags.contains(&rule.required_tag.as_str()))
        })
        .max_by_key(|rule| rule.priority);
    matching.map_or(
        DamageResponse {
            damage_multiplier: profile.default_damage_multiplier,
            projectile_pass_through: profile.default_projectile_pass_through
                && delivery == "PROJECTILE",
        },
        |rule| DamageResponse {
            damage_multiplier: rule.damage_multiplier,
            projectile_pass_through: rule.projectile_pass_through && delivery == "PROJECTILE",
        },
    )
}

/// Collision capability only, never damage immunity. Phase windows are half-open
/// [start_ms, end_ms) in the current clip's timeline.
pub fn passes_solids(profile: &SpectralDescriptor, animation_id: &str, time_ms: u32) -> bool {
    profile.collision_mode == "ALWAYS_INTANGIBLE"
        || profile.phase_windows.iter().any(|window| {
            window.solid_pass_through
                && window.animation_id == animation_id
                && window.start_ms <= time_ms
                && time_ms < window.end_ms
        })
}

pub fn validate(monster: &Monster) -> Result<(), String> {
    let Some(profile) = monster.v2_profile.as_ref() else {
        return Ok(());
    };
    let marked = if monster.tags.iter().any(|t| t == "SHADE") {
        Some("SHADE")
    } else if monster.tags.iter().any(|t| t == "GHOST") {
        Some("GHOST")
    } else if monster.tags.iter().any(|t| t == "GHOUL") {
        Some("GHOUL")
    } else {
        None
    };
    if monster.tags.iter().any(|t| t == "SHADE") && monster.tags.iter().any(|t| t == "GHOST") {
        return Err("conflicting spectral modifiers".into());
    }
    let phase_marked = monster.animations.iter().any(|clip| {
        clip.events
            .iter()
            .any(|event| matches!(event.kind.as_str(), "PHASE_BEGIN" | "PHASE_END"))
    });
    let Some(spectral) = &profile.spectral else {
        return if marked.is_some() || phase_marked {
            Err("spectral modifier requires interaction rules".into())
        } else {
            Ok(())
        };
    };
    if marked != Some(spectral.kind.as_str())
        || !["SHADE", "GHOST", "GHOUL"].contains(&spectral.kind.as_str())
    {
        return Err("spectral identity disagrees with creature tags".into());
    }
    for clip in &monster.animations {
        let starts: Vec<_> = clip
            .events
            .iter()
            .filter(|event| event.kind == "PHASE_BEGIN")
            .collect();
        let ends: Vec<_> = clip
            .events
            .iter()
            .filter(|event| event.kind == "PHASE_END")
            .collect();
        if starts.is_empty() && ends.is_empty() {
            continue;
        }
        if starts.len() != 1 || ends.len() != 1 {
            return Err("phase requires one begin/end pair".into());
        }
        let (start, end) = (starts[0], ends[0]);
        let duration = clip
            .frames
            .iter()
            .try_fold(0u32, |sum, frame| sum.checked_add(frame.duration_ms))
            .ok_or("phase clip duration overflow")?;
        if start.reference_id != end.reference_id
            || !monster
                .attacks
                .iter()
                .any(|attack| attack.id == start.reference_id && attack.animation_id == clip.id)
            || start.time_ms >= end.time_ms
            || end.time_ms >= duration
        {
            return Err(
                "phase pair must reference this clip's attack and stay within its duration".into(),
            );
        }
    }
    let dancing = monster.tags.iter().any(|t| t == "DANCER")
        && monster.attacks.len() == 1
        && monster.attacks[0].id == "v2_passive_contact"
        && monster.attacks[0].delivery == "CONTACT";
    if spectral.kind == "SHADE"
        && !dancing
        && (monster.attacks.len() != 2
            || monster.attacks.iter().any(|attack| {
                !["v2_shade_tendril_lash", "v2_shade_tendril_thrust"].contains(&attack.id.as_str())
            }))
    {
        return Err("shades attack only through their bounded deforming tendrils".into());
    }
    let mut expected = base(&spectral.kind);
    expected.phase_windows = windows(&monster.animations);
    // A passive dancer keeps spectral opacity and damage interactions; without
    // a combat lunge it has no transient phase window. Ghosts remain intangible.
    if spectral.kind != "GHOST" && expected.phase_windows.is_empty() && !dancing {
        return Err("bounded spectral phasing requires an actual phase interval".into());
    }
    if *spectral != expected {
        return Err("invalid spectral damage precedence, opacity, or phase windows".into());
    }
    for window in &spectral.phase_windows {
        let clip = monster
            .animations
            .iter()
            .find(|c| c.id == window.animation_id)
            .ok_or("missing phase clip")?;
        let duration = clip
            .frames
            .iter()
            .try_fold(0u32, |sum, frame| sum.checked_add(frame.duration_ms))
            .ok_or("phase clip duration overflow")?;
        if window.start_ms >= window.end_ms
            || window.end_ms >= duration
            || clip
                .events
                .iter()
                .filter(|e| e.kind == "PHASE_BEGIN")
                .count()
                != 1
            || clip.events.iter().filter(|e| e.kind == "PHASE_END").count() != 1
        {
            return Err("invalid bounded phase interval".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phase_collision_uses_half_open_intervals_without_damage_immunity() {
        let mut ghoul = base("GHOUL");
        ghoul.phase_windows.push(PhaseWindow {
            animation_id: "phase".into(),
            start_ms: 150,
            end_ms: 550,
            solid_pass_through: true,
        });
        assert!(!passes_solids(&ghoul, "phase", 149));
        assert!(passes_solids(&ghoul, "phase", 150));
        assert!(passes_solids(&ghoul, "phase", 549));
        assert!(!passes_solids(&ghoul, "phase", 550));
        assert!(!passes_solids(&ghoul, "other", 300));
        assert_eq!(
            damage_response(&ghoul, "FIRE", "PROJECTILE", &[]).damage_multiplier,
            1.0
        );
        assert!(passes_solids(&base("GHOST"), "idle", 0));
    }
    #[test]
    fn shade_element_exception_wins_over_projectile_immunity() {
        let shade = base("SHADE");
        for element in ["FIRE", "WATER"] {
            assert_eq!(
                damage_response(&shade, element, "PROJECTILE", &[]),
                DamageResponse {
                    damage_multiplier: 1.0,
                    projectile_pass_through: false
                }
            );
        }
        assert_eq!(
            damage_response(&shade, "PHYSICAL", "PROJECTILE", &[]),
            DamageResponse {
                damage_multiplier: 0.0,
                projectile_pass_through: true
            }
        );
        assert_eq!(
            damage_response(&shade, "PHYSICAL", "MELEE", &[]).damage_multiplier,
            1.0
        );
    }
    #[test]
    fn ghost_requires_magic_and_ghoul_is_not_automatically_immune() {
        let ghost = base("GHOST");
        assert_eq!(
            damage_response(&ghost, "FIRE", "PROJECTILE", &[]).damage_multiplier,
            0.0
        );
        assert_eq!(
            damage_response(&ghost, "FIRE", "PROJECTILE", &["MAGIC"]).damage_multiplier,
            1.0
        );
        assert_eq!(
            damage_response(&ghost, "MAGIC", "MELEE", &[]).damage_multiplier,
            1.0
        );
        assert_eq!(
            damage_response(&base("GHOUL"), "PHYSICAL", "PROJECTILE", &[]).damage_multiplier,
            1.0
        );
    }
}
