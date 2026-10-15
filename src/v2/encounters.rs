//! Typed, bounded encounter capabilities. This module describes assets and limits;
//! targeting, entity identity, persistence, rewards and physics belong to the host.
use crate::{parser::MonsterSpec, proto::*};

fn has(spec: &MonsterSpec, name: &str) -> bool {
    spec.features.iter().any(|f| f == name)
}

pub fn interpret(mut spec: MonsterSpec, prompt: &str) -> MonsterSpec {
    let lower = prompt.to_lowercase();
    let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
    let word = |w: &str| words.contains(&w);
    let slime = word("slime")
        && ["splitting", "splits", "split", "splintering"]
            .iter()
            .any(|w| word(w));
    let mimic = word("mimic") || word("amphora") || word("amorpha");
    let sentinel = word("talos")
        || word("hoplite")
        || (word("shield") && (word("sentinel") || word("bronze")));
    let pusher = word("sisyphus")
        || ((word("boulder") || word("rock"))
            && ["pusher", "pushes", "pushing"].iter().any(|w| word(w)));
    let feature = if mimic {
        "OBJECT_MIMIC"
    } else if slime {
        "SPLITTING_SLIME"
    } else if sentinel {
        "SHIELD_SENTINEL"
    } else if pusher {
        "BOULDER_PUSHER"
    } else {
        return spec;
    };
    // Identity recipes use stable anatomy, not incidental parser hybrids/spawns.
    spec.features.retain(|f| {
        !["DANCER", "RARE_DANCER", "FRIENDLY_LURE", "EXPLICIT_ATTACK"].contains(&f.as_str())
    });
    spec.features.push(feature.into());
    spec.secondary_affinities.clear();
    spec.spawn = None;
    spec.confidence = 1.0;
    spec.heads = 1;
    spec.attack.delivery = "MELEE".into();
    spec.attack.concept = "BASH".into();
    spec.attack.element = "PHYSICAL".into();
    match feature {
        "SPLITTING_SLIME" => {
            spec.affinity = "SLIME".into();
            spec.body_plan = "AMORPHOUS".into();
            spec.arm_count = 0;
            spec.limb_count = 0;
            spec.features.push("HOP".into());
        }
        "OBJECT_MIMIC" => {
            spec.affinity = feature.into();
            spec.body_plan = "AMORPHOUS".into();
            spec.arm_count = 0;
            spec.limb_count = 0;
            spec.features.push("FRIENDLY_LURE".into());
            spec.features.push(
                if word("banana") {
                    "MIMIC_BANANA"
                } else if word("bush") || word("shrub") {
                    "MIMIC_BUSH"
                } else {
                    "MIMIC_VASE"
                }
                .into(),
            );
            spec.attack.concept = "BITE".into();
        }
        _ => {
            spec.affinity = feature.into();
            spec.body_plan = "HUMANOID".into();
            spec.arm_count = 2;
            spec.limb_count = 2;
            if sentinel {
                spec.materials = vec!["METAL".into()];
            }
        }
    }
    spec
}

/// First-batch encounter rigs have fixed anatomy. Reject unsupported requested
/// topology rather than silently depicting fewer heads/limbs than metadata says.
pub fn validate_spec(spec: &MonsterSpec) -> Result<(), String> {
    let Some(d) = describe(spec) else {
        return Ok(());
    };
    let limbs = if d.split.is_some() || d.mimic.is_some() {
        0
    } else {
        2
    };
    if spec.heads != 1
        || spec.arm_count != limbs
        || spec.limb_count != limbs
        || spec.mount.is_some()
    {
        return Err(format!("{} currently supports one head, {limbs} arms, {limbs} legs and no mount; alternate encounter anatomy is not supported", d.archetype));
    }
    Ok(())
}

pub fn describe(spec: &MonsterSpec) -> Option<EncounterDescriptor> {
    let archetype = [
        "SPLITTING_SLIME",
        "OBJECT_MIMIC",
        "SHIELD_SENTINEL",
        "BOULDER_PUSHER",
    ]
    .into_iter()
    .find(|name| has(spec, name))?;
    let mut d = EncounterDescriptor {
        contract_version: 1,
        archetype: archetype.into(),
        ..Default::default()
    };
    match archetype {
        "SPLITTING_SLIME" => {
            d.split = Some(SplitDescriptor {
                count: 2,
                max_generations: 2,
                max_offspring: 6,
                child_scale: 0.6,
                child_health_fraction: 0.3,
                child_speed_multiplier: 1.3,
                emergence_ms: 720,
                separation_px: 28.0,
                emergence_animation_id: "v2_emerge".into(),
                burst_animation_id: "v2_split_burst".into(),
            })
        }
        "OBJECT_MIMIC" => {
            d.mimic = Some(MimicDescriptor {
                disguise: if has(spec, "MIMIC_BANANA") {
                    "LARGE_BANANA"
                } else if has(spec, "MIMIC_BUSH") {
                    "BUSH"
                } else {
                    "VASE"
                }
                .into(),
                trigger_radius: 110.0,
                reveal_ms: 720,
            })
        }
        "SHIELD_SENTINEL" => {
            d.shield = Some(ShieldDescriptor {
                front_arc_degrees: 120.0,
                bash_windup_ms: 400,
                bash_recovery_ms: 720,
                guard_animation_id: "v2_guard".into(),
                bash_animation_id: "v2_shield_bash".into(),
                recovery_animation_id: "v2_shield_recovery".into(),
            })
        }
        "BOULDER_PUSHER" => {
            d.pushed_prop = Some(PushedPropDescriptor {
                kind: "BOULDER".into(),
                id_suffix: "boulder".into(),
                radius: 24.0,
                health: 45.0,
                push_speed: 150.0,
                coordinate_space: "LOCAL_PIXELS_X_FORWARD_Y_DOWN".into(),
                anchor_x: 42.0,
                anchor_y: -24.0,
                push_animation_id: "v2_push".into(),
                recovery_animation_id: "v2_push_recovery".into(),
                release_event: "PUSH_RELEASE".into(),
            })
        }
        _ => unreachable!(),
    }
    Some(d)
}

