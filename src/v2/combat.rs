//! V2 combat is bounded data: the consuming engine owns targeting and state transitions.
use crate::{
    anatomy::Body,
    animation::{self, Pose},
    parser::MonsterSpec,
    proto::*,
};

pub const LURE_CLIP: &str = "v2_lure";
pub const REVEAL_CLIP: &str = "v2_reveal";
pub const REVEAL_DURATION_MS: u32 = 720;
pub const LURE_RADIUS: f32 = 96.0;

fn enabled(spec: &MonsterSpec, feature: &str) -> bool {
    spec.features.iter().any(|f| f == feature)
}

/// Returns the pattern encoded by an attack, never inferred from its projectile art.
pub fn pattern_name(attack: &Attack) -> Option<&str> {
    attack.id.strip_prefix("v2_pattern_")
}

fn visual_clip(
    id: &str,
    state: &str,
    count: u32,
    frame_ms: u32,
    looped: bool,
    sources: &[Pose],
    poses: &mut Vec<Pose>,
) -> Animation {
    let mut frames = Vec::new();
    for i in 0..count {
        let phase = i as f32 / if looped { count } else { count - 1 } as f32;
        let mut pose = sources[((sources.len() - 1) as f32 * phase).round() as usize].clone();
        pose.clip_id = id.into();
        pose.state = state.into();
        pose.phase = phase;
        pose.root_x = 0.0;
        pose.root_y = 0.0;
        pose.root_angle = 0.0;
        frames.push(AnimationFrame {
            sprite_frame_id: poses.len() as u32,
            duration_ms: frame_ms,
            root_dx: 0.0,
            root_dy: 0.0,
        });
        poses.push(pose);
    }
    Animation {
        id: id.into(),
        semantic_state: state.into(),
        frames,
        events: vec![],
        looped,
        movement_mode: String::new(),
    }
}

/// Raise one anatomical arm as a rigid attached subtree, including held props.
/// Rotating about its shoulder preserves every parent-child distance.
fn friendly_wave(body: &Body, pose: &mut Pose) {
    if pose.physical_positions.len() != body.nodes.len() {
        return;
    }
    let Some(hand) = body.nodes.iter().position(|n| n.kind == "HAND") else {
        return;
    };
    let mut anchor = body.nodes[hand].parent;
    let mut shoulder = None;
    while let Some(i) = anchor {
        if body.nodes[i].kind == "SHOULDER" {
            shoulder = Some(i);
            break;
        }
        if body.nodes[i].kind == "TORSO" {
            break;
        }
        anchor = body.nodes[i].parent;
    }
    let Some(shoulder) = shoulder else {
        return;
    };
    let pivot = pose.physical_positions[shoulder];
    let side = if pose.physical_positions[hand].0 < pivot.0 {
        -1.0
    } else {
        1.0
    };
    let angle = -side * (1.8 + 0.22 * (pose.phase * std::f32::consts::TAU).sin());
    let (sin, cos) = angle.sin_cos();
    for i in 0..body.nodes.len() {
        let mut ancestor = Some(i);
        let mut attached = false;
        while let Some(j) = ancestor {
            if j == shoulder {
                attached = true;
                break;
            }
            ancestor = body.nodes[j].parent;
        }
        if attached {
            let p = &mut pose.physical_positions[i];
            let (x, y) = (p.0 - pivot.0, p.1 - pivot.1);
            p.0 = pivot.0 + x * cos - y * sin;
            p.1 = pivot.1 + x * sin + y * cos;
            p.2 += angle;
        }
    }
}

