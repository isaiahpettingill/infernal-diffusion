//! Baked attack-origin tracks use the same posed geometry and projection as the
//! sprite renderer. Rest colliders remain available, but are not substituted for
//! an animated growing limb's active contact location.
use crate::{anatomy::Body, animation::Pose, proto::*, render3d};

fn active_duration(step: &AttackStep) -> u32 {
    if step.primitive == "SPAWN_PROJECTILE" {
        1
    } else {
        step.duration_ms.max(1)
    }
}

fn damaging(step: &AttackStep) -> bool {
    [
        "SWIPE",
        "BITE",
        "THRUST",
        "SLAM",
        "SPAWN_PROJECTILE",
        "SPAWN_FIELD",
        "SPAWN_MINION",
    ]
    .contains(&step.primitive.as_str())
}

pub fn populate(
    profile: &mut V2Profile,
    body: &Body,
    clips: &[Animation],
    attacks: &[Attack],
    poses: &[Pose],
    size: u32,
) {
    for descriptor in &mut profile.attack_patterns {
        let Some(attack) = attacks
            .iter()
            .find(|attack| attack.id == descriptor.attack_id)
        else {
            continue;
        };
        let Some(clip) = clips.iter().find(|clip| clip.id == attack.animation_id) else {
            continue;
        };
        let mut origins: Vec<_> = body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| node.id == attack.origin_node)
            .map(|(index, _)| index)
            .collect();
        if attack.id == "v2_tree_bilateral_smash" {
            origins = body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.kind == "HAND")
                .take(2)
                .map(|(index, _)| index)
                .collect();
        }
        let mut time_ms = 0u32;
        for frame in &clip.frames {
            let Some(pose) = poses.get(frame.sprite_frame_id as usize) else {
                continue;
            };
            let end = time_ms.saturating_add(frame.duration_ms);
            let active_steps: Vec<_> = attack
                .steps
                .iter()
                .filter(|step| {
                    damaging(step)
                        && step.at_ms < end
                        && step.at_ms.saturating_add(active_duration(step)) > time_ms
                })
                .collect();
            for &index in &origins {
                let mut views = render3d::collider_views_for_pose(body, index, pose, size);
                let hit_radius = active_steps
                    .iter()
                    .map(|step| step.radius)
                    .filter(|radius| *radius > 0.0)
                    .fold(0.0, f32::max);
                if hit_radius > 0.0 {
                    for view in &mut views {
                        view.radius = hit_radius * render3d::PIXELS_PER_UNIT;
                    }
                }
                descriptor.motion_samples.push(AttackOriginSample {
                    time_ms,
                    sprite_frame_id: frame.sprite_frame_id,
                    node_id: body.nodes[index].id.clone(),
                    views,
                    active: !active_steps.is_empty(),
                });
            }
            time_ms = end;
        }
    }
    if profile
        .spectral
        .as_ref()
        .is_some_and(|spectral| spectral.kind == "SHADE")
    {
        for attack in attacks {
            let Some(clip) = clips.iter().find(|clip| clip.id == attack.animation_id) else {
                continue;
            };
            let mut time_ms = 0u32;
            for frame in &clip.frames {
                let Some(pose) = poses.get(frame.sprite_frame_id as usize) else {
                    continue;
                };
                let hurtboxes = body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, node)| {
                        !["EYE", "MOUTH", "WING", "FIN", "VORTEX", "WEAPON"]
                            .contains(&node.kind.as_str())
                    })
                    .map(|(index, node)| PosedHurtbox {
                        node_id: node.id.clone(),
                        views: render3d::collider_views_for_shade_receiver(body, index, pose, size),
                    })
                    .collect();
                profile.body_motion_samples.push(BodyMotionSample {
                    animation_id: clip.id.clone(),
                    time_ms,
                    sprite_frame_id: frame.sprite_frame_id,
                    hurtboxes,
                });
                time_ms = time_ms.saturating_add(frame.duration_ms);
            }
        }
    }
}

