use crate::{anatomy::Body, animation::Pose};
use rapier2d::prelude::*;

#[derive(Clone, Copy)]
enum Scenario {
    Stand,
    Impact(f32),
    Recoil,
    Death,
}

fn physical(kind: &str) -> bool {
    !matches!(kind, "EYE" | "MOUTH" | "HORN")
}

fn simulate(body: &Body, scenario: Scenario, count: usize) -> Vec<Vec<(f32, f32, f32)>> {
    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let mut handles = vec![None; body.nodes.len()];
    let gravity_scale = if matches!(scenario, Scenario::Death) {
        1.0
    } else {
        body.gravity
    };
    for (i, node) in body.nodes.iter().enumerate() {
        if !physical(&node.kind) {
            continue;
        }
        let area = std::f32::consts::PI * node.rx * node.ry;
        let handle = bodies.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector::new(node.x, node.y))
                .rotation(node.angle)
                .gravity_scale(gravity_scale)
                .linear_damping(if matches!(scenario, Scenario::Death) {
                    0.35
                } else {
                    2.5
                })
                .angular_damping(if matches!(scenario, Scenario::Death) {
                    0.35
                } else {
                    3.0
                })
                .additional_mass((area * node.density * 0.1).max(0.1))
                .can_sleep(false)
                .build(),
        );
        let solid = matches!(node.kind.as_str(), "TORSO" | "FOOT" | "TAIL");
        colliders.insert_with_parent(
            ColliderBuilder::ball(node.rx.min(node.ry).clamp(0.6, 5.0))
                .sensor(!solid)
                .density(0.0)
                .friction(0.9)
                .build(),
            handle,
            &mut bodies,
        );
        handles[i] = Some(handle);
    }
    let ground_y = body
        .nodes
        .iter()
        .filter(|node| matches!(node.kind.as_str(), "FOOT" | "SOLE"))
        .map(|node| node.y + node.ry)
        .reduce(f32::max)
        .unwrap_or_else(|| body.nodes[0].y + body.nodes[0].ry);
    colliders.insert(
        ColliderBuilder::cuboid(200.0, 1.0)
            .translation(Vector::new(0.0, ground_y + 1.0))
            .friction(0.9)
            .build(),
    );
    let mut joints = ImpulseJointSet::new();
    for (i, node) in body.nodes.iter().enumerate() {
        let Some(child) = handles[i] else {
            continue;
        };
        let Some(parent_index) = node.parent else {
            continue;
        };
        let Some(parent) = handles[parent_index] else {
            continue;
        };
        let p = &body.nodes[parent_index];
        let delta = Vector::new(node.x - p.x, node.y - p.y);
        let joint = RevoluteJointBuilder::new()
            .local_anchor1(delta * 0.5)
            .local_anchor2(-delta * 0.5)
            .limits(if matches!(scenario, Scenario::Death) {
                [-1.5, 1.5]
            } else {
                [-0.7, 0.7]
            })
            .motor_position(
                0.0,
                if matches!(scenario, Scenario::Death) {
                    0.0
                } else {
                    80.0
                },
                if matches!(scenario, Scenario::Death) {
                    0.0
                } else {
                    12.0
                },
            )
            .motor_max_force(if matches!(scenario, Scenario::Death) {
                0.0
            } else {
                300.0
            });
        joints.insert(parent, child, joint, true);
    }
    let torso = handles[0].expect("anatomy must have a torso");
    let mass = bodies[torso].mass();
    match scenario {
        Scenario::Stand => {}
        Scenario::Impact(force) => {
            bodies[torso].apply_impulse(Vector::new(mass * force, 0.0), true);
            bodies[torso].apply_torque_impulse(mass * force * 0.15, true);
        }
        Scenario::Recoil => {
            bodies[torso].apply_impulse(Vector::new(-mass * 0.75, 0.0), true);
            bodies[torso].apply_torque_impulse(-mass * 0.08, true);
        }
        Scenario::Death => {
            bodies[torso].apply_impulse(Vector::new(mass * 0.6, 0.0), true);
            bodies[torso].apply_torque_impulse(mass * 1.3, true);
        }
    }
    let mut pipeline = PhysicsPipeline::new();
    let mut islands = IslandManager::new();
    let mut broad = BroadPhaseBvh::new();
    let mut narrow = NarrowPhase::new();
    let mut multibody = MultibodyJointSet::new();
    let mut ccd = CCDSolver::new();
    let params = IntegrationParameters {
        dt: 1.0 / 120.0,
        ..Default::default()
    };
    let mut samples = Vec::with_capacity(count);
    for step in 0..count * 6 {
        pipeline.step(
            Vector::new(0.0, 9.81),
            &params,
            &mut islands,
            &mut broad,
            &mut narrow,
            &mut bodies,
            &mut colliders,
            &mut joints,
            &mut multibody,
            &mut ccd,
            &(),
            &(),
        );
        if step % 6 == 5 {
            let mut positions = Vec::with_capacity(body.nodes.len());
            for (i, node) in body.nodes.iter().enumerate() {
                let (x, y, angle) = if let Some(handle) = handles[i] {
                    let rb = &bodies[handle];
                    (
                        rb.translation().x,
                        rb.translation().y,
                        rb.rotation().angle(),
                    )
                } else if let Some(parent) = node.parent {
                    let p = &body.nodes[parent];
                    let handle = handles[parent].expect("cosmetic parent must be physical");
                    let rb = &bodies[handle];
                    let angle = rb.rotation().angle() - p.angle;
                    let ca = angle.cos();
                    let sa = angle.sin();
                    let dx = node.x - p.x;
                    let dy = node.y - p.y;
                    (
                        rb.translation().x + dx * ca - dy * sa,
                        rb.translation().y + dx * sa + dy * ca,
                        node.angle + angle,
                    )
                } else {
                    (node.x, node.y, node.angle)
                };
                let limit = match scenario {
                    Scenario::Stand => 1000.0,
                    Scenario::Death => 18.0,
                    _ => 8.0,
                };
                positions.push((
                    node.x + (x - node.x).clamp(-limit, limit),
                    node.y + (y - node.y).clamp(-limit, limit),
                    node.angle + (angle - node.angle).clamp(-1.5, 1.5),
                ));
            }
            samples.push(positions);
        }
    }
    samples
}