pub fn animate(
    spec: &MonsterSpec,
    body: &Body,
    seed: u64,
) -> (Vec<Animation>, Vec<MovementMode>, Vec<Attack>, Vec<Pose>) {
    let (mut clips, modes, mut attacks, mut poses) = animation::make(spec, body);
    if !enabled(spec, "PROFILE_V2") {
        return (clips, modes, attacks, poses);
    }
    let lure = enabled(spec, "FRIENDLY_LURE");
    let explicit = enabled(spec, "EXPLICIT_ATTACK");
    let ranged = spec.attack.delivery == "PROJECTILE" || (lure && !explicit);
    if ranged {
        let pattern =
            if lure && (!explicit || matches!(spec.attack.element.as_str(), "ACID" | "FIRE")) {
                if spec.attack.element == "FIRE" {
                    "fire_spray"
                } else {
                    "acid_spray"
                }
            } else if spec.attack.element == "ACID" {
                "acid_spray"
            } else if spec.attack.element == "FIRE"
                && matches!(spec.attack.concept.as_str(), "BREATH" | "SPRAY" | "FIRE")
            {
                "fire_spray"
            } else if spec.attack.concept == "SHOTGUN" {
                "fan"
            } else if matches!(spec.attack.concept.as_str(), "BOW" | "RAILGUN" | "LASER") {
                "aimed"
            } else {
                ["aimed", "burst", "fan"][(seed % 3) as usize]
            };
        if let Some(primary) = attacks.iter_mut().find(|a| a.id == "primary") {
            let (volleys, count, spread, interval, speed): (u32, u32, f32, u32, f32) = match pattern
            {
                "burst" => (3, 1, 0.0, 120, 170.0),
                "fan" => (1, 5, 48.0, 0, 145.0),
                "acid_spray" => (4, 3, 32.0, 120, 110.0),
                "fire_spray" => (5, 3, 40.0, 90, 140.0),
                _ => (1, 1, 0.0, 0, 210.0),
            };
            primary.id = format!("v2_pattern_{pattern}");
            primary.delivery = "PROJECTILE".into();
            if pattern == "acid_spray" {
                primary.element = "ACID".into();
            }
            if pattern == "fire_spray" {
                primary.element = "FIRE".into();
            }
            primary.telegraph_ms = 360;
            primary.cooldown_ms = 1200 + (volleys - 1) * interval;
            primary.range = speed * if pattern.ends_with("spray") { 0.8 } else { 1.2 };
            primary.damage = (primary.damage / (volleys * count).max(1)).max(2);
            primary.spawn = None;
            primary.origin_node = body
                .nodes
                .iter()
                .find(|n| n.id == primary.origin_node)
                .or_else(|| body.nodes.iter().find(|n| n.id == "mouth_0"))
                .or_else(|| body.nodes.first())
                .map(|n| n.id.clone())
                .unwrap_or_default();
            primary.steps = vec![AttackStep {
                primitive: "TELEGRAPH".into(),
                value: 1.0,
                ..Default::default()
            }];
            for volley in 0..volleys {
                primary.steps.push(AttackStep {
                    primitive: "SPAWN_PROJECTILE".into(),
                    at_ms: primary.telegraph_ms + volley * interval,
                    value: 1.0,
                    target: "TARGET_ACTOR".into(),
                    count,
                    spread_deg: spread,
                    speed,
                    radius: if pattern.ends_with("spray") { 3.0 } else { 4.0 },
                    duration_ms: if pattern.ends_with("spray") {
                        800
                    } else {
                        1200
                    },
                });
            }
            let end = primary.telegraph_ms + (volleys - 1) * interval + 240;
            primary.steps.push(AttackStep {
                primitive: "COOLDOWN".into(),
                at_ms: end,
                value: 1.0,
                ..Default::default()
            });
            let source: Vec<Pose> = poses
                .iter()
                .filter(|p| p.clip_id == primary.animation_id)
                .cloned()
                .collect();
            let mut clip = visual_clip(
                &primary.animation_id,
                "ATTACK",
                end / 30 + 2,
                30,
                false,
                &source,
                &mut poses,
            );
            clip.events.push(AnimationEvent {
                time_ms: 0,
                kind: "TELEGRAPH".into(),
                reference_id: primary.id.clone(),
            });
            clip.events.push(AnimationEvent {
                time_ms: primary.telegraph_ms,
                kind: "ATTACK".into(),
                reference_id: primary.id.clone(),
            });
            // Each real emission gets a matching event and a visible recoil pose.
            for step in primary
                .steps
                .iter()
                .filter(|s| s.primitive == "SPAWN_PROJECTILE")
            {
                clip.events.push(AnimationEvent {
                    time_ms: step.at_ms,
                    kind: "PROJECTILE_SPAWN".into(),
                    reference_id: primary.id.clone(),
                });
                let index = (step.at_ms / 30) as usize;
                if let Some(frame) = clip.frames.get(index) {
                    let pose = &mut poses[frame.sprite_frame_id as usize];
                    let recoil = &source[source.len() / 2];
                    pose.physical_positions = recoil.physical_positions.clone();
                    pose.phase = 0.5;
                }
            }
            if let Some(old) = clips.iter_mut().find(|c| c.id == primary.animation_id) {
                *old = clip;
            }
        }
    }
    if lure {
        let idle: Vec<Pose> = poses
            .iter()
            .filter(|p| p.clip_id == "idle")
            .cloned()
            .collect();
        let mut lure_clip = visual_clip(LURE_CLIP, "LURE", 8, 120, true, &idle, &mut poses);
        for frame in &lure_clip.frames {
            friendly_wave(body, &mut poses[frame.sprite_frame_id as usize]);
        }
        lure_clip.events.push(AnimationEvent {
            time_ms: 240,
            kind: "LURE_HEARTS".into(),
            reference_id: LURE_CLIP.into(),
        });
        clips.push(lure_clip);
        let mut reveal = visual_clip(REVEAL_CLIP, "REVEAL", 12, 60, false, &idle, &mut poses);
        reveal.events.push(AnimationEvent {
            time_ms: 0,
            kind: "REVEAL".into(),
            reference_id: REVEAL_CLIP.into(),
        });
        reveal.events.push(AnimationEvent {
            time_ms: REVEAL_DURATION_MS,
            kind: "REVEAL_COMPLETE".into(),
            reference_id: REVEAL_CLIP.into(),
        });
        clips.push(reveal);
    }
    (clips, modes, attacks, poses)
}

