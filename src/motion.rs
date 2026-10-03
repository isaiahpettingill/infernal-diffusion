//! Deterministic, anatomy-constrained pose baking shared by the 2D and 3D bakes.
//!
//! Rotations propagate through the anatomical graph. Feet are solved against
//! contact targets instead of translating every limb independently. One-shot
//! actions have anticipation, contact and recovery; death finishes in a held pose.
use crate::{
    anatomy::Body,
    animation::Pose,
    proto::{Animation, Attack, MovementMode},
};
use std::f32::consts::{PI, TAU};

type Position = (f32, f32, f32);

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn ramp(t: f32, a: f32, b: f32) -> f32 {
    smooth((t - a) / (b - a).max(0.0001))
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
fn rotate(x: f32, y: f32, a: f32) -> (f32, f32) {
    (x * a.cos() - y * a.sin(), x * a.sin() + y * a.cos())
}
fn ordinal(id: &str) -> usize {
    id.rsplit('_')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}
fn is_rider(mut id: &str) -> bool {
    while let Some(rest) = id.strip_prefix("mount_") {
        id = rest;
    }
    id.starts_with("rider_")
}

fn distance(a: Position, b: Position) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The exact visible ground envelope of an oriented anatomical ellipse.
fn bottom(body: &Body, i: usize, p: Position) -> f32 {
    let n = &body.nodes[i];
    p.1 + ((n.rx * p.2.sin()).powi(2) + (n.ry * p.2.cos()).powi(2)).sqrt()
}
fn ground(body: &Body) -> f32 {
    let support = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.kind.as_str(), "FOOT" | "SOLE" | "TREAD" | "ROOT"))
        .map(|(i, n)| bottom(body, i, (n.x, n.y, n.angle)))
        .reduce(f32::max);
    support.unwrap_or_else(|| {
        body.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.feature)
            .map(|(i, n)| bottom(body, i, (n.x, n.y, n.angle)))
            .reduce(f32::max)
            .unwrap_or(0.0)
    })
}

fn forward(body: &Body, root: Position, local: &[f32]) -> Vec<Position> {
    let mut result: Vec<Position> = Vec::with_capacity(body.nodes.len());
    for (i, n) in body.nodes.iter().enumerate() {
        let p = if let Some(parent) = n.parent.filter(|&p| p < i) {
            let anchor = &body.nodes[parent];
            let ap = result[parent];
            let delta = ap.2 - anchor.angle + local[i];
            let (x, y) = rotate(n.x - anchor.x, n.y - anchor.y, delta);
            (ap.0 + x, ap.1 + y, n.angle + delta)
        } else {
            let origin = &body.nodes[0];
            let (x, y) = rotate(n.x - origin.x, n.y - origin.y, root.2);
            (
                origin.x + x + root.0,
                origin.y + y + root.1,
                n.angle + root.2 + local[i],
            )
        };
        result.push(p);
    }
    result
}

/// Reattach off-chain details after IK. A feature receives its parent's complete
/// transform (including rotation), so eyes/horns/weapons cannot drift away.
fn reattach(body: &Body, positions: &mut [Position], fixed: &[bool]) {
    for (i, n) in body.nodes.iter().enumerate() {
        if fixed[i] {
            continue;
        }
        if let Some(parent) = n.parent.filter(|&p| p < i) {
            let p = &body.nodes[parent];
            let a = positions[parent].2 - p.angle;
            let (x, y) = rotate(n.x - p.x, n.y - p.y, a);
            positions[i] = (
                positions[parent].0 + x,
                positions[parent].1 + y,
                n.angle + a,
            );
        }
    }
}

/// Anatomical chain from the endpoint to a hip/shoulder. Mounted rider feet must
/// never be constrained to the mount's ground plane.
fn chain(body: &Body, end: usize, anchor_kind: &str) -> Vec<usize> {
    let mut c = vec![end];
    let mut next = body.nodes[end].parent;
    while let Some(i) = next {
        c.push(i);
        if body.nodes[i].kind == anchor_kind {
            c.reverse();
            return c;
        }
        if body.nodes[i].kind == "TORSO" {
            break;
        }
        next = body.nodes[i].parent;
    }
    Vec::new()
}

