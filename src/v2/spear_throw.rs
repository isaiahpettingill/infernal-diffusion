//! Deterministic V2 javelin variants. Release is an executable projectile event;
//! the carried spear is hidden until the recovery reaches its re-equip pose.
use crate::{anatomy::Body, animation::Pose, parser::MonsterSpec, proto::*};
pub const ATTACK: &str = "v2_spear_throw";
pub const CLIP: &str = "attack_v2_spear_throw";
pub const RELEASE_MS: u32 = 450;
const FRAME_MS: u32 = 50;
const FRAMES: u32 = 25;

pub fn selected(spec: &MonsterSpec, seed: u64) -> bool {
    spec.features.iter().any(|f| f == "PROFILE_V2")
        && !spec.features.iter().any(|f| f == "DANCER" || f == "SHADE")
        && spec.attack.concept == "SPEAR"
        && seed % 3 == 1
}

pub fn released(pose: &Pose) -> bool {
    pose.clip_id == CLIP && pose.phase >= 9.0 / 24.0 && pose.phase < 22.0 / 24.0
}

fn below(body: &Body, child: usize, root: usize) -> bool {
    let mut next = Some(child);
    for _ in 0..=body.nodes.len() {
        let Some(i) = next else {
            return false;
        };
        if i == root {
            return true;
        }
        next = body.nodes[i].parent;
    }
    false
}

fn rotate_about(body: &Body, pose: &mut Pose, root: usize, pivot: (f32, f32), angle: f32) {
    let (s, c) = angle.sin_cos();
    for i in 0..body.nodes.len() {
        if below(body, i, root) {
            let p = &mut pose.physical_positions[i];
            let x = p.0 - pivot.0;
            let y = p.1 - pivot.1;
            p.0 = pivot.0 + c * x - s * y;
            p.1 = pivot.1 + s * x + c * y;
            p.2 += angle;
        }
    }
}

fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn arm_angle(frame: u32) -> f32 {
    match frame {
        0..=6 => -1.65 * ease(frame as f32 / 6.0),
        7..=9 => -1.65 + 0.70 * ease((frame - 6) as f32 / 3.0),
        10..=14 => -0.95 + 1.38 * ease((frame - 9) as f32 / 5.0),
        15..=20 => 0.43 - 1.35 * ease((frame - 14) as f32 / 6.0),
        _ => -0.92 * (1.0 - ease((frame - 20) as f32 / 4.0)),
    }
}