/// Briefly simulates a supported stance and adds bounded magical support when
/// the torso escapes its viable pose envelope.
pub fn assess_and_repair(body: &mut Body) {
    if body.gravity == 0.0 {
        return;
    }
    let samples = simulate(body, Scenario::Stand, 12);
    let Some(last) = samples.last() else {
        return;
    };
    let torso = last[0];
    let home = &body.nodes[0];
    let deviation = (torso.0 - home.x).abs() + (torso.1 - home.y).abs();
    if !deviation.is_finite() || deviation > 12.0 || (torso.2 - home.angle).abs() > 0.9 {
        body.gravity = body.gravity.min(0.65);
        if !body.repairs.iter().any(|r| r == "GRAVITY_REDUCTION") {
            body.repairs.push("GRAVITY_REDUCTION".into());
        }
        body.repairs.push("JOINT_REINFORCEMENT".into());
    }
}

pub fn apply_simulated_reactions(body: &Body, poses: &mut [Pose]) {
    for (state, scenario, count) in [
        ("IMPACT_LIGHT", Scenario::Impact(-0.65), 3),
        ("IMPACT_HEAVY", Scenario::Impact(-1.6), 4),
        ("KNOCKBACK", Scenario::Impact(-2.0), 4),
        ("ATTACK", Scenario::Recoil, 6),
        ("ATTACK_SECONDARY", Scenario::Recoil, 5),
        ("SPECIAL_ATTACK", Scenario::Recoil, 8),
        ("DEATH", Scenario::Death, 8),
    ] {
        let indices: Vec<usize> = poses
            .iter()
            .enumerate()
            .filter(|(_, p)| p.state == state)
            .map(|(i, _)| i)
            .collect();
        if indices.is_empty() {
            continue;
        }
        let samples = simulate(body, scenario, count);
        for (n, index) in indices.into_iter().enumerate() {
            let mut sample = samples[n % count].clone();
            if state.starts_with("IMPACT") && n >= count {
                for (i, position) in sample.iter_mut().enumerate() {
                    position.0 = body.nodes[i].x - (position.0 - body.nodes[i].x);
                    position.2 = body.nodes[i].angle - (position.2 - body.nodes[i].angle);
                }
            }
            poses[index].physical_positions = sample;
        }
    }
}