pub fn validate(monster: &Monster) -> Result<(), String> {
    let Some(profile) = &monster.v2_profile else {
        return Ok(());
    };
    for descriptor in &profile.attack_patterns {
        let attack = monster
            .attacks
            .iter()
            .find(|attack| attack.id == descriptor.attack_id)
            .ok_or("missing tracked attack")?;
        let clip = monster
            .animations
            .iter()
            .find(|clip| clip.id == attack.animation_id)
            .ok_or("missing tracked clip")?;
        // Lightweight recipe-only fixtures lack sprites/colliders; real imported
        // packages must always contain the complete baked contact track.
        if monster.sprites.is_none()
            && monster.colliders.is_empty()
            && descriptor.motion_samples.is_empty()
        {
            continue;
        }
        let origins = if attack.id == "v2_tree_bilateral_smash" {
            2
        } else {
            1
        };
        if descriptor.motion_samples.len() != clip.frames.len() * origins
            || descriptor.motion_samples.len() > 8192
        {
            return Err("attack motion track must cover every frame and contact origin".into());
        }
        let mut time_ms = 0u32;
        for (index, frame) in clip.frames.iter().enumerate() {
            let end = time_ms
                .checked_add(frame.duration_ms)
                .ok_or("attack track duration overflow")?;
            let expected_active = attack.steps.iter().any(|step| {
                damaging(step)
                    && step.at_ms < end
                    && step.at_ms.saturating_add(active_duration(step)) > time_ms
            });
            let mut seen = std::collections::BTreeSet::new();
            for sample in &descriptor.motion_samples[index * origins..(index + 1) * origins] {
                if sample.time_ms != time_ms
                    || sample.sprite_frame_id != frame.sprite_frame_id
                    || sample.active != expected_active
                    || !seen.insert(sample.node_id.as_str())
                    || !monster
                        .colliders
                        .iter()
                        .any(|collider| collider.node_id == sample.node_id)
                    || (origins == 1 && sample.node_id != attack.origin_node)
                    || sample.views.len() != render3d::ANGLES_DEG.len()
                    || sample.views.iter().enumerate().any(|(direction, view)| {
                        view.direction_index != direction as u32
                            || !view.x.is_finite()
                            || !view.y.is_finite()
                            || !view.radius.is_finite()
                            || view.radius <= 0.0
                            || view.radius > 4096.0
                    })
                {
                    return Err("invalid posed attack contact sample".into());
                }
            }
            time_ms = end;
        }
    }
    validate_body_motion(monster, profile)?;
    Ok(())
}

fn validate_body_motion(monster: &Monster, profile: &V2Profile) -> Result<(), String> {
    let shade = profile
        .spectral
        .as_ref()
        .is_some_and(|spectral| spectral.kind == "SHADE");
    if !shade {
        return if profile.body_motion_samples.is_empty() {
            Ok(())
        } else {
            Err("dynamic whole-body receivers require shade deformation".into())
        };
    }
    // Recipe-only unit fixtures have neither baked sprites nor collider data.
    // Real imports still require complete tracks even if an attacker removes them.
    if monster.sprites.is_none()
        && monster.colliders.is_empty()
        && profile.body_motion_samples.is_empty()
        && profile
            .attack_patterns
            .iter()
            .all(|pattern| pattern.motion_samples.is_empty())
    {
        return Ok(());
    }
    let mut shared_frames = std::collections::BTreeMap::new();
    let expected_nodes: std::collections::BTreeSet<_> = monster
        .colliders
        .iter()
        .filter(|collider| collider.hurtbox)
        .map(|collider| collider.node_id.as_str())
        .collect();
    if expected_nodes.is_empty() || profile.body_motion_samples.len() > 4096 {
        return Err("missing or excessive deformed receiver data".into());
    }
    let mut cursor = 0usize;
    for attack in &monster.attacks {
        let clip = monster
            .animations
            .iter()
            .find(|clip| clip.id == attack.animation_id)
            .ok_or("missing receiver clip")?;
        let mut time_ms = 0u32;
        for frame in &clip.frames {
            let sample = profile
                .body_motion_samples
                .get(cursor)
                .ok_or("missing deformed receiver frame")?;
            if sample.animation_id != clip.id
                || sample.time_ms != time_ms
                || sample.sprite_frame_id != frame.sprite_frame_id
                || sample.hurtboxes.len() != expected_nodes.len()
            {
                return Err("deformed receiver frame disagrees with animation".into());
            }
            let key = (sample.animation_id.as_str(), sample.sprite_frame_id);
            if let Some(previous) = shared_frames.insert(key, &sample.hurtboxes) {
                if previous != &sample.hurtboxes {
                    return Err("shared shade frame has inconsistent damage receivers".into());
                }
            }
            let mut seen = std::collections::BTreeSet::new();
            for hurtbox in &sample.hurtboxes {
                if !seen.insert(hurtbox.node_id.as_str())
                    || !expected_nodes.contains(hurtbox.node_id.as_str())
                    || hurtbox.views.len() != render3d::ANGLES_DEG.len()
                    || hurtbox.views.iter().enumerate().any(|(direction, view)| {
                        view.direction_index != direction as u32
                            || !view.x.is_finite()
                            || !view.y.is_finite()
                            || view.x.abs() > 1_000_000.0
                            || view.y.abs() > 1_000_000.0
                            || !view.radius.is_finite()
                            || view.radius <= 0.0
                            || view.radius > 4096.0
                    })
                {
                    return Err("invalid morphed damage receiver".into());
                }
            }
            cursor += 1;
            time_ms = time_ms
                .checked_add(frame.duration_ms)
                .ok_or("receiver duration overflow")?;
        }
    }
    if cursor != profile.body_motion_samples.len() {
        return Err("extra unbound deformed receiver frames".into());
    }
    Ok(())
}