pub fn validate(monster: &Monster) -> Result<(), String> {
    let Some(profile) = &monster.v2_profile else {
        return Ok(());
    };
    let marked = [
        "SPLITTING_SLIME",
        "OBJECT_MIMIC",
        "SHIELD_SENTINEL",
        "BOULDER_PUSHER",
    ]
    .into_iter()
    .any(|s| monster.tags.iter().any(|tag| tag == s));
    let Some(d) = &profile.encounter else {
        return if marked {
            Err("encounter tags require typed metadata".into())
        } else {
            Ok(())
        };
    };
    let fail = || "invalid bounded encounter descriptor or clip reference".to_owned();
    if d.contract_version != 1
        || usize::from(d.split.is_some())
            + usize::from(d.mimic.is_some())
            + usize::from(d.shield.is_some())
            + usize::from(d.pushed_prop.is_some())
            != 1
    {
        return Err(fail());
    }
    let clip = |id: &str| monster.animations.iter().find(|a| a.id == id);
    let duration = |id: &str| {
        clip(id).map(|a| {
            a.frames
                .iter()
                .map(|f| u64::from(f.duration_ms))
                .sum::<u64>()
        })
    };
    match d.archetype.as_str() {
        "SPLITTING_SLIME" => {
            let s = d.split.as_ref().ok_or_else(fail)?;
            if s.count != 2
                || !(1..=2).contains(&s.max_generations)
                || !(2..=6).contains(&s.max_offspring)
                || !s.child_scale.is_finite()
                || !(0.3..=0.75).contains(&s.child_scale)
                || !s.child_health_fraction.is_finite()
                || !(0.05..0.5).contains(&s.child_health_fraction)
                || !s.child_speed_multiplier.is_finite()
                || !(1.05..=2.0).contains(&s.child_speed_multiplier)
                || !s.separation_px.is_finite()
                || !(8.0..=128.0).contains(&s.separation_px)
                || !(300..=2000).contains(&s.emergence_ms)
                || duration(&s.emergence_animation_id) != Some(u64::from(s.emergence_ms))
                || clip(&s.burst_animation_id).is_none()
            {
                return Err(fail());
            }
        }
        "OBJECT_MIMIC" => {
            let m = d.mimic.as_ref().ok_or_else(fail)?;
            let lure = profile.lure.as_ref().ok_or_else(fail)?;
            if !["BUSH", "LARGE_BANANA", "VASE"].contains(&m.disguise.as_str())
                || !m.trigger_radius.is_finite()
                || !(16.0..=512.0).contains(&m.trigger_radius)
                || lure.trigger_radius != m.trigger_radius
                || lure.telegraph_ms != m.reveal_ms
                || lure.effect != "OBJECT_DISGUISE"
                || lure.reset_policy != "STAY_REVEALED"
            {
                return Err(fail());
            }
        }
        "SHIELD_SENTINEL" => {
            let s = d.shield.as_ref().ok_or_else(fail)?;
            if !s.front_arc_degrees.is_finite()
                || !(30.0..=160.0).contains(&s.front_arc_degrees)
                || !(200..=1500).contains(&s.bash_windup_ms)
                || clip(&s.guard_animation_id).is_none()
                || duration(&s.bash_animation_id).is_none_or(|ms| ms <= u64::from(s.bash_windup_ms))
                || duration(&s.recovery_animation_id) != Some(u64::from(s.bash_recovery_ms))
            {
                return Err(fail());
            }
        }
        "BOULDER_PUSHER" => {
            let p = d.pushed_prop.as_ref().ok_or_else(fail)?;
            if p.kind != "BOULDER"
                || p.id_suffix != "boulder"
                || p.coordinate_space != "LOCAL_PIXELS_X_FORWARD_Y_DOWN"
                || !p.radius.is_finite()
                || !(8.0..=64.0).contains(&p.radius)
                || !p.health.is_finite()
                || !(1.0..=1000.0).contains(&p.health)
                || !p.push_speed.is_finite()
                || !(20.0..=400.0).contains(&p.push_speed)
                || !p.anchor_x.is_finite()
                || !(p.radius..=160.0).contains(&p.anchor_x)
                || !p.anchor_y.is_finite()
                || !(-128.0..=0.0).contains(&p.anchor_y)
                || clip(&p.recovery_animation_id).is_none()
                || p.release_event != "PUSH_RELEASE"
                || clip(&p.push_animation_id).is_none_or(|a| {
                    a.events
                        .iter()
                        .filter(|e| {
                            e.kind == p.release_event
                                && e.reference_id == p.id_suffix
                                && u64::from(e.time_ms)
                                    < duration(&p.push_animation_id).unwrap_or(0)
                        })
                        .count()
                        != 1
                })
            {
                return Err(fail());
            }
        }
        _ => return Err(fail()),
    }
    Ok(())
}