/// FABRIK with fixed bone lengths. Reach clamping preserves the requested floor
/// height where possible; it never stretches the leg to meet an impossible step.
fn solve_chain(
    body: &Body,
    positions: &mut [Position],
    c: &[usize],
    target: (f32, f32),
    fixed: &mut [bool],
) {
    if c.len() < 3 {
        return;
    }
    let lengths: Vec<f32> = c
        .windows(2)
        .map(|w| {
            let a = &body.nodes[w[0]];
            let b = &body.nodes[w[1]];
            (a.x - b.x).hypot(a.y - b.y).max(0.001)
        })
        .collect();
    let reach = lengths.iter().sum::<f32>() * 0.995;
    let anchor = positions[c[0]];
    let dy = (target.1 - anchor.1).clamp(-reach, reach);
    let max_x = (reach * reach - dy * dy).max(0.0).sqrt();
    let target = (
        anchor.0 + (target.0 - anchor.0).clamp(-max_x, max_x),
        anchor.1 + dy,
        0.0,
    );
    let mut points: Vec<Position> = c.iter().map(|&i| positions[i]).collect();
    // A tiny anatomical bend disambiguates a completely straight rest chain.
    let side = if body.nodes[c[1]].x >= body.nodes[c[0]].x {
        1.0
    } else {
        -1.0
    };
    for p in points.iter_mut().skip(1).take(c.len() - 2) {
        p.0 += side * 0.12;
    }
    for _ in 0..48 {
        let last = points.len() - 1;
        points[last] = target;
        for j in (0..last).rev() {
            let d = distance(points[j], points[j + 1]).max(0.00001);
            let r = lengths[j] / d;
            points[j] = (
                points[j + 1].0 + (points[j].0 - points[j + 1].0) * r,
                points[j + 1].1 + (points[j].1 - points[j + 1].1) * r,
                0.0,
            );
        }
        points[0] = anchor;
        for j in 1..points.len() {
            let d = distance(points[j], points[j - 1]).max(0.00001);
            let r = lengths[j - 1] / d;
            points[j] = (
                points[j - 1].0 + (points[j].0 - points[j - 1].0) * r,
                points[j - 1].1 + (points[j].1 - points[j - 1].1) * r,
                0.0,
            );
        }
        if distance(*points.last().unwrap(), target) < 0.001 {
            break;
        }
    }
    for (j, &i) in c.iter().enumerate() {
        if j == 0 {
            fixed[i] = true;
            continue;
        }
        let n = &body.nodes[i];
        let parent = &body.nodes[c[j - 1]];
        let old = (n.y - parent.y).atan2(n.x - parent.x);
        let now = (points[j].1 - points[j - 1].1).atan2(points[j].0 - points[j - 1].0);
        positions[i] = (points[j].0, points[j].1, n.angle + now - old);
        fixed[i] = true;
    }
    // End effectors stay flat through stance, and attached claws rotate with them.
    let end = *c.last().unwrap();
    positions[end].2 = body.nodes[end].angle;
}

fn phase_offset(archetype: &str, mode: &str, leg: usize) -> f32 {
    if mode.contains("HOP") {
        return 0.0;
    }
    match archetype {
        "QUADRUPED" | "DRAGON" | "MOUNTED" => {
            if mode.contains("RUN") {
                [0.0, 0.08, 0.5, 0.58][leg % 4]
            } else {
                [0.0, 0.5, 0.75, 0.25][leg % 4]
            }
        }
        // Paired indexing: front L/R=0/1, then the next anatomical pair.
        // Insects use alternating tripods 0,3,4 and 1,2,5; spiders use
        // alternating contralateral tetrapods with the same rule.
        "ARACHNID" | "INSECT" => ((leg / 2 + leg % 2) % 2) as f32 * 0.5,
        "CENTIPEDE" => (leg / 2) as f32 * 0.13 + (leg % 2) as f32 * 0.5,
        _ => (leg % 2) as f32 * 0.5,
    }
}

/// Linear stance and eased swing form a continuous periodic foot trajectory.
/// Contact occupies most of a walk; faster gaits have shorter duty cycles.
fn foot_cycle(phase: f32, duty: f32, stride: f32, lift: f32) -> (f32, f32) {
    let p = phase.rem_euclid(1.0);
    if p < duty {
        (stride * (0.5 - p / duty), 0.0)
    } else {
        let s = (p - duty) / (1.0 - duty);
        (
            -stride * 0.5 + stride * smooth(s),
            -lift * (PI * s).sin().powi(2),
        )
    }
}

/// Limit the locomotion clock to the leg's available stance workspace. Otherwise
/// a short leg reaches its IK limit halfway through stance and visibly slides,
/// even though every bone length remains valid. Root deltas are generated from
/// this same resulting speed, so visual contacts and exported travel agree.
pub(super) fn limit_ground_speed(body: &Body, clips: &[Animation], modes: &mut [MovementMode]) {
    let scale = (body.nodes[0].ry / 6.0).clamp(0.65, 2.0);
    for mode in modes
        .iter_mut()
        .filter(|m| matches!(m.animation_id.as_str(), "move" | "fast_move"))
    {
        if mode.speed <= 0.0 || mode.id.contains("HOP") {
            continue;
        }
        let Some(clip) = clips.iter().find(|c| c.id == mode.animation_id) else {
            continue;
        };
        let fast = clip.semantic_state == "FAST_MOVE";
        let duty = if mode.id.contains("LIMP") {
            0.8
        } else if fast {
            0.5
        } else {
            0.65
        };
        let duration = clip.frames.iter().map(|f| f.duration_ms).sum::<u32>() as f32 / 1000.0;
        let lower = (if fast { 1.8 } else { 1.1 } - 0.35) * scale;
        let mut half_stride = f32::INFINITY;
        for (i, n) in body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "FOOT" && !is_rider(&n.id))
        {
            let c = chain(body, i, "HIP");
            if c.len() < 3 {
                continue;
            }
            let hip = &body.nodes[c[0]];
            let reach = c
                .windows(2)
                .map(|w| {
                    (body.nodes[w[0]].x - body.nodes[w[1]].x)
                        .hypot(body.nodes[w[0]].y - body.nodes[w[1]].y)
                })
                .sum::<f32>()
                * 0.98;
            let vertical = (n.y - hip.y - lower + (hip.x - body.nodes[0].x).abs() * 0.022).abs();
            let horizontal = (reach * reach - vertical * vertical).max(0.0).sqrt();
            let available = (horizontal - (n.x - hip.x).abs() - 0.2 * scale).max(0.5 * scale);
            half_stride = half_stride.min(available);
        }
        if half_stride.is_finite() {
            mode.speed = mode.speed.min(2.0 * half_stride / (duration * duty));
        }
    }
}

