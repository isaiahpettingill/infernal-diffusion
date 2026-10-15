//! Authored encounter rigs and bounded presentation clips. Targeting, splitting,
//! shield blocking and the independent boulder belong to the consuming engine.
use crate::{
    anatomy::{Body, Node},
    animation::Pose,
    parser::MonsterSpec,
    proto::{Animation, AnimationEvent, Attack, AttackStep, MovementMode},
};
use std::f32::consts::{PI, TAU};

pub fn enabled(spec: &MonsterSpec) -> bool {
    has(spec, "PROFILE_V2")
        && [
            "SPLITTING_SLIME",
            "OBJECT_MIMIC",
            "SHIELD_SENTINEL",
            "BOULDER_PUSHER",
        ]
        .iter()
        .any(|f| has(spec, f))
}
fn has(spec: &MonsterSpec, feature: &str) -> bool {
    spec.features.iter().any(|f| f == feature)
}

#[allow(clippy::too_many_arguments)]
fn node(
    body: &mut Body,
    id: &str,
    kind: &str,
    parent: Option<usize>,
    p: [f32; 3],
    r: [f32; 3],
    material: &str,
    s: f32,
) -> usize {
    let i = body.nodes.len();
    body.nodes.push(Node {
        id: id.into(),
        kind: kind.into(),
        parent,
        x: p[0] * s,
        y: p[1] * s,
        z: Some(p[2] * s),
        rx: r[0] * s,
        ry: r[1] * s,
        rz: Some(r[2] * s),
        angle: 0.0,
        material: material.into(),
        density: if material == "METAL" { 2.5 } else { 1.0 },
        feature: false,
    });
    i
}