pub fn apply(
    spec: &MonsterSpec,
    body: &Body,
    seed: u64,
    clips: &mut [Animation],
    attacks: &mut [Attack],
    poses: &mut Vec<Pose>,
) {
    if !selected(spec, seed) {
        return;
    }
    let Some(primary) = attacks.iter_mut().find(|a| a.id == "primary") else {
        return;
    };
    let Some(weapon) = body
        .nodes
        .iter()
        .position(|n| n.id == primary.origin_node && n.kind == "WEAPON")
    else {
        return;
    };
    let Some(hand) = body.nodes[weapon]
        .parent
        .filter(|&i| body.nodes[i].kind == "HAND")
    else {
        return;
    };
    let Some(elbow) = body.nodes[hand].parent else {
        return;
    };
    let Some(shoulder) = body.nodes[elbow]
        .parent
        .filter(|&i| body.nodes[i].kind == "SHOULDER")
    else {
        return;
    };
    let Some(clip) = clips.iter_mut().find(|c| c.id == primary.animation_id) else {
        return;
    };
    let mut neutral = poses[0].clone();
    neutral.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
    neutral.clip_id = CLIP.into();
    neutral.state = "ATTACK".into();
    neutral.root_x = 0.0;
    neutral.root_y = 0.0;
    neutral.root_angle = 0.0;
    clip.id = CLIP.into();
    clip.frames.clear();
    clip.events.clear();
    clip.looped = false;
    for frame in 0..FRAMES {
        let mut pose = neutral.clone();
        pose.phase = frame as f32 / (FRAMES - 1) as f32;
        let pivot = (body.nodes[shoulder].x, body.nodes[shoulder].y);
        rotate_about(body, &mut pose, shoulder, pivot, arm_angle(frame));
        // Orient the spear independently at the wrist, preserving the rigid
        // hand-to-weapon attachment distance throughout cock, release and return.
        let aiming = if frame <= 9 {
            ease(frame as f32 / 6.0)
        } else if frame >= 22 {
            1.0 - ease((frame - 22) as f32 / 2.0)
        } else {
            1.0
        };
        let angle = -pose.physical_positions[weapon].2 * aiming;
        let hp = pose.physical_positions[hand];
        rotate_about(body, &mut pose, weapon, (hp.0, hp.1), angle);
        clip.frames.push(AnimationFrame {
            sprite_frame_id: poses.len() as u32,
            duration_ms: FRAME_MS,
            root_dx: 0.0,
            root_dy: 0.0,
        });
        poses.push(pose);
    }
    primary.id = ATTACK.into();
    primary.animation_id = CLIP.into();
    primary.delivery = "PROJECTILE".into();
    primary.element = "PHYSICAL".into();
    primary.telegraph_ms = RELEASE_MS;
    primary.cooldown_ms = 1500;
    primary.range = 240.0;
    primary.projectile_id.clear();
    primary.spawn = None;
    primary.steps = vec![
        AttackStep {
            primitive: "TELEGRAPH".into(),
            value: 1.0,
            ..Default::default()
        },
        AttackStep {
            primitive: "SPAWN_PROJECTILE".into(),
            at_ms: RELEASE_MS,
            value: 1.0,
            target: "TARGET_ACTOR".into(),
            count: 1,
            speed: 200.0,
            radius: 3.0,
            duration_ms: 1200,
            ..Default::default()
        },
        AttackStep {
            primitive: "COOLDOWN".into(),
            at_ms: 1150,
            value: 1.0,
            ..Default::default()
        },
    ];
    for (time_ms, kind) in [
        (0, "TELEGRAPH"),
        (RELEASE_MS, "ATTACK"),
        (RELEASE_MS, "PROJECTILE_SPAWN"),
    ] {
        clip.events.push(AnimationEvent {
            time_ms,
            kind: kind.into(),
            reference_id: ATTACK.into(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    #[test]
    fn seeded_throw_has_release_recovery_and_thrust_alternative() {
        let mut spec = crate::v2::interpret(
            crate::parser::parse_vocabulary("orc with spear"),
            "orc with spear",
        );
        crate::v2::prepare(&mut spec);
        for seed in [7, 42, 43] {
            let mut spec = spec.clone();
            crate::v2::anatomy::prepare_seeded(&mut spec, seed);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(seed));
            crate::v2::anatomy::refine(&spec, &mut body, seed);
            let (clips, _, attacks, poses) = crate::v2::combat::animate(&spec, &body, seed);
            crate::v2::combat::validate_patterns(&clips, &attacks).unwrap();
            crate::v2::combat::validate_rig_extensions(&body, &clips, &poses).unwrap();
            if seed == 42 {
                assert!(attacks
                    .iter()
                    .any(|a| a.steps.iter().any(|s| s.primitive == "THRUST")));
                continue;
            }
            let attack = attacks.iter().find(|a| a.id == ATTACK).unwrap();
            assert_eq!(attack.delivery, "PROJECTILE");
            assert_eq!(
                attack
                    .steps
                    .iter()
                    .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                    .count(),
                1
            );
            let clip = clips.iter().find(|c| c.id == CLIP).unwrap();
            let release_ms = attack
                .steps
                .iter()
                .find(|s| s.primitive == "SPAWN_PROJECTILE")
                .unwrap()
                .at_ms;
            let mut time = 0;
            let release_frame = clip
                .frames
                .iter()
                .find(|frame| {
                    let starts_at = time;
                    time += frame.duration_ms;
                    starts_at <= release_ms && release_ms < time
                })
                .unwrap();
            assert!(released(&poses[release_frame.sprite_frame_id as usize]));
            assert!(!released(&poses[clip.frames[0].sprite_frame_id as usize]));
            assert!(!released(
                &poses[clip.frames.last().unwrap().sprite_frame_id as usize]
            ));
            for frame in &clip.frames {
                let p = &poses[frame.sprite_frame_id as usize];
                for (i, n) in body.nodes.iter().enumerate() {
                    if let Some(parent) = n.parent {
                        let rest = (n.x - body.nodes[parent].x).hypot(n.y - body.nodes[parent].y);
                        let now = (p.physical_positions[i].0 - p.physical_positions[parent].0)
                            .hypot(p.physical_positions[i].1 - p.physical_positions[parent].1);
                        assert!((rest - now).abs() < 0.005, "{}", n.id);
                    }
                }
            }
        }
    }
}