/// Guards the V2 extension without imposing new rules on legacy attacks.
pub fn validate_patterns(animations: &[Animation], attacks: &[Attack]) -> Result<(), String> {
    for attack in attacks.iter().filter(|a| pattern_name(a).is_some()) {
        let name = pattern_name(attack).unwrap();
        if !["aimed", "burst", "fan", "acid_spray", "fire_spray"].contains(&name)
            || attack.delivery != "PROJECTILE"
            || attack.spawn.is_some()
            || (name == "acid_spray" && attack.element != "ACID")
            || (name == "fire_spray" && attack.element != "FIRE")
        {
            return Err("incompatible V2 pattern and delivery/element".into());
        }
        let clip = animations
            .iter()
            .find(|c| c.id == attack.animation_id)
            .ok_or("missing V2 attack clip")?;
        let duration = clip
            .frames
            .iter()
            .try_fold(0u32, |n, f| n.checked_add(f.duration_ms))
            .ok_or("V2 clip duration overflow")?;
        let shots: Vec<_> = attack
            .steps
            .iter()
            .filter(|s| s.primitive == "SPAWN_PROJECTILE")
            .collect();
        if shots.is_empty() || shots.len() > 5 {
            return Err("invalid V2 volley count".into());
        }
        let (expected_volleys, expected_count, expected_spread, expected_interval) = match name {
            "aimed" => (1, 1, 0.0, 0),
            "burst" => (3, 1, 0.0, 120),
            "fan" => (1, 5, 48.0, 0),
            "acid_spray" => (4, 3, 32.0, 120),
            "fire_spray" => (5, 3, 40.0, 90),
            _ => unreachable!(),
        };
        if shots.len() != expected_volleys
            || shots.iter().any(|s| {
                s.count != expected_count
                    || s.spread_deg != expected_spread
                    || s.target != "TARGET_ACTOR"
            })
            || shots
                .windows(2)
                .any(|pair| pair[1].at_ms.checked_sub(pair[0].at_ms) != Some(expected_interval))
        {
            return Err("V2 pattern disagrees with bounded emission recipe".into());
        }
        for shot in shots {
            if !(1..=5).contains(&shot.count)
                || !shot.spread_deg.is_finite()
                || !(0.0..=60.0).contains(&shot.spread_deg)
                || !shot.speed.is_finite()
                || !(1.0..=250.0).contains(&shot.speed)
                || shot.at_ms < attack.telegraph_ms
                || shot.at_ms >= duration
                || shot.duration_ms > 1500
                || !shot.radius.is_finite()
                || !(0.0..=8.0).contains(&shot.radius)
                || !clip.events.iter().any(|e| {
                    e.kind == "PROJECTILE_SPAWN"
                        && e.time_ms == shot.at_ms
                        && e.reference_id == attack.id
                })
            {
                return Err("unbounded or visually unbound V2 emission".into());
            }
        }
    }
    Ok(())
}