/// Footfall metadata is derived from the same gait phase as the visible feet.
/// Aerial, gliding and stationary clips cannot emit invented footsteps.
pub(super) fn sync_contacts(
    body: &Body,
    clips: &mut [Animation],
    modes: &[MovementMode],
    poses: &[Pose],
) {
    for clip in clips
        .iter_mut()
        .filter(|c| matches!(c.id.as_str(), "move" | "fast_move"))
    {
        clip.events.retain(|e| e.kind != "FOOT_CONTACT");
        let Some(mode) = modes
            .iter()
            .find(|m| m.animation_id == clip.id && m.speed > 0.0)
        else {
            continue;
        };
        let archetype = poses[clip.frames[0].sprite_frame_id as usize]
            .motion_archetype
            .as_str();
        for foot in body
            .nodes
            .iter()
            .filter(|n| n.kind == "FOOT" && !is_rider(&n.id))
        {
            let phase = (-phase_offset(archetype, &mode.id, ordinal(&foot.id))).rem_euclid(1.0);
            let frame = ((phase * clip.frames.len() as f32).round() as usize) % clip.frames.len();
            clip.events.push(crate::proto::AnimationEvent {
                time_ms: clip.frames[..frame].iter().map(|f| f.duration_ms).sum(),
                kind: "FOOT_CONTACT".into(),
                reference_id: foot.id.clone(),
            });
        }
        clip.events.sort_by_key(|e| e.time_ms);
    }
}

struct Action {
    wind: f32,
    hit: f32,
    recovery: f32,
}
fn action(clip: &Animation, attack: &Attack, frame: usize) -> Action {
    let hit_ms = attack
        .steps
        .iter()
        .filter(|s| {
            matches!(
                s.primitive.as_str(),
                "SWIPE"
                    | "BITE"
                    | "THRUST"
                    | "SLAM"
                    | "SPAWN_PROJECTILE"
                    | "SPAWN_FIELD"
                    | "SPAWN_MINION"
            )
        })
        .map(|s| s.at_ms)
        .max()
        .unwrap_or(attack.telegraph_ms);
    let mut elapsed = 0;
    let mut contact = 0;
    for (i, f) in clip.frames.iter().enumerate() {
        if elapsed <= hit_ms {
            contact = i;
        }
        elapsed += f.duration_ms;
    }
    let end = (clip.frames.len() - 1) as f32;
    let contact = (contact as f32).clamp(2.0, (end - 1.0).max(2.0));
    let t = frame as f32;
    let wind_end = (contact - 1.0).max(1.0);
    let wind = ramp(t, 0.0, wind_end) * (1.0 - ramp(t, wind_end, contact));
    let hit = ramp(t, wind_end, contact) * (1.0 - ramp(t, contact, end));
    Action {
        wind,
        hit,
        recovery: ramp(t, contact, end),
    }
}