/// These actors have deliberate silhouettes. Replacing their provisional legacy
/// topology prevents a mimic sprouting a humanoid head or a pusher holding a rock.
pub fn refine(spec: &MonsterSpec, body: &mut Body) -> bool {
    if !enabled(spec) || spec.mount.is_some() {
        return false;
    }
    body.nodes.clear();
    let s = crate::anatomy::morphology_scale(spec.size);
    if has(spec, "SPLITTING_SLIME") {
        node(
            body,
            "torso",
            "TORSO",
            None,
            [0.0, -10.0, 0.0],
            [16.0, 10.0, 13.0],
            "SLIME",
            s,
        );
        node(
            body,
            "mouth_0",
            "MOUTH",
            Some(0),
            [13.0, -8.0, 0.0],
            [2.0, 2.0, 4.0],
            "SLIME",
            s,
        );
    } else if has(spec, "OBJECT_MIMIC") {
        let material = if has(spec, "MIMIC_BUSH") {
            "LEAF"
        } else if has(spec, "MIMIC_BANANA") {
            "FLESH_OCHRE"
        } else {
            "STONE"
        };
        node(
            body,
            "torso",
            "TORSO",
            None,
            [0.0, -15.0, 0.0],
            [14.0, 15.0, 12.0],
            material,
            s,
        );
        node(
            body,
            "v2_mimic_upper",
            "SHELL",
            Some(0),
            [0.0, -25.0, 0.0],
            [14.0, 10.0, 12.0],
            material,
            s,
        );
        node(
            body,
            "v2_mimic_lower",
            "SHELL",
            Some(0),
            [0.0, -10.0, 0.0],
            [14.0, 10.0, 12.0],
            material,
            s,
        );
        node(
            body,
            "mouth_0",
            "MOUTH",
            Some(0),
            [12.0, -15.0, 0.0],
            [3.0, 5.0, 9.0],
            "FLESH",
            s,
        );
    } else {
        let pusher = has(spec, "BOULDER_PUSHER");
        let material = if pusher { "FLESH_ASH" } else { "METAL" };
        node(
            body,
            "torso",
            "TORSO",
            None,
            [if pusher { -7.0 } else { 0.0 }, -24.0, 0.0],
            [
                if pusher { 12.0 } else { 8.0 },
                if pusher { 14.0 } else { 12.0 },
                if pusher { 10.0 } else { 7.0 },
            ],
            material,
            s,
        );
        node(
            body,
            "head_0",
            "HEAD",
            Some(0),
            [
                if pusher { 5.0 } else { 0.0 },
                if pusher { -29.0 } else { -41.0 },
                0.0,
            ],
            [if pusher { 7.0 } else { 6.0 }, 7.0, 6.0],
            material,
            s,
        );
        node(
            body,
            "mouth_0",
            "MOUTH",
            Some(1),
            [
                if pusher { 11.0 } else { 5.0 },
                if pusher { -27.0 } else { -39.0 },
                0.0,
            ],
            [2.0, 2.0, 3.0],
            material,
            s,
        );
        for side in 0..2 {
            let z = if side == 0 { 8.0 } else { -8.0 };
            let hip = node(
                body,
                &format!("hip_{side}"),
                "HIP",
                Some(0),
                [-5.0, -17.0, z * 0.72],
                [4.5, 4.5, 4.0],
                material,
                s,
            );
            let knee = node(
                body,
                &format!("leg_{side}"),
                "LIMB",
                Some(hip),
                [if pusher { 2.0 } else { -1.0 }, -9.0, z],
                [4.0, 5.0, 3.7],
                material,
                s,
            );
            let shin = node(
                body,
                &format!("shin_{side}"),
                "SHIN",
                Some(knee),
                [-3.0, -4.0, z],
                [3.0, 4.0, 3.0],
                material,
                s,
            );
            node(
                body,
                &format!("foot_{side}"),
                "FOOT",
                Some(shin),
                [2.0, -2.0, z],
                [6.0, 2.0, 4.0],
                material,
                s,
            );
            let shoulder = node(
                body,
                &format!("shoulder_{side}"),
                "SHOULDER",
                Some(0),
                [if pusher { -8.0 } else { 0.0 }, -32.0, z],
                [5.0, 5.0, 4.0],
                material,
                s,
            );
            let elbow = node(
                body,
                &format!("arm_{side}"),
                "ARM",
                Some(shoulder),
                [if pusher { 2.0 } else { 6.0 }, -18.0, z],
                [4.0, 6.0, 4.0],
                material,
                s,
            );
            node(
                body,
                &format!("hand_{side}"),
                "HAND",
                Some(elbow),
                [if pusher { 18.0 } else { 11.0 }, -23.0, z],
                [4.0, 4.0, 3.5],
                material,
                s,
            );
        }
        if !pusher {
            node(
                body,
                "v2_shield",
                "SHIELD",
                Some(0),
                [13.0, -24.0, 0.0],
                [3.0, 23.0, 22.0],
                "METAL",
                s,
            );
            node(
                body,
                "v2_core",
                "CORE",
                Some(0),
                [6.0, -25.0, 0.0],
                [3.0, 5.0, 4.5],
                "ENERGY",
                s,
            );
            node(
                body,
                "v2_helmet_crest",
                "CREST",
                Some(1),
                [-1.0, -50.0, 0.0],
                [8.0, 4.0, 2.5],
                "RED_CLOTH",
                s,
            );
        }
    }
    true
}