pub fn validate(
    animations: &[Animation],
    attacks: &[Attack],
    poses: &[Pose],
) -> Result<(), String> {
    validate_patterns(animations, attacks)?;
    for clip in animations
        .iter()
        .filter(|c| c.id == LURE_CLIP || c.id == REVEAL_CLIP)
    {
        if attacks.iter().any(|a| a.animation_id == clip.id)
            || clip
                .events
                .iter()
                .any(|e| matches!(e.kind.as_str(), "ATTACK" | "PROJECTILE_SPAWN"))
            || clip.frames.iter().any(|f| {
                poses
                    .get(f.sprite_frame_id as usize)
                    .map(|p| p.state != clip.semantic_state)
                    .unwrap_or(true)
            })
        {
            return Err("lure/reveal must be harmless and visually linked".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    fn sample(
        prompt: &str,
        seed: u64,
        lure: bool,
    ) -> (Vec<Animation>, Vec<MovementMode>, Vec<Attack>, Vec<Pose>) {
        let mut spec = crate::parser::parse_vocabulary(prompt);
        spec.features.push("PROFILE_V2".into());
        if lure {
            spec.features.push("FRIENDLY_LURE".into());
        }
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(seed));
        animate(&spec, &body, seed)
    }
    #[test]
    fn patterns_bind_every_emission_to_visual_clip() {
        for seed in 0..9 {
            let (clips, _, attacks, poses) = sample("goblin shooting acid", seed, true);
            validate(&clips, &attacks, &poses).unwrap();
            let attack = attacks.iter().find(|a| pattern_name(a).is_some()).unwrap();
            assert_eq!(attack.element, "ACID");
            assert_eq!(
                attack
                    .steps
                    .iter()
                    .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                    .count(),
                4
            );
            assert!(clips.iter().any(|c| c.id == LURE_CLIP && c.looped));
            assert!(clips.iter().any(|c| c.id == REVEAL_CLIP && !c.looped));
        }
    }
    #[test]
    fn incompatible_pattern_and_unbound_emissions_rejected() {
        let (mut clips, _, mut attacks, poses) = sample("friendly goblin", 1, true);
        let index = attacks
            .iter()
            .position(|a| pattern_name(a).is_some())
            .unwrap();
        attacks[index].element = "ICE".into();
        assert!(validate(&clips, &attacks, &poses).is_err());
        attacks[index].element = "ACID".into();
        let clip = clips
            .iter_mut()
            .find(|c| c.id == attacks[index].animation_id)
            .unwrap();
        clip.events.retain(|e| e.kind != "PROJECTILE_SPAWN");
        assert!(validate(&clips, &attacks, &poses).is_err());
    }
    #[test]
    fn aimed_burst_and_fan_are_distinct_bounded_patterns() {
        let mut spec = crate::parser::parse_vocabulary("goblin");
        spec.features.push("PROFILE_V2".into());
        spec.attack.delivery = "PROJECTILE".into();
        spec.attack.element = "PHYSICAL".into();
        spec.attack.concept = "SPIT".into();
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
        for (seed, expected, volleys, count) in
            [(0, "aimed", 1, 1), (1, "burst", 3, 1), (2, "fan", 1, 5)]
        {
            let (clips, _, attacks, poses) = animate(&spec, &body, seed);
            validate(&clips, &attacks, &poses).unwrap();
            let attack = attacks.iter().find(|a| pattern_name(a).is_some()).unwrap();
            assert_eq!(pattern_name(attack), Some(expected));
            let shots: Vec<_> = attack
                .steps
                .iter()
                .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                .collect();
            assert_eq!(shots.len(), volleys);
            assert!(shots.iter().all(|s| s.count == count));
            let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
            for shot in shots {
                let frame = &clip.frames[(shot.at_ms / 30) as usize];
                assert_eq!(poses[frame.sprite_frame_id as usize].phase, 0.5);
            }
        }
    }
    #[test]
    fn explicit_ice_lure_keeps_ice_and_ice_projectiles() {
        let mut spec = crate::parser::parse_vocabulary("goblin");
        spec.features.extend([
            "PROFILE_V2".into(),
            "FRIENDLY_LURE".into(),
            "EXPLICIT_ATTACK".into(),
        ]);
        spec.attack.delivery = "PROJECTILE".into();
        spec.attack.element = "ICE".into();
        spec.attack.concept = "BOW".into();
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
        let (clips, _, attacks, poses) = animate(&spec, &body, 0);
        validate(&clips, &attacks, &poses).unwrap();
        let attack = attacks.iter().find(|a| pattern_name(a).is_some()).unwrap();
        assert_eq!(attack.element, "ICE");
        assert_eq!(crate::projectile::recipe_kind(&spec, attack), "ICE_SHARD");
        spec.features.retain(|f| f != "PROFILE_V2");
        assert_eq!(crate::projectile::recipe_kind(&spec, attack), "ARROW");
    }
    #[test]
    fn generated_patterns_and_lures_pass_package_validation() {
        for (seed, element, expected) in [
            (0, "PHYSICAL", "aimed"),
            (1, "PHYSICAL", "burst"),
            (2, "PHYSICAL", "fan"),
            (3, "ACID", "acid_spray"),
            (4, "FIRE", "fire_spray"),
        ] {
            let mut spec = crate::parser::parse_vocabulary("goblin");
            spec.features
                .extend(["PROFILE_V2".into(), "EXPLICIT_ATTACK".into()]);
            spec.attack.delivery = "PROJECTILE".into();
            spec.attack.element = element.into();
            spec.attack.concept = if element == "FIRE" { "BREATH" } else { "SPIT" }.into();
            let dir = tempfile::tempdir().unwrap();
            let monster = crate::generate_from_spec_with_profile(
                spec,
                seed,
                dir.path(),
                crate::PackageFormat::Protobuf,
                crate::GenerationProfile::V2,
            )
            .unwrap();
            assert!(monster
                .attacks
                .iter()
                .any(|a| pattern_name(a) == Some(expected)));
            let profile = monster.v2_profile.as_ref().unwrap();
            for descriptor in &profile.attack_patterns {
                let attack = monster
                    .attacks
                    .iter()
                    .find(|a| a.id == descriptor.attack_id)
                    .unwrap();
                assert_eq!(
                    descriptor.projectile_count,
                    attack
                        .steps
                        .iter()
                        .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                        .map(|s| s.count)
                        .sum::<u32>()
                );
            }
        }
        let generated = crate::generate_in_memory_with_profile(
            "friendly frog",
            7,
            crate::GenerationProfile::V2,
        )
        .unwrap();
        assert!(generated.packages.iter().any(|p| p
            .monster
            .v2_profile
            .as_ref()
            .and_then(|profile| profile.lure.as_ref())
            .is_some()));
        for package in generated.packages {
            crate::validate(&package.monster, &package.sprites).unwrap();
            if let Some(lure) = package
                .monster
                .v2_profile
                .as_ref()
                .and_then(|p| p.lure.as_ref())
            {
                assert!(lure.harmless);
                assert!(package
                    .monster
                    .animations
                    .iter()
                    .any(|a| a.id == lure.lure_animation_id && a.semantic_state == "LURE"));
                assert!(package
                    .monster
                    .animations
                    .iter()
                    .any(|a| a.id == lure.reveal_animation_id && a.semantic_state == "REVEAL"));
            }
        }
    }
    #[test]
    fn lure_wave_preserves_connected_arm_lengths() {
        let mut spec = crate::parser::parse_vocabulary("friendly goblin");
        spec.features
            .extend(["PROFILE_V2".into(), "FRIENDLY_LURE".into()]);
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(0));
        let (_, _, _, poses) = animate(&spec, &body, 0);
        let hand = body.nodes.iter().position(|n| n.kind == "HAND").unwrap();
        let lure: Vec<_> = poses.iter().filter(|p| p.state == "LURE").collect();
        assert!(
            (lure[0].physical_positions[hand].0 - lure[2].physical_positions[hand].0).abs() > 0.1
        );
        for pose in lure {
            for (i, node) in body.nodes.iter().enumerate() {
                if let Some(parent) = node.parent {
                    let p = pose.physical_positions[i];
                    let q = pose.physical_positions[parent];
                    let rest = (node.x - body.nodes[parent].x).hypot(node.y - body.nodes[parent].y);
                    assert!(
                        ((p.0 - q.0).hypot(p.1 - q.1) - rest).abs() < 0.02,
                        "wave detached {}",
                        node.id
                    );
                }
            }
        }
    }
    #[test]
    fn legacy_animation_is_unchanged_without_profile_feature() {
        let spec = crate::parser::parse_vocabulary("fire dragon");
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(7));
        let old = animation::make(&spec, &body);
        let new = animate(&spec, &body, 7);
        assert_eq!(old.0, new.0);
        assert_eq!(old.1, new.1);
        assert_eq!(old.2, new.2);
        assert_eq!(format!("{:?}", old.3), format!("{:?}", new.3));
    }
}