pub(super) fn bake(
    body: &Body,
    clips: &[Animation],
    modes: &[MovementMode],
    attacks: &[Attack],
    poses: &mut [Pose],
) {
    if body.nodes.is_empty() {
        return;
    }
    let floor = ground(body);
    let scale = (body.nodes[0].ry / 6.0).clamp(0.65, 2.0);
    for clip in clips {
        let attack = attacks.iter().find(|a| a.animation_id == clip.id);
        let mode = clip.movement_mode.as_str();
        let speed = modes
            .iter()
            .find(|m| m.animation_id == clip.id)
            .map_or(0.0, |m| m.speed);
        let duration = clip.frames.iter().map(|f| f.duration_ms).sum::<u32>() as f32 / 1000.0;
        for (frame, f) in clip.frames.iter().enumerate() {
            let pose = &mut poses[f.sprite_frame_id as usize];
            let p = pose.phase;
            let t = p * TAU;
            let archetype = pose.motion_archetype.as_str();
            let mut local = vec![0.0; body.nodes.len()];
            let mut root = (0.0, 0.0, 0.0);
            let mut planted = true;
            let fast = pose.state == "FAST_MOVE";
            let airborne = matches!(pose.state.as_str(), "FLY" | "FLY_TURN" | "JUMP_AIR");
            let moving = matches!(pose.state.as_str(), "MOVE" | "FAST_MOVE" | "CLIMB");
            let soft = matches!(
                archetype,
                "BLOB" | "GASTROPOD" | "SERPENT" | "SAND_WORM" | "AQUATIC"
            );
            let a = attack.map(|attack| action(clip, attack, frame));
            match pose.state.as_str() {
                "IDLE" => {
                    root.1 = t.sin() * 0.35 * scale;
                }
                "MOVE" | "FAST_MOVE" => {
                    root.1 = if mode.contains("HOP") {
                        -5.0 * (PI * p).sin().powi(2) * scale
                    } else if mode.starts_with("ANCHORED") {
                        0.0
                    } else if body.gravity == 0.0 {
                        t.sin() * 1.4 * scale
                    } else {
                        (if fast { 1.8 } else { 1.1 }) * scale + (2.0 * t).cos() * 0.35 * scale
                    };
                    root.2 = if mode.starts_with("ANCHORED") {
                        t.sin() * 0.07
                    } else if mode.contains("FLOP") || mode.contains("THRASH") {
                        t.sin() * 0.14
                    } else {
                        t.sin() * 0.022
                    };
                    if soft {
                        root.0 = t.sin() * 0.8 * scale;
                        root.1 = t.cos() * 0.5 * scale;
                    }
                }
                "FLY" => {
                    root.1 = -5.0 * scale - t.sin() * 0.8 * scale;
                    root.2 = t.sin() * 0.035;
                    planted = false;
                }
                "TURN" | "FLY_TURN" => {
                    let s = (PI * p).sin().powi(2);
                    root.1 = s * scale;
                    root.2 = -0.14 * s;
                    planted = !airborne;
                    if airborne {
                        root.1 -= 5.0 * scale;
                    }
                }
                "CLIMB" => {
                    root.0 = t.sin() * 0.4 * scale;
                    root.2 = -0.16;
                    planted = false;
                }
                "JUMP_START" | "TAKEOFF" => {
                    root.1 = (2.2 * ramp(p, 0.0, 0.35) - 7.2 * ramp(p, 0.35, 1.0)) * scale;
                    root.2 = -0.08 * (PI * p).sin();
                    planted = p < 0.4;
                }
                "JUMP_AIR" => {
                    root.1 = -5.0 * scale;
                    planted = false;
                }
                "LAND" | "LAND_FROM_FLIGHT" => {
                    root.1 = if p < 0.4 {
                        mix(-5.0, 2.0, ramp(p, 0.0, 0.4)) * scale
                    } else {
                        2.0 * (1.0 - ramp(p, 0.4, 1.0)) * scale
                    };
                    planted = p >= 0.4;
                }
                "BURROW" => {
                    root.1 = 24.0 * ramp(p, 0.15, 1.0) * scale;
                    root.2 = 0.3 * (PI * p).sin();
                    planted = false;
                }
                "BURROWED_MOVE" => {
                    root.1 = 24.0 * scale + t.sin() * 0.3;
                    planted = false;
                }
                "EMERGE" => {
                    root.1 = (24.0 * (1.0 - ramp(p, 0.0, 0.7)) - 1.5 * (PI * p).sin()) * scale;
                    planted = p > 0.8;
                }
                "IMPACT_LIGHT" | "IMPACT_HEAVY" | "KNOCKBACK" => {
                    let heavy = pose.state != "IMPACT_LIGHT";
                    let strength = if heavy { 1.0 } else { 0.45 };
                    let envelope = (1.0 - p).powi(2) * (1.0 + 0.22 * (p * PI * 3.0).sin());
                    let side = if clip.id.ends_with("back") { 1.0 } else { -1.0 };
                    root.0 = side * 3.5 * strength * scale * envelope;
                    root.1 = 1.3 * strength * scale * envelope;
                    root.2 = side * 0.2 * strength * envelope;
                }
                "STUN" => {
                    let e = (PI * p).sin();
                    root.1 = 1.1 * e * scale;
                    root.2 = (p * TAU * 2.0).sin() * 0.085 * e;
                }
                "DEATH" => {
                    let collapse = ramp(p, 0.12, 0.73);
                    let settle = ramp(p, 0.73, 0.9);
                    let spectral = matches!(archetype, "FLOATING" | "WHIRLWIND");
                    root.2 = if spectral {
                        0.2 * collapse
                    } else if matches!(archetype, "ARACHNID" | "INSECT" | "CENTIPEDE") {
                        0.13 * collapse
                    } else if soft || archetype == "TANK" {
                        0.08 * collapse
                    } else if matches!(archetype, "QUADRUPED" | "DRAGON" | "MOUNTED") {
                        0.12 * collapse
                    } else {
                        1.38 * collapse
                    };
                    root.0 = (-1.2 * ramp(p, 0.0, 0.12) + 3.5 * collapse) * scale;
                    root.1 = if spectral {
                        -8.0 * collapse * scale
                    } else {
                        (2.0 * ramp(p, 0.0, 0.3) + 10.0 * collapse + 0.5 * (1.0 - settle)) * scale
                    };
                    planted = false;
                }
                _ => {}
            }
            if moving && archetype == "TANK" {
                root.1 = t.sin() * 0.12 * scale;
                root.2 = t.sin() * 0.005;
            }
            if moving && mode.contains("LIMP") {
                root.1 += (t + 0.6).sin().max(0.0) * 1.3 * scale;
            }
            if let Some(a) = &a {
                root.0 = (-1.5 * a.wind + 2.7 * a.hit) * scale;
                root.1 = (1.2 * a.wind + 0.8 * a.hit) * scale;
                root.2 = -0.055 * a.wind + 0.11 * a.hit;
                if attack.is_some_and(|a| a.delivery == "PROJECTILE") {
                    root.0 = (-1.2 * a.wind - 2.0 * a.hit) * scale;
                    root.2 = -0.07 * a.hit;
                }
                match clip.id.as_str() {
                    "attack_leap_slam" => {
                        root.1 -= 7.0 * (PI * ramp(p, 0.15, 0.75)).sin().max(0.0) * scale;
                        planted = !(0.2..0.7).contains(&p);
                    }
                    "attack_swoop" => {
                        root.1 -= 8.0 * (PI * p).sin() * scale;
                        root.2 += 0.25 * (TAU * p).sin();
                        planted = false;
                    }
                    "attack_charge" => {
                        root.1 += 0.8 * a.wind * scale;
                        root.2 += 0.1 * a.hit;
                    }
                    "attack_dodge_counter" => {
                        root.0 -= 4.0 * (PI * p).sin() * scale;
                    }
                    "attack_burrow_ambush" => {
                        root.1 += 16.0 * (PI * ramp(p, 0.0, 0.85)).sin().max(0.0) * scale;
                        planted = false;
                    }
                    "attack_blink_strike" => {
                        root.0 += 3.0 * (PI * p).sin() * scale;
                    }
                    _ => {}
                }
            }
            for (i, n) in body.nodes.iter().enumerate().skip(1) {
                let order = ordinal(&n.id) as f32;
                match n.kind.as_str() {
                    "HEAD" => {
                        local[i] = if pose.state == "IDLE" {
                            (t - 0.6).sin() * 0.025
                        } else if moving {
                            -root.2 * 0.7
                        } else {
                            0.0
                        }
                    }
                    "TAIL" | "TENTACLE" => {
                        local[i] = (t - order * 0.45).sin() * if soft { 0.07 } else { 0.035 }
                    }
                    "ARM" => {
                        if moving {
                            local[i] =
                                (t + 0.25 + phase_offset(archetype, mode, order as usize) * TAU)
                                    .sin()
                                    * if fast { 0.36 } else { 0.24 };
                        }
                    }
                    "WING" => {
                        let side = if (order as usize).is_multiple_of(2) {
                            1.0
                        } else {
                            -1.0
                        };
                        let flap = airborne
                            || matches!(pose.state.as_str(), "TAKEOFF" | "LAND_FROM_FLIGHT")
                            || clip.id == "attack_swoop";
                        local[i] = side
                            * if flap {
                                (t - 0.4).sin() * 0.65
                            } else {
                                (t + 0.35).sin() * 0.045
                            };
                    }
                    "ROOT" if moving => local[i] = (t + order * 1.7).sin() * 0.2,
                    "LOBE" if moving && archetype == "BLOB" => {
                        local[i] = (t - order * 0.8).sin() * 0.07
                    }
                    "SOLE" if moving => local[i] = (t + 0.3).sin() * 0.025,
                    "TREAD" if moving => local[i] = (t + order * PI).sin() * 0.012,
                    "ROTOR" => local[i] = t * 2.0,
                    "VORTEX" => local[i] = (t - order * 0.3).sin() * 0.12,
                    _ => {}
                }
                if airborne && matches!(n.kind.as_str(), "LIMB" | "SHIN") {
                    local[i] = 0.32;
                }
                if pose.state == "JUMP_START"
                    && p > 0.45
                    && matches!(n.kind.as_str(), "LIMB" | "SHIN")
                {
                    local[i] = 0.32 * ramp(p, 0.45, 1.0);
                }
                if pose.state == "DEATH" {
                    let c = ramp(p, 0.14 + (order % 3.0) * 0.025, 0.75);
                    local[i] = match n.kind.as_str() {
                        "HEAD" => {
                            if matches!(archetype, "QUADRUPED" | "DRAGON" | "MOUNTED" | "THEROPOD")
                            {
                                0.08 * c
                            } else {
                                0.2 * c
                            }
                        }
                        "NECK" => {
                            if matches!(archetype, "QUADRUPED" | "DRAGON" | "MOUNTED" | "THEROPOD")
                            {
                                0.03 * c
                            } else {
                                0.13 * c
                            }
                        }
                        "ARM" => {
                            if matches!(archetype, "ARACHNID" | "INSECT" | "CENTIPEDE") {
                                -0.3 * c
                            } else if n.x < body.nodes[0].x {
                                -0.4 * c
                            } else {
                                0.65 * c
                            }
                        }
                        "HAND" => {
                            if matches!(archetype, "ARACHNID" | "INSECT" | "CENTIPEDE") {
                                0.2 * c
                            } else {
                                0.3 * c
                            }
                        }
                        "LIMB" => {
                            if matches!(archetype, "ARACHNID" | "INSECT" | "CENTIPEDE") {
                                if n.x < body.nodes[n.parent.unwrap_or(0)].x {
                                    -0.9 * c
                                } else {
                                    0.9 * c
                                }
                            } else {
                                -0.4 * c
                            }
                        }
                        "SHIN" => 0.8 * c,
                        "FOOT" => {
                            if matches!(archetype, "ARACHNID" | "INSECT" | "CENTIPEDE") {
                                if n.x < body.nodes[0].x {
                                    0.9 * c
                                } else {
                                    -0.9 * c
                                }
                            } else {
                                0.0
                            }
                        }
                        "WING" => {
                            if (order as usize).is_multiple_of(2) {
                                0.5 * c
                            } else {
                                -0.5 * c
                            }
                        }
                        "TAIL" | "TENTACLE" => 0.1 * c,
                        _ => 0.0,
                    };
                }
            }
            // Bite, tail and horn attacks act through their actual attachment.
            if let (Some(attack), Some(a)) = (attack, &a) {
                let origin = body
                    .nodes
                    .iter()
                    .position(|n| n.id == attack.origin_node)
                    .unwrap_or(0);
                let mut current = Some(origin);
                while let Some(i) = current {
                    match body.nodes[i].kind.as_str() {
                        "HEAD" => local[i] += -0.18 * a.wind + 0.24 * a.hit,
                        "NECK" => local[i] += -0.14 * a.wind + 0.2 * a.hit,
                        "TAIL" | "STINGER" | "TENTACLE" => local[i] += -0.5 * a.wind + 0.7 * a.hit,
                        _ => {}
                    }
                    current = body.nodes[i].parent;
                }
            }
            if pose.state == "DEATH"
                && matches!(
                    archetype,
                    "ARACHNID"
                        | "INSECT"
                        | "CENTIPEDE"
                        | "QUADRUPED"
                        | "DRAGON"
                        | "MOUNTED"
                        | "THEROPOD"
                )
            {
                for (i, n) in body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| matches!(n.kind.as_str(), "LIMB" | "SHIN" | "FOOT"))
                {
                    let _ = n;
                    local[i] = 0.0;
                }
            }
            let mut positions = forward(body, root, &local);
            let mut fixed = vec![false; body.nodes.len()];
            for (i, n) in body.nodes.iter().enumerate() {
                fixed[i] = !n.feature
                    || matches!(
                        n.kind.as_str(),
                        "FOOT"
                            | "HAND"
                            | "WING"
                            | "TAIL"
                            | "TENTACLE"
                            | "VORTEX"
                            | "ROOT"
                            | "LOBE"
                            | "ROTOR"
                            | "STINGER"
                    );
            }
            // Independent feet have explicit support and swing phases. The rider
            // inherits the saddle; its feet do not reach through the mount.
            for (i, n) in body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.kind == "FOOT" && !is_rider(&n.id))
            {
                let c = chain(body, i, "HIP");
                if c.is_empty() {
                    continue;
                }
                let mut target = (n.x, n.y);
                if moving && speed > 0.0 && !mode.contains("ANCHORED") && pose.state != "CLIMB" {
                    let impaired = mode.contains("LIMP") && ordinal(&n.id) % 2 == 1;
                    let duty = if impaired {
                        0.8
                    } else if fast {
                        0.5
                    } else {
                        0.65
                    };
                    let stride = speed * duration * duty;
                    let lift = if impaired {
                        1.0
                    } else if fast {
                        4.2
                    } else {
                        2.8
                    } * scale;
                    let foot = foot_cycle(
                        p + phase_offset(archetype, mode, ordinal(&n.id)),
                        duty,
                        stride,
                        lift,
                    );
                    target.0 += foot.0;
                    target.1 += foot.1;
                    if mode.contains("HOP") {
                        target.1 += root.1;
                    }
                }
                if planted {
                    solve_chain(body, &mut positions, &c, target, &mut fixed);
                } else if pose.state == "CLIMB" {
                    let cyc = foot_cycle(
                        p + phase_offset(archetype, mode, ordinal(&n.id)),
                        0.6,
                        5.0 * scale,
                        1.8 * scale,
                    );
                    solve_chain(
                        body,
                        &mut positions,
                        &c,
                        (n.x + cyc.1, n.y + cyc.0),
                        &mut fixed,
                    );
                }
            }
            if pose.state == "DEATH"
                && matches!(
                    archetype,
                    "ARACHNID"
                        | "INSECT"
                        | "CENTIPEDE"
                        | "QUADRUPED"
                        | "DRAGON"
                        | "MOUNTED"
                        | "THEROPOD"
                )
            {
                let collapse = ramp(p, 0.12, 0.75);
                // Loss of leg support folds distal joints toward their hips.
                // Solve a compact endpoint instead of letting a dangling shin
                // become the lowest point and lift the whole corpse off the floor.
                for (i, n) in body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.kind == "FOOT" && !is_rider(&n.id))
                {
                    let c = chain(body, i, "HIP");
                    if c.len() < 3 {
                        continue;
                    }
                    let lengths: Vec<f32> = c
                        .windows(2)
                        .map(|w| {
                            (body.nodes[w[0]].x - body.nodes[w[1]].x)
                                .hypot(body.nodes[w[0]].y - body.nodes[w[1]].y)
                        })
                        .collect();
                    let total = lengths.iter().sum::<f32>();
                    let longest = lengths.iter().copied().reduce(f32::max).unwrap();
                    let folded_reach = (2.0 * longest - total).max(0.0) + 0.8 * scale;
                    let hip = positions[c[0]];
                    let rest_hip = &body.nodes[c[0]];
                    let start = (n.x + root.0, n.y + root.1);
                    let target = (
                        mix(start.0, hip.0 + (n.x - rest_hip.x) * 0.18, collapse),
                        mix(start.1, hip.1 + folded_reach, collapse),
                    );
                    solve_chain(body, &mut positions, &c, target, &mut fixed);
                }
            }
            if let (Some(attack), Some(a)) = (attack, &a) {
                let origin = body
                    .nodes
                    .iter()
                    .position(|n| n.id == attack.origin_node)
                    .unwrap_or(0);
                let mut i = Some(origin);
                let mut hand = None;
                while let Some(j) = i {
                    if body.nodes[j].kind == "HAND" {
                        hand = Some(j);
                        break;
                    }
                    i = body.nodes[j].parent;
                }
                if let Some(hand) = hand {
                    let c = chain(body, hand, "SHOULDER");
                    if let Some(&shoulder) = c.first() {
                        let rest = positions[hand];
                        let anchor = positions[shoulder];
                        let tx = rest.0
                            + (anchor.0 - 2.0 * scale - rest.0) * a.wind
                            + (anchor.0 + 10.0 * scale - rest.0) * a.hit;
                        let ty = rest.1
                            + (anchor.1 - 8.0 * scale - rest.1) * a.wind
                            + (anchor.1 - 2.0 * scale - rest.1) * a.hit;
                        solve_chain(body, &mut positions, &c, (tx, ty), &mut fixed);
                        positions[hand].2 = body.nodes[hand].angle - 1.0 * a.wind + 0.45 * a.hit;
                    }
                }
                // The last key returns to the supported neutral pose, including
                // delayed secondary motion, instead of snapping on clip exit.
                if a.recovery >= 1.0 {
                    positions = forward(body, (0.0, 0.0, 0.0), &vec![0.0; body.nodes.len()]);
                }
            }
            if pose.state == "DEATH" {
                // Relax the wrist so a held weapon settles flat with the body,
                // rather than acting as a vertical prop through the ground.
                for n in body.nodes.iter().filter(|n| n.kind == "WEAPON") {
                    if let Some(parent) = n.parent.filter(|&i| body.nodes[i].kind == "HAND") {
                        positions[parent].2 = mix(
                            positions[parent].2,
                            body.nodes[parent].angle - n.angle,
                            ramp(p, 0.25, 0.8),
                        );
                    }
                }
            }
            reattach(body, &mut positions, &fixed);
            if pose.state == "DEATH" && !matches!(archetype, "FLOATING" | "WHIRLWIND") {
                // Resolve the entire connected corpse against the floor, rather
                // than clamping individual ragdoll nodes (which tears joints).
                let lowest = positions
                    .iter()
                    .enumerate()
                    .map(|(i, &p)| bottom(body, i, p))
                    .reduce(f32::max)
                    .unwrap();
                let correction = (floor - lowest) * ramp(p, 0.0, 0.5);
                for pos in &mut positions {
                    pos.1 += correction;
                }
            }
            pose.root_x = root.0;
            pose.root_y = root.1;
            pose.root_angle = root.2;
            pose.physical_positions = positions;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    fn build(prompt: &str) -> (Body, Vec<Animation>, Vec<Pose>) {
        let spec = crate::parser::parse_vocabulary(prompt);
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(31));
        let (clips, _, _, poses) = crate::animation::make(&spec, &body);
        (body, clips, poses)
    }
    #[test]
    fn one_shots_finish_and_loops_do_not_duplicate_first_frame() {
        let (_, clips, poses) = build("dragon");
        for clip in clips {
            let last = &poses[clip.frames.last().unwrap().sprite_frame_id as usize];
            assert_eq!(last.phase == 1.0, !clip.looped, "{}", clip.id);
        }
    }
    #[test]
    fn every_pose_is_finite_and_attachments_remain_connected() {
        for prompt in [
            "orc with a sword",
            "wolf",
            "spider",
            "dragon",
            "cobra",
            "slug",
            "ghost",
            "sand worm",
            "mushroom",
            "robot drone",
        ] {
            let (body, _, poses) = build(prompt);
            for p in poses {
                assert_eq!(p.physical_positions.len(), body.nodes.len());
                for (i, n) in body.nodes.iter().enumerate() {
                    let q = p.physical_positions[i];
                    assert!(
                        q.0.is_finite() && q.1.is_finite() && q.2.is_finite(),
                        "{prompt} {}",
                        p.clip_id
                    );
                    if n.feature && !matches!(n.kind.as_str(), "FOOT" | "HAND") {
                        if let Some(parent) = n.parent {
                            let expected =
                                (n.x - body.nodes[parent].x).hypot(n.y - body.nodes[parent].y);
                            assert!(
                                (distance(q, p.physical_positions[parent]) - expected).abs() < 0.01,
                                "{prompt} {} {} detached",
                                p.clip_id,
                                n.id
                            );
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn grounded_legs_keep_bone_lengths() {
        for prompt in ["wolf", "orc", "spider", "dragon"] {
            let (body, _, poses) = build(prompt);
            for p in poses
                .iter()
                .filter(|p| matches!(p.state.as_str(), "MOVE" | "FAST_MOVE" | "IDLE"))
            {
                for (i, n) in body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| matches!(n.kind.as_str(), "LIMB" | "SHIN" | "FOOT"))
                {
                    if let Some(parent) = n.parent {
                        let length = (n.x - body.nodes[parent].x).hypot(n.y - body.nodes[parent].y);
                        assert!(
                            (distance(p.physical_positions[i], p.physical_positions[parent])
                                - length)
                                .abs()
                                < 0.02,
                            "{prompt} {} {} stretched",
                            p.clip_id,
                            n.id
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn death_settles_into_a_held_grounded_pose() {
        for prompt in ["orc", "wolf", "spider", "cobra", "slug", "dragon"] {
            let (body, _, poses) = build(prompt);
            let death: Vec<_> = poses.iter().filter(|p| p.state == "DEATH").collect();
            let last = death.last().unwrap();
            let previous = death[death.len() - 2];
            for (a, b) in last
                .physical_positions
                .iter()
                .zip(&previous.physical_positions)
            {
                assert!(distance(*a, *b) < 0.01, "{prompt} corpse is still moving");
            }
            let lowest = last
                .physical_positions
                .iter()
                .enumerate()
                .map(|(i, &p)| bottom(&body, i, p))
                .reduce(f32::max)
                .unwrap();
            assert!(
                (lowest - ground(&body)).abs() < 0.01,
                "{prompt} misses ground"
            );
            assert!(
                distance(last.physical_positions[0], death[0].physical_positions[0]) > 1.0,
                "{prompt} did not collapse"
            );
        }
    }
    #[test]
    fn stance_feet_remain_planted_at_the_exported_root_speed() {
        for seed in [4, 31, 42] {
            for prompt in ["orc", "wolf", "spider", "hornet", "centipede", "dragon"] {
                let spec = crate::parser::parse_vocabulary(prompt);
                let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(seed));
                let (clips, modes, _, poses) = crate::animation::make(&spec, &body);
                for mode in modes
                    .iter()
                    .filter(|m| matches!(m.animation_id.as_str(), "move" | "fast_move"))
                {
                    let clip = clips.iter().find(|c| c.id == mode.animation_id).unwrap();
                    let duty = if clip.semantic_state == "FAST_MOVE" {
                        0.5
                    } else {
                        0.65
                    };
                    for pair in clip.frames.windows(2) {
                        let a = &poses[pair[0].sprite_frame_id as usize];
                        let b = &poses[pair[1].sprite_frame_id as usize];
                        for (i, n) in body
                            .nodes
                            .iter()
                            .enumerate()
                            .filter(|(_, n)| n.kind == "FOOT")
                        {
                            let offset =
                                phase_offset(&a.motion_archetype, &mode.id, ordinal(&n.id));
                            let start = (a.phase + offset).rem_euclid(1.0);
                            let end = (b.phase + offset).rem_euclid(1.0);
                            if end < start || start >= duty || end >= duty {
                                continue;
                            }
                            let pa = a.physical_positions[i];
                            let pb = b.physical_positions[i];
                            assert!(
                                (pa.1 - n.y).abs() < 0.12 && (pb.1 - n.y).abs() < 0.12,
                                "{prompt}/{seed} {} lost contact",
                                n.id
                            );
                            let travel = mode.speed * pair[0].duration_ms as f32 / 1000.0;
                            assert!(
                                ((pb.0 - pa.0) + travel).abs() < 0.12,
                                "{prompt}/{seed} {} slides during {}",
                                n.id,
                                mode.id
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn nested_riders_never_emit_substrate_foot_contacts() {
        let spec = crate::parser::parse_vocabulary("pig riding a pig riding a pig");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        assert!(body
            .nodes
            .iter()
            .any(|n| n.id.starts_with("mount_rider_foot")));
        let (clips, _, _, _) = crate::animation::make(&spec, &body);
        for clip in clips
            .iter()
            .filter(|c| matches!(c.id.as_str(), "move" | "fast_move"))
        {
            let contacts: Vec<_> = clip
                .events
                .iter()
                .filter(|e| e.kind == "FOOT_CONTACT")
                .collect();
            assert_eq!(contacts.len(), 4);
            assert!(contacts.iter().all(|e| !is_rider(&e.reference_id)));
        }
    }

    #[test]
    fn quadruped_death_loses_leg_support_and_lowers_the_trunk() {
        for prompt in ["wolf", "bear", "dragon", "rhino"] {
            let (_, _, poses) = build(prompt);
            let dead: Vec<_> = poses.iter().filter(|p| p.state == "DEATH").collect();
            assert!(
                dead.last().unwrap().physical_positions[0].1
                    > dead[0].physical_positions[0].1 + 0.5,
                "{prompt} still stands on its legs"
            );
        }
    }

    #[test]
    fn arthropod_death_lowers_the_body_and_folds_feet() {
        for prompt in ["spider", "hornet", "centipede", "scorpion"] {
            let (body, _, poses) = build(prompt);
            let death: Vec<_> = poses.iter().filter(|p| p.state == "DEATH").collect();
            let start = death[0];
            let end = death.last().unwrap();
            assert!(
                end.physical_positions[0].1 > start.physical_positions[0].1 + 0.4,
                "{prompt} rose instead of collapsing"
            );
            for (i, n) in body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.kind == "FOOT")
            {
                let c = chain(&body, i, "HIP");
                let hip = c[0];
                let before = start.physical_positions[i].1 - start.physical_positions[hip].1;
                let after = end.physical_positions[i].1 - end.physical_positions[hip].1;
                assert!(after < before - 1.0, "{prompt} {} did not fold up", n.id);
            }
        }
    }

    #[test]
    fn wings_flap_relative_to_their_parent_even_when_marked_features() {
        let spec = crate::parser::parse_vocabulary("dragon");
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(31));
        for n in body.nodes.iter_mut().filter(|n| n.kind == "WING") {
            n.feature = true;
        }
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        for (i, n) in body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "WING")
        {
            let parent = n.parent.unwrap();
            for state in ["FLY", "TAKEOFF"] {
                let values: Vec<_> = poses
                    .iter()
                    .filter(|p| p.state == state)
                    .map(|p| {
                        p.physical_positions[i].2
                            - n.angle
                            - (p.physical_positions[parent].2 - body.nodes[parent].angle)
                    })
                    .collect();
                let min = values.iter().copied().reduce(f32::min).unwrap();
                let max = values.iter().copied().reduce(f32::max).unwrap();
                assert!(max - min > 0.8, "{} did not flap in {state}", n.id);
            }
        }
    }

    #[test]
    fn paired_arthropods_alternate_contralateral_support_groups() {
        let phases: Vec<_> = (0..6)
            .map(|i| phase_offset("INSECT", "MULTILEG_SCUTTLE", i))
            .collect();
        assert_eq!(phases, [0.0, 0.5, 0.5, 0.0, 0.0, 0.5]);
        for archetype in ["INSECT", "ARACHNID"] {
            let count = if archetype == "INSECT" { 6 } else { 8 };
            for pair in 0..count / 2 {
                assert_ne!(
                    phase_offset(archetype, "MULTILEG_SCUTTLE", pair * 2),
                    phase_offset(archetype, "MULTILEG_SCUTTLE", pair * 2 + 1)
                );
            }
        }
    }

    #[test]
    fn foot_cycle_has_contact_and_a_continuous_seam() {
        assert_eq!(foot_cycle(0.3, 0.65, 10.0, 3.0).1, 0.0);
        assert!(foot_cycle(0.8, 0.65, 10.0, 3.0).1 < -2.0);
        let a = foot_cycle(0.0, 0.65, 10.0, 3.0);
        let b = foot_cycle(1.0, 0.65, 10.0, 3.0);
        assert_eq!(a, b);
    }
    #[test]
    fn attack_ends_at_rest_and_long_travel_has_recovery() {
        let (body, clips, poses) = build("dragon");
        for clip in clips.iter().filter(|c| c.semantic_state.contains("ATTACK")) {
            let last = &poses[clip.frames.last().unwrap().sprite_frame_id as usize];
            for (i, n) in body.nodes.iter().enumerate() {
                assert!(
                    distance(last.physical_positions[i], (n.x, n.y, n.angle)) < 0.01,
                    "{} {}",
                    clip.id,
                    n.id
                );
            }
            for event in &clip.events {
                assert!(
                    event.time_ms < clip.frames.iter().map(|f| f.duration_ms).sum::<u32>(),
                    "event outside {}",
                    clip.id
                );
            }
        }
    }
}