/// Bounded seed allometry is applied to the rest graph, not just the artwork, so
/// sprites, colliders and posed attack origins continue to describe one body.
pub fn vary_rest_shape(body: &mut Body, seed: u64) {
    let mut bits = seed.wrapping_add(0x9e3779b97f4a7c15);
    bits = (bits ^ (bits >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    bits = (bits ^ (bits >> 27)).wrapping_mul(0x94d049bb133111eb);
    bits ^= bits >> 31;
    let length = 0.90 + (bits & 0xffff) as f32 / 65535.0 * 0.20;
    let breadth = 0.90 + ((bits >> 16) & 0xffff) as f32 / 65535.0 * 0.20;
    for n in &mut body.nodes {
        n.x *= length;
        n.rx *= length;
        n.z = n.z.map(|v| v * breadth);
        n.rz = n.rz.map(|v| v * breadth);
    }
}

fn event(clip: &mut Animation, time_ms: u32, kind: &str, reference_id: &str) {
    clip.events.push(AnimationEvent {
        time_ms,
        kind: kind.into(),
        reference_id: reference_id.into(),
    });
}
fn clip(
    body: &Body,
    poses: &mut Vec<Pose>,
    id: &str,
    state: &str,
    count: u32,
    frame_ms: u32,
    looped: bool,
) -> Animation {
    let mut frames = Vec::new();
    for i in 0..count {
        frames.push(crate::proto::AnimationFrame {
            sprite_frame_id: poses.len() as u32,
            duration_ms: frame_ms,
            root_dx: 0.0,
            root_dy: 0.0,
        });
        poses.push(Pose {
            clip_id: id.into(),
            state: state.into(),
            phase: i as f32 / if looped { count } else { count - 1 } as f32,
            root_x: 0.0,
            root_y: 0.0,
            root_angle: 0.0,
            physical_positions: body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect(),
            motion_archetype: "ENCOUNTER".into(),
        });
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
fn index(body: &Body, id: &str) -> Option<usize> {
    body.nodes.iter().position(|n| n.id == id)
}
fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}
fn envelope(phase: f32, peak: f32) -> f32 {
    smooth(if phase <= peak {
        phase / peak
    } else {
        (1.0 - phase) / (1.0 - peak)
    })
}
fn move_all(pose: &mut Pose, dx: f32, dy: f32) {
    for p in &mut pose.physical_positions {
        p.0 += dx;
        p.1 += dy;
    }
}

/// Solve a two-link arm without extending the limb or moving its fixed shoulder.
fn arm(body: &Body, pose: &mut Pose, side: usize, target: (f32, f32)) {
    let Some(h) = index(body, &format!("hand_{side}")) else {
        return;
    };
    let Some(e) = body.nodes[h].parent else {
        return;
    };
    let Some(s) = body.nodes[e].parent else {
        return;
    };
    let a = pose.physical_positions[s];
    let l1 = (body.nodes[e].x - body.nodes[s].x).hypot(body.nodes[e].y - body.nodes[s].y);
    let l2 = (body.nodes[h].x - body.nodes[e].x).hypot(body.nodes[h].y - body.nodes[e].y);
    let dx = target.0 - a.0;
    let dy = target.1 - a.1;
    let d = dx
        .hypot(dy)
        .max(0.001)
        .clamp((l1 - l2).abs() + 0.001, l1 + l2 - 0.001);
    let distance = dx.hypot(dy).max(0.001);
    let ux = dx / distance;
    let uy = dy / distance;
    let along = (l1 * l1 - l2 * l2 + d * d) / (2.0 * d);
    let height = (l1 * l1 - along * along).max(0.0).sqrt();
    pose.physical_positions[e] = (
        a.0 + along * ux - height * uy,
        a.1 + along * uy + height * ux,
        0.0,
    );
    pose.physical_positions[h] = (a.0 + d * ux, a.1 + d * uy, 0.0);
}

fn pose(spec: &MonsterSpec, body: &Body, pose: &mut Pose) {
    pose.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
    pose.root_x = 0.0;
    pose.root_y = 0.0;
    pose.root_angle = 0.0;
    pose.motion_archetype = "ENCOUNTER".into();
    let s = crate::anatomy::morphology_scale(spec.size);
    let phase = pose.phase;
    let movement = matches!(pose.state.as_str(), "MOVE" | "FAST_MOVE");
    let attack = pose.state.starts_with("ATTACK") || pose.state == "SPECIAL_ATTACK";
    if has(spec, "SPLITTING_SLIME") {
        if movement {
            move_all(pose, 0.0, -8.0 * s * (phase * PI).sin().max(0.0));
        } else if attack {
            move_all(
                pose,
                5.0 * s * envelope(phase, 0.5),
                -3.0 * s * (phase * PI).sin(),
            );
        }
    } else if has(spec, "OBJECT_MIMIC") {
        // No gait, wave or ambient twitch gives away the environmental disguise.
        let open = if pose.clip_id == "v2_reveal" {
            smooth(phase)
        } else if pose.clip_id == "v2_lure" {
            0.0
        } else if attack {
            0.72 + 0.28 * envelope(phase, 0.45)
        } else {
            // Host selects the lure before reveal and normal clips afterward.
            // Thus an ordinary idle never silently re-disguises a revealed actor.
            0.72
        };
        if let Some(i) = index(body, "v2_mimic_upper") {
            pose.physical_positions[i].1 -= open * 10.0 * s;
            pose.physical_positions[i].2 = -open * 0.35;
        }
        if let Some(i) = index(body, "v2_mimic_lower") {
            pose.physical_positions[i].1 += open * 2.0 * s;
        }
        if let Some(i) = index(body, "mouth_0") {
            pose.physical_positions[i].1 -= open * 2.0 * s;
        }
    } else {
        if movement {
            for side in 0..2 {
                let swing = (phase * TAU + side as f32 * PI).sin();
                for prefix in ["leg", "shin", "foot"] {
                    if let Some(i) = index(body, &format!("{prefix}_{side}")) {
                        pose.physical_positions[i].0 += swing * 3.5 * s;
                        pose.physical_positions[i].1 -= swing.max(0.0) * 1.5 * s;
                    }
                }
            }
            // Keep grounded feet while the heavy upper body rocks slightly.
            for (i, n) in body.nodes.iter().enumerate() {
                if !matches!(n.kind.as_str(), "HIP" | "LIMB" | "SHIN" | "FOOT") {
                    pose.physical_positions[i].1 -= (phase * TAU).cos() * 0.6 * s;
                }
            }
        }
        if has(spec, "SHIELD_SENTINEL") {
            let bash = if pose.clip_id == "v2_shield_bash" {
                envelope(phase, 0.52)
            } else {
                0.0
            };
            let recover = if pose.clip_id == "v2_shield_recovery" {
                1.0 - smooth(phase)
            } else if pose.clip_id == "v2_shield_bash" {
                // The bash settles into exactly the first vulnerable recovery
                // pose, so consecutive one-shot clips cannot pop back to guard.
                smooth((phase - 0.60) / 0.40)
            } else {
                0.0
            };
            if let Some(i) = index(body, "v2_shield") {
                pose.physical_positions[i].0 += (8.0 * bash - 15.0 * recover) * s;
                pose.physical_positions[i].1 += 12.0 * recover * s;
                pose.physical_positions[i].2 = -0.95 * recover;
            }
            for side in 0..2 {
                if let Some(i) = index(body, &format!("hand_{side}")) {
                    arm(
                        body,
                        pose,
                        side,
                        (
                            body.nodes[i].x + (7.0 * bash - 8.0 * recover) * s,
                            body.nodes[i].y + 8.0 * recover * s,
                        ),
                    );
                }
            }
            if bash > 0.0 {
                if let Some(i) = index(body, "head_0") {
                    pose.physical_positions[i].0 += 2.0 * bash * s;
                }
            }
        } else {
            let push = if pose.clip_id == "v2_push" {
                smooth(phase / 0.78)
            } else if pose.clip_id == "v2_push_recovery" {
                1.0 - smooth(phase)
            } else if attack {
                envelope(phase, 0.45)
            } else {
                0.0
            };
            for side in 0..2 {
                if let Some(i) = index(body, &format!("hand_{side}")) {
                    arm(
                        body,
                        pose,
                        side,
                        (
                            body.nodes[i].x + 8.0 * push * s,
                            body.nodes[i].y - 2.0 * push * s,
                        ),
                    );
                }
            }
            if let Some(i) = index(body, "head_0") {
                pose.physical_positions[i].0 += push * 3.0 * s;
                pose.physical_positions[i].1 += push * 2.0 * s;
            }
        }
    }
    if pose.state == "DEATH" {
        let collapse = smooth(phase);
        for p in &mut pose.physical_positions {
            p.1 *= 1.0 - 0.78 * collapse;
            p.0 -= 5.0 * collapse * s;
            p.2 -= 0.45 * collapse;
        }
    } else if matches!(
        pose.state.as_str(),
        "IMPACT_LIGHT" | "IMPACT_HEAVY" | "KNOCKBACK"
    ) {
        move_all(pose, -3.0 * s * (phase * PI).sin(), 0.0);
    } else if pose.state == "STUN" {
        for p in &mut pose.physical_positions {
            p.0 += (phase * TAU).sin() * s;
        }
    }
}

/// Reuse the common reaction/movement vocabulary, then replace only encounter
/// artwork and bind the special presentations to explicit engine-facing events.
pub fn animate(
    spec: &MonsterSpec,
    body: &Body,
    clips: &mut Vec<Animation>,
    modes: &mut [MovementMode],
    attacks: &mut Vec<Attack>,
    poses: &mut Vec<Pose>,
) {
    if !enabled(spec) {
        return;
    }
    attacks.retain(|a| a.id == "primary" || a.id == "secondary");
    let origin = if has(spec, "SHIELD_SENTINEL") {
        "v2_shield"
    } else if has(spec, "BOULDER_PUSHER") {
        "hand_0"
    } else {
        "mouth_0"
    };
    for attack in attacks.iter_mut() {
        attack.delivery = "MELEE".into();
        attack.element = "PHYSICAL".into();
        attack.origin_node = origin.into();
        attack.projectile_id.clear();
        attack.spawn = None;
        attack.range = 28.0;
        let time = attack.telegraph_ms;
        attack.steps = vec![
            AttackStep {
                primitive: "TELEGRAPH".into(),
                value: 1.0,
                ..Default::default()
            },
            AttackStep {
                primitive: if has(spec, "OBJECT_MIMIC") {
                    "BITE"
                } else {
                    "SWIPE"
                }
                .into(),
                at_ms: time,
                target: "TARGET_ACTOR".into(),
                radius: 18.0,
                duration_ms: 100,
                value: 1.0,
                ..Default::default()
            },
            AttackStep {
                primitive: "COOLDOWN".into(),
                at_ms: time + 180,
                value: 1.0,
                ..Default::default()
            },
        ];
    }
    // Drop legacy mobility/special offense alongside the discarded attacks.
    clips.retain(|c| {
        !c.events.iter().any(|e| {
            matches!(
                e.kind.as_str(),
                "ATTACK" | "ATTACK_HIT" | "PROJECTILE_SPAWN" | "FIELD_SPAWN" | "MINION_SPAWN"
            ) && !attacks.iter().any(|a| a.id == e.reference_id)
        })
    });
    // Remove unbound magical/summon clips. Their gameplay belongs to an engine.
    clips.retain(|c| !matches!(c.id.as_str(), "attack_special" | "attack_summon"));
    if has(spec, "SPLITTING_SLIME") {
        let mut emerge = clip(body, poses, "v2_emerge", "EMERGE", 12, 60, false);
        event(&mut emerge, 720, "EMERGE_COMPLETE", "v2_emerge");
        clips.push(emerge);
        let mut split = clip(body, poses, "v2_split_burst", "SPLIT", 12, 50, false);
        event(&mut split, 300, "SPLIT_BURST", "v2_split_burst");
        clips.push(split);
    }
    if has(spec, "OBJECT_MIMIC") {
        let mut lure = clip(body, poses, "v2_lure", "LURE", 8, 120, true);
        event(&mut lure, 0, "OBJECT_DISGUISE", "v2_lure");
        clips.push(lure);
        let mut reveal = clip(body, poses, "v2_reveal", "REVEAL", 12, 60, false);
        event(&mut reveal, 0, "REVEAL", "v2_reveal");
        event(&mut reveal, 720, "REVEAL_COMPLETE", "v2_reveal");
        clips.push(reveal);
    }
    if has(spec, "SHIELD_SENTINEL") {
        clips.push(clip(body, poses, "v2_guard", "GUARD", 8, 120, true));
        let mut bash = clip(body, poses, "v2_shield_bash", "ATTACK", 16, 50, false);
        event(&mut bash, 0, "TELEGRAPH", "primary");
        event(&mut bash, 400, "ATTACK", "primary");
        event(&mut bash, 400, "ATTACK_HIT", "primary");
        clips.push(bash);
        clips.push(clip(
            body,
            poses,
            "v2_shield_recovery",
            "RECOVERY",
            12,
            60,
            false,
        ));
        if let Some(a) = attacks.iter_mut().find(|a| a.id == "primary") {
            a.animation_id = "v2_shield_bash".into();
            a.telegraph_ms = 400;
            a.cooldown_ms = 1520;
            a.steps[1].at_ms = 400;
            a.steps[2].at_ms = 650;
        }
        clips.retain(|c| c.id != "attack_primary");
    }
    if has(spec, "BOULDER_PUSHER") {
        let mut push = clip(body, poses, "v2_push", "PUSH", 16, 60, false);
        event(&mut push, 720, "PUSH_RELEASE", "boulder");
        clips.push(push);
        clips.push(clip(
            body,
            poses,
            "v2_push_recovery",
            "RECOVERY",
            12,
            60,
            false,
        ));
    }
    for p in poses.iter_mut() {
        pose(spec, body, p);
    }
    // Root deltas describe locomotion only; clip artwork is in-place.
    for mode in modes {
        if let Some(c) = clips.iter_mut().find(|c| c.id == mode.animation_id) {
            for f in &mut c.frames {
                f.root_dx = mode.speed * f.duration_ms as f32 / 1000.0;
                f.root_dy = 0.0;
            }
        }
    }
    // Dropped legacy clips must not occupy atlas frames or alter direction stride.
    let mut remap = std::collections::HashMap::new();
    let mut kept = Vec::new();
    for c in clips {
        for f in &mut c.frames {
            let old = f.sprite_frame_id;
            f.sprite_frame_id = *remap.entry(old).or_insert_with(|| {
                let id = kept.len() as u32;
                kept.push(poses[old as usize].clone());
                id
            });
        }
    }
    *poses = kept;
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    fn specimen(feature: &str) -> (MonsterSpec, Body, Vec<Animation>, Vec<Attack>, Vec<Pose>) {
        let mut spec = crate::parser::parse_vocabulary("humanoid");
        spec.features.extend(["PROFILE_V2".into(), feature.into()]);
        if feature == "OBJECT_MIMIC" {
            spec.features
                .extend(["MIMIC_BUSH".into(), "FRIENDLY_LURE".into()]);
        }
        let mut body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(7));
        crate::v2::anatomy::refine(&spec, &mut body, 7);
        let (c, _, a, p) = crate::v2::combat::animate(&spec, &body, 7);
        (spec, body, c, a, p)
    }
    #[test]
    fn encounter_topologies_and_clips_are_attached_and_bounded() {
        for flag in [
            "SPLITTING_SLIME",
            "OBJECT_MIMIC",
            "SHIELD_SENTINEL",
            "BOULDER_PUSHER",
        ] {
            let (_, body, clips, attacks, poses) = specimen(flag);
            assert!(body
                .nodes
                .iter()
                .enumerate()
                .all(|(i, n)| n.parent.is_none_or(|p| p < i)));
            assert!(body.nodes.iter().all(|n| n.kind != "WEAPON"));
            assert!(
                attacks
                    .iter()
                    .all(|a| a.delivery == "MELEE"
                        && body.nodes.iter().any(|n| n.id == a.origin_node))
            );
            crate::v2::combat::validate(&clips, &attacks, &poses).unwrap();
            assert!(poses
                .iter()
                .all(|p| p.physical_positions.len() == body.nodes.len()
                    && p.physical_positions
                        .iter()
                        .all(|v| v.0.is_finite() && v.1.is_finite() && v.2.is_finite())));
            for c in &clips {
                assert!(c
                    .frames
                    .iter()
                    .all(|f| (f.sprite_frame_id as usize) < poses.len()));
            }
        }
    }
    #[test]
    fn mimic_disguise_is_motionless_and_reveal_opens_real_jaws() {
        let (_, body, clips, _, poses) = specimen("OBJECT_MIMIC");
        let lure = clips.iter().find(|c| c.id == "v2_lure").unwrap();
        assert_eq!(lure.events[0].kind, "OBJECT_DISGUISE");
        assert!(!clips
            .iter()
            .flat_map(|c| &c.events)
            .any(|e| e.kind == "LURE_HEARTS"));
        for f in &lure.frames {
            assert_eq!(
                poses[f.sprite_frame_id as usize].physical_positions,
                poses[lure.frames[0].sprite_frame_id as usize].physical_positions
            );
        }
        let reveal = clips.iter().find(|c| c.id == "v2_reveal").unwrap();
        assert_eq!(
            reveal.frames.iter().map(|f| f.duration_ms).sum::<u32>(),
            720
        );
        let upper = index(&body, "v2_mimic_upper").unwrap();
        assert!(
            poses[reveal.frames.last().unwrap().sprite_frame_id as usize].physical_positions[upper]
                .1
                < body.nodes[upper].y - 5.0
        );
    }
    #[test]
    fn pusher_has_empty_hands_and_release_is_non_damaging() {
        let (_, body, clips, _, poses) = specimen("BOULDER_PUSHER");
        assert_eq!(body.nodes.iter().filter(|n| n.kind == "HAND").count(), 2);
        assert!(!body
            .nodes
            .iter()
            .any(|n| n.id.contains("rock") || n.id.contains("boulder")));
        let push = clips.iter().find(|c| c.id == "v2_push").unwrap();
        assert_eq!(push.events.len(), 1);
        assert_eq!(push.events[0].kind, "PUSH_RELEASE");
        assert_eq!(push.events[0].reference_id, "boulder");
        assert_eq!(push.events[0].time_ms, 720);
        for f in &push.frames {
            let p = &poses[f.sprite_frame_id as usize];
            for side in 0..2 {
                let hand = index(&body, &format!("hand_{side}")).unwrap();
                let elbow = body.nodes[hand].parent.unwrap();
                let shoulder = body.nodes[elbow].parent.unwrap();
                for (a, b) in [(hand, elbow), (elbow, shoulder)] {
                    let rest = (body.nodes[a].x - body.nodes[b].x)
                        .hypot(body.nodes[a].y - body.nodes[b].y);
                    let now = (p.physical_positions[a].0 - p.physical_positions[b].0)
                        .hypot(p.physical_positions[a].1 - p.physical_positions[b].1);
                    assert!((rest - now).abs() < 0.01);
                }
            }
        }
    }
    #[test]
    fn seeded_allometry_is_deterministic_and_clips_join_continuously() {
        let (_, mut body, _, _, _) = specimen("SHIELD_SENTINEL");
        let mut same = body.clone();
        let mut different = body.clone();
        vary_rest_shape(&mut body, 7);
        vary_rest_shape(&mut same, 7);
        vary_rest_shape(&mut different, 42);
        assert_eq!(body.nodes[0].rx, same.nodes[0].rx);
        assert_ne!(body.nodes[0].rx, different.nodes[0].rx);
        let (_, _, clips, _, poses) = specimen("SHIELD_SENTINEL");
        let bash = clips.iter().find(|c| c.id == "v2_shield_bash").unwrap();
        let recovery = clips.iter().find(|c| c.id == "v2_shield_recovery").unwrap();
        assert_eq!(
            poses[bash.frames.last().unwrap().sprite_frame_id as usize].physical_positions,
            poses[recovery.frames[0].sprite_frame_id as usize].physical_positions
        );
    }
    #[test]
    fn sentinel_recovery_moves_shield_off_core() {
        let (_, body, clips, _, poses) = specimen("SHIELD_SENTINEL");
        let shield = index(&body, "v2_shield").unwrap();
        let core = index(&body, "v2_core").unwrap();
        assert!(body.nodes[shield].ry > body.nodes[0].ry * 1.8);
        let recovery = clips.iter().find(|c| c.id == "v2_shield_recovery").unwrap();
        let p = &poses[recovery.frames[0].sprite_frame_id as usize];
        assert!(p.physical_positions[shield].0 < p.physical_positions[core].0);
    }
}
