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

pub const ARM_EXTENSION_EVENT: &str = "V2_ARM_EXTENSION_MAX_1_8";
pub const AWKWARD_EVENT: &str = "V2_AWKWARD_COORDINATION_MAX_0_18_RAD";

fn in_subtree(body: &Body, child: usize, root: usize) -> bool {
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

fn transform_subtree(body: &Body, pose: &mut Pose, root: usize, angle: f32, scale: f32) {
    let pivot = pose.physical_positions[root];
    let (sin, cos) = angle.sin_cos();
    for i in 0..body.nodes.len() {
        if in_subtree(body, i, root) {
            let p = &mut pose.physical_positions[i];
            let (x, y) = (p.0 - pivot.0, p.1 - pivot.1);
            p.0 = pivot.0 + scale * (cos * x - sin * y);
            p.1 = pivot.1 + scale * (sin * x + cos * y);
            p.2 += angle;
        }
    }
}

fn pulse(time: f32, peak: f32, end: f32) -> f32 {
    let x = if time <= peak {
        time / peak
    } else {
        (end - time) / (end - peak)
    };
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn expressive_attacks(
    spec: &MonsterSpec,
    body: &Body,
    clips: &mut Vec<Animation>,
    attacks: &mut Vec<Attack>,
    poses: &mut Vec<Pose>,
) {
    // Lower-body identity drives propulsion and caudal attacks. Upper tree arms
    // remain capable of their own smash on another compatible substrate.
    let mut kinds = Vec::new();
    if matches!(spec.affinity.as_str(), "MERMAID" | "MERMAN")
        || enabled(spec, "HYBRID_LOWER_MERMAID")
    {
        kinds.extend(["mermaid_lunge", "mermaid_tail_whip"]);
    }
    if (spec.affinity == "TREE_MONSTER" || enabled(spec, "HYBRID_UPPER_TREE"))
        && body.nodes.iter().filter(|n| n.kind == "SHOULDER").count() >= 2
    {
        kinds.push("tree_bilateral_smash");
    }
    for kind in &kinds {
        let id = format!("v2_{kind}");
        let clip_id = format!("attack_{id}");
        let mut neutral = poses[0].clone();
        neutral.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
        let mut clip = visual_clip(&clip_id, "ATTACK", 24, 50, false, &[neutral], poses);
        let tree = *kind == "tree_bilateral_smash";
        let lunge = *kind == "mermaid_lunge";
        let origin = if tree {
            body.nodes
                .iter()
                .position(|n| n.id == "v2_upper_torso")
                .or_else(|| body.nodes.iter().position(|n| n.id == "torso"))
        } else if lunge {
            body.nodes.iter().position(|n| n.id == "torso")
        } else {
            // Extra mutant tails must not steal the primary caudal hit origin.
            body.nodes
                .iter()
                .rposition(|n| n.id.starts_with("v2_fish_tail_"))
                .or_else(|| {
                    body.nodes
                        .iter()
                        .rposition(|n| n.kind == "TAIL" && !n.id.contains("extra"))
                })
                .or_else(|| body.nodes.iter().rposition(|n| n.kind == "TAIL"))
        }
        .unwrap_or(0);
        let mut steps = vec![AttackStep {
            primitive: "TELEGRAPH".into(),
            value: 1.0,
            ..Default::default()
        }];
        if lunge {
            steps.push(AttackStep {
                primitive: "DASH".into(),
                at_ms: 450,
                target: "TARGET_ACTOR".into(),
                speed: 80.0,
                duration_ms: 200,
                value: 1.0,
                ..Default::default()
            });
        }
        steps.push(AttackStep {
            primitive: if tree { "SLAM" } else { "SWIPE" }.into(),
            at_ms: 650,
            target: "TARGET_ACTOR".into(),
            count: if tree { 2 } else { 1 },
            radius: if tree { 72.0 } else { 52.0 },
            duration_ms: 100,
            value: 1.0,
            ..Default::default()
        });
        steps.push(AttackStep {
            primitive: "COOLDOWN".into(),
            at_ms: 1000,
            value: 1.0,
            ..Default::default()
        });
        let attack = Attack {
            id: id.clone(),
            delivery: "MELEE".into(),
            element: "PHYSICAL".into(),
            origin_node: body.nodes[origin].id.clone(),
            damage: if tree { 28 } else { 19 },
            telegraph_ms: 450,
            cooldown_ms: if tree { 1800 } else { 1400 },
            range: if tree { 90.0 } else { 76.0 },
            animation_id: clip_id.clone(),
            steps,
            ..Default::default()
        };
        for (time_ms, kind) in [(0, "TELEGRAPH"), (450, "ATTACK"), (650, "ATTACK_HIT")] {
            clip.events.push(AnimationEvent {
                time_ms,
                kind: kind.into(),
                reference_id: id.clone(),
            });
        }
        if tree {
            clip.events.push(AnimationEvent {
                time_ms: 0,
                kind: ARM_EXTENSION_EVENT.into(),
                reference_id: id.clone(),
            });
        }
        for (frame_index, frame) in clip.frames.iter().enumerate() {
            let pose = &mut poses[frame.sprite_frame_id as usize];
            let time = frame_index as f32 * 50.0;
            let drive = pulse(time, 650.0, 1150.0);
            if tree {
                // Both shoulders share a common strike envelope; no unilateral replacement.
                for (i, node) in body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.kind == "SHOULDER")
                {
                    let side = if node.x < body.com.0 { -1.0 } else { 1.0 };
                    let windup = pulse(time, 350.0, 650.0);
                    let angle = -side * 1.4 * windup + side * 0.6 * drive;
                    transform_subtree(body, pose, i, angle, 1.0 + 0.8 * drive);
                }
            } else {
                // The torso participates in every strike, with a connected tail coil/whip.
                let tails: Vec<_> = body
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.kind == "TAIL")
                    .map(|(i, _)| i)
                    .collect();
                for (rank, i) in tails.into_iter().enumerate() {
                    let coil = if lunge {
                        -0.22 * drive
                    } else {
                        (if rank % 2 == 0 { 0.32 } else { 0.18 }) * drive
                    };
                    transform_subtree(body, pose, i, coil, 1.0);
                }
                let angle = if lunge { -0.65 * drive } else { 0.72 * drive };
                let (sin, cos) = angle.sin_cos();
                let root = pose.physical_positions[0];
                for p in &mut pose.physical_positions {
                    let (x, y) = (p.0 - root.0, p.1 - root.1);
                    p.0 =
                        root.0 + x * cos - y * sin + if lunge { 16.0 * drive } else { 4.0 * drive };
                    p.1 = root.1 + x * sin + y * cos - 5.0 * drive;
                    p.2 += angle;
                }
                pose.root_x = if lunge { 16.0 * drive } else { 4.0 * drive };
                pose.root_y = -5.0 * drive;
                pose.root_angle = angle;
            }
        }
        clips.push(clip);
        attacks.push(attack);
    }
}

fn awkward_coordination(
    body: &Body,
    seed: u64,
    clips: &mut [Animation],
    attacks: &mut [Attack],
    poses: &mut [Pose],
) {
    for (index, attack) in attacks.iter_mut().enumerate() {
        let delay = 90 + ((seed.wrapping_add(index as u64)) % 3) as u32 * 30;
        let Some(clip) = clips.iter_mut().find(|c| c.id == attack.animation_id) else {
            continue;
        };
        attack.telegraph_ms += delay;
        attack.cooldown_ms += delay + 180;
        for step in &mut attack.steps {
            if step.primitive != "TELEGRAPH" {
                step.at_ms += delay;
            }
        }
        for event in &mut clip.events {
            if event.time_ms != 0 {
                event.time_ms += delay;
            }
        }
        if let Some(first) = clip.frames.first().cloned() {
            let mut hold = first;
            hold.duration_ms = delay;
            clip.frames.insert(0, hold);
        }
        if let Some(last) = clip.frames.last_mut() {
            last.duration_ms += 180;
        }
        clip.events.push(AnimationEvent {
            time_ms: 0,
            kind: AWKWARD_EVENT.into(),
            reference_id: attack.id.clone(),
        });
        // Shared reverse frames must receive a transform only once.
        let mut visited = std::collections::BTreeSet::new();
        // Opposite arms lag differently, but hit times and damage remain reliable.
        for frame in &clip.frames {
            if !visited.insert(frame.sprite_frame_id) {
                continue;
            }
            let pose = &mut poses[frame.sprite_frame_id as usize];
            if pose.physical_positions.len() != body.nodes.len() {
                continue;
            }
            for (rank, (i, _)) in body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.kind == "SHOULDER")
                .enumerate()
            {
                let angle = 0.18
                    * (pose.phase * std::f32::consts::TAU + rank as f32 * 1.3).sin()
                    * (std::f32::consts::PI * pose.phase).sin();
                transform_subtree(body, pose, i, angle, 1.0);
            }
        }
    }
}

pub const UNFOLD_EVENT: &str = "V2_FOLDED_EXTRA_ARMS_UNFOLD";

fn unfold_extra_arms(body: &Body, clips: &mut [Animation], attacks: &[Attack], poses: &mut [Pose]) {
    let extra: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == "SHOULDER")
        .skip(2)
        .map(|(i, _)| i)
        .collect();
    if extra.is_empty() {
        return;
    }
    for pose in poses {
        if pose.physical_positions.len() != body.nodes.len() {
            continue;
        }
        let open =
            if ["ATTACK", "ATTACK_SECONDARY", "SPECIAL_ATTACK"].contains(&pose.state.as_str()) {
                pulse(pose.phase, 0.55, 1.0)
            } else {
                0.0
            };
        for &i in &extra {
            let side = if body.nodes[i].x < body.com.0 {
                -1.0
            } else {
                1.0
            };
            transform_subtree(body, pose, i, side * 0.9 * (1.0 - open), 1.0);
        }
    }
    for attack in attacks {
        if let Some(clip) = clips.iter_mut().find(|c| c.id == attack.animation_id) {
            clip.events.push(AnimationEvent {
                time_ms: 0,
                kind: UNFOLD_EVENT.into(),
                reference_id: attack.id.clone(),
            });
        }
    }
}

/// Keep extra tails distinguishable in baked side views. Rotate their whole
/// connected subtree about the existing torso socket; never stretch a bone.
fn splay_extra_tails(body: &Body, poses: &mut [Pose]) {
    let roots: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| {
            n.id.starts_with("v2_extra_tail_")
                && n.parent
                    .is_some_and(|p| !body.nodes[p].id.starts_with("v2_extra_tail_"))
        })
        .map(|(i, _)| i)
        .collect();
    for pose in poses {
        if pose.physical_positions.len() != body.nodes.len() || pose.state == "DEATH" {
            continue;
        }
        let angle = 0.3 + 0.1 * (pose.phase * std::f32::consts::TAU + 0.6).sin();
        let (sin, cos) = angle.sin_cos();
        for &root in &roots {
            let pivot = pose.physical_positions[body.nodes[root].parent.unwrap()];
            for i in 0..body.nodes.len() {
                if in_subtree(body, i, root) {
                    let p = &mut pose.physical_positions[i];
                    let (x, y) = (p.0 - pivot.0, p.1 - pivot.1);
                    p.0 = pivot.0 + x * cos - y * sin;
                    p.1 = pivot.1 + x * sin + y * cos;
                    p.2 += angle;
                }
            }
        }
    }
}

/// Explicit growth is restricted to tree arm descendants in one marked clip.
/// Every other edge in these expressive clips retains its anatomical length.
pub fn validate_rig_extensions(
    body: &Body,
    clips: &[Animation],
    poses: &[Pose],
) -> Result<(), String> {
    for clip in clips.iter().filter(|c| {
        c.id.starts_with("attack_v2_mermaid_")
            || c.id.starts_with("attack_v2_spectral_")
            || c.id == "attack_v2_tree_bilateral_smash"
            || c.id.starts_with("attack_v2_shade_tendril_")
    }) {
        let extension = clip.id == "attack_v2_tree_bilateral_smash";
        let shade = clip.id.starts_with("attack_v2_shade_tendril_");
        if shade
            && (!clip.events.iter().any(|e| {
                e.kind == SHADE_EXTENSION_EVENT && clip.id == format!("attack_{}", e.reference_id)
            }) || body
                .nodes
                .iter()
                .filter(|n| n.id.starts_with("v2_shade_tendril_"))
                .count()
                != 15)
        {
            return Err("shade growth requires exact tagged tendril topology and contract".into());
        }
        if shade {
            for chain in 0..3 {
                let mut parent = 0;
                for segment in 0..5 {
                    let index = body
                        .nodes
                        .iter()
                        .position(|n| n.id == format!("v2_shade_tendril_{chain}_{segment}"))
                        .ok_or("missing shade segment")?;
                    if body.nodes[index].kind != "TENTACLE"
                        || body.nodes[index].parent != Some(parent)
                    {
                        return Err("shade segment has invalid anatomical parent".into());
                    }
                    parent = index;
                }
            }
        }
        let marked = clip
            .events
            .iter()
            .any(|e| e.kind == ARM_EXTENSION_EVENT && e.reference_id == "v2_tree_bilateral_smash");
        if marked != extension {
            return Err("missing or unexpected bounded arm-extension contract".into());
        }
        let shoulders: Vec<_> = body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "SHOULDER")
            .map(|(i, _)| i)
            .collect();
        if extension && shoulders.len() < 2 {
            return Err("bilateral smash requires two arms".into());
        }
        let mut extended = vec![false; shoulders.len()];
        for frame in &clip.frames {
            let pose = poses
                .get(frame.sprite_frame_id as usize)
                .ok_or("missing expressive pose")?;
            if pose.physical_positions.len() != body.nodes.len() {
                return Err("incomplete expressive rig".into());
            }
            let root = pose.physical_positions[0];
            let (max_x, max_y) = if extension || shade {
                (0.03, 0.03)
            } else {
                (16.03, 5.03)
            };
            if (root.0 - body.nodes[0].x).abs() > max_x || (root.1 - body.nodes[0].y).abs() > max_y
            {
                return Err("expressive root motion exceeds declared bounds".into());
            }
            for (i, node) in body.nodes.iter().enumerate() {
                let p = pose.physical_positions[i];
                if !p.0.is_finite() || !p.1.is_finite() || !p.2.is_finite() {
                    return Err("nonfinite expressive pose".into());
                }
                if let Some(parent) = node.parent {
                    let q = pose.physical_positions[parent];
                    let rest = (node.x - body.nodes[parent].x).hypot(node.y - body.nodes[parent].y);
                    let distance = (p.0 - q.0).hypot(p.1 - q.1);
                    let arm = shoulders
                        .iter()
                        .position(|&s| i != s && in_subtree(body, i, s));
                    let max = if (extension && arm.is_some())
                        || (shade && node.id.starts_with("v2_shade_tendril_"))
                    {
                        1.8
                    } else {
                        1.0
                    };
                    if distance < rest - 0.03 || distance > rest * max + 0.03 {
                        return Err("expressive rig exceeds declared bone-length bounds".into());
                    }
                    if extension && rest > 0.1 && distance > rest * 1.4 {
                        if let Some(a) = arm {
                            extended[a] = true;
                        }
                    }
                }
            }
        }
        if shade {
            let hit = clip
                .events
                .iter()
                .find(|e| e.kind == "ATTACK_HIT")
                .ok_or("shade has no hit frame")?
                .time_ms;
            let lead = if clip.id.ends_with("thrust") { 1 } else { 0 };
            let tip = body
                .nodes
                .iter()
                .position(|n| n.id == format!("v2_shade_tendril_{lead}_4"))
                .ok_or("missing shade hit tip")?;
            let parent = body.nodes[tip].parent.unwrap();
            let rest = (body.nodes[tip].x - body.nodes[parent].x)
                .hypot(body.nodes[tip].y - body.nodes[parent].y);
            let mut time = 0u32;
            let mut active = false;
            for frame in &clip.frames {
                let end = time
                    .checked_add(frame.duration_ms)
                    .ok_or("shade duration overflow")?;
                if time <= hit && hit < end {
                    let pose = &poses[frame.sprite_frame_id as usize];
                    let p = pose.physical_positions[tip];
                    let q = pose.physical_positions[parent];
                    active = (p.0 - q.0).hypot(p.1 - q.1) >= rest * 1.7;
                }
                time = end;
            }
            if !active {
                return Err("shade hit event lacks its extended tendril pose".into());
            }
        }
        if extension && extended.iter().any(|v| !*v) {
            return Err("bilateral smash must visibly extend every arm".into());
        }
    }
    Ok(())
}

pub const PHASE_BEGIN_EVENT: &str = "PHASE_BEGIN";
pub const PHASE_END_EVENT: &str = "PHASE_END";
pub const SPECTRAL_LUNGE_ID: &str = "v2_spectral_phase_lunge";

fn spectral_attacks(
    spec: &MonsterSpec,
    body: &Body,
    clips: &mut Vec<Animation>,
    modes: &mut Vec<MovementMode>,
    attacks: &mut Vec<Attack>,
    poses: &mut Vec<Pose>,
) {
    if enabled(spec, "SHADE") || spec.affinity == "SHADE" {
        return;
    }
    if !["GHOST", "GHOUL"]
        .iter()
        .any(|tag| enabled(spec, tag) || spec.affinity == *tag)
    {
        return;
    }
    let mut neutral = poses[0].clone();
    neutral.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
    let origin = body
        .nodes
        .iter()
        .find(|n| n.id == "torso")
        .unwrap_or(&body.nodes[0])
        .id
        .clone();
    let id = SPECTRAL_LUNGE_ID;
    let mut lunge = visual_clip(
        "attack_v2_spectral_phase_lunge",
        "ATTACK",
        24,
        50,
        false,
        &[neutral.clone()],
        poses,
    );
    for (time, kind) in [
        (0, "TELEGRAPH"),
        (150, PHASE_BEGIN_EVENT),
        (450, "ATTACK"),
        (550, PHASE_END_EVENT),
        (650, "ATTACK_HIT"),
    ] {
        lunge.events.push(AnimationEvent {
            time_ms: time,
            kind: kind.into(),
            reference_id: id.into(),
        });
    }
    for (i, frame) in lunge.frames.iter().enumerate() {
        let pose = &mut poses[frame.sprite_frame_id as usize];
        let time = i as f32 * 50.0;
        if (150.0..550.0).contains(&time) {
            pose.state = "PHASE".into();
        }
        let drive = if time < 550.0 {
            0.0
        } else {
            pulse(time - 550.0, 100.0, 600.0)
        };
        let hover = -3.0 * (std::f32::consts::PI * pose.phase).sin();
        for p in &mut pose.physical_positions {
            p.0 += 10.0 * drive;
            p.1 += hover;
        }
        pose.root_x = 10.0 * drive;
        pose.root_y = hover;
    }
    attacks.push(Attack {
        id: id.into(),
        delivery: "MELEE".into(),
        element: "SHADOW".into(),
        origin_node: origin.clone(),
        damage: 17,
        telegraph_ms: 450,
        cooldown_ms: 1700,
        range: 60.0,
        animation_id: lunge.id.clone(),
        steps: vec![
            AttackStep {
                primitive: "TELEGRAPH".into(),
                value: 1.0,
                ..Default::default()
            },
            AttackStep {
                primitive: "DASH".into(),
                at_ms: 550,
                value: 1.0,
                target: "TARGET_ACTOR".into(),
                speed: 100.0,
                duration_ms: 100,
                ..Default::default()
            },
            AttackStep {
                primitive: "SWIPE".into(),
                at_ms: 650,
                value: 1.0,
                target: "TARGET_ACTOR".into(),
                count: 1,
                radius: 36.0,
                duration_ms: 100,
                ..Default::default()
            },
            AttackStep {
                primitive: "COOLDOWN".into(),
                at_ms: 1000,
                value: 1.0,
                ..Default::default()
            },
        ],
        ..Default::default()
    });
    clips.push(lunge);
    let id = "v2_pattern_spectral_burst";
    let mut burst = visual_clip(
        "attack_v2_spectral_burst",
        "ATTACK",
        30,
        30,
        false,
        &[neutral.clone()],
        poses,
    );
    for (i, frame) in burst.frames.iter().enumerate() {
        let pose = &mut poses[frame.sprite_frame_id as usize];
        let charge = pulse(i as f32 * 30.0, 420.0, 870.0);
        for (j, node) in body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "SHOULDER")
        {
            let side = if node.x < body.com.0 { -1.0 } else { 1.0 };
            transform_subtree(body, pose, j, -side * 0.8 * charge, 1.0);
        }
        let recoil = if i == 14 || i == 19 { -2.0 } else { 0.0 };
        for p in &mut pose.physical_positions {
            p.0 += recoil;
            p.1 -= 3.0 * charge;
        }
        pose.root_x = recoil;
        pose.root_y = -3.0 * charge;
    }
    burst.events.push(AnimationEvent {
        time_ms: 0,
        kind: "TELEGRAPH".into(),
        reference_id: id.into(),
    });
    burst.events.push(AnimationEvent {
        time_ms: 420,
        kind: "ATTACK".into(),
        reference_id: id.into(),
    });
    let mut steps = vec![AttackStep {
        primitive: "TELEGRAPH".into(),
        value: 1.0,
        ..Default::default()
    }];
    for time in [420, 570] {
        steps.push(AttackStep {
            primitive: "SPAWN_PROJECTILE".into(),
            at_ms: time,
            value: 1.0,
            target: "TARGET_ACTOR".into(),
            count: 3,
            spread_deg: 24.0,
            speed: 120.0,
            radius: 4.0,
            duration_ms: 1200,
        });
        burst.events.push(AnimationEvent {
            time_ms: time,
            kind: "PROJECTILE_SPAWN".into(),
            reference_id: id.into(),
        });
        poses[burst.frames[(time / 30) as usize].sprite_frame_id as usize].phase = 0.5;
    }
    steps.push(AttackStep {
        primitive: "COOLDOWN".into(),
        at_ms: 810,
        value: 1.0,
        ..Default::default()
    });
    attacks.push(Attack {
        id: id.into(),
        delivery: "PROJECTILE".into(),
        element: "SHADOW".into(),
        origin_node: origin,
        damage: 5,
        telegraph_ms: 420,
        cooldown_ms: 1800,
        range: 144.0,
        animation_id: burst.id.clone(),
        steps,
        ..Default::default()
    });
    clips.push(burst);
    let mut drift = visual_clip(
        "v2_spectral_drift",
        "SPECTRAL_DRIFT",
        12,
        100,
        true,
        &[neutral],
        poses,
    );
    drift.movement_mode = "SPECTRAL_DRIFT".into();
    for frame in &mut drift.frames {
        frame.root_dx = 2.0;
        let pose = &mut poses[frame.sprite_frame_id as usize];
        let hover = -2.5 * (pose.phase * std::f32::consts::TAU).sin();
        for p in &mut pose.physical_positions {
            p.1 += hover;
        }
        pose.root_y = hover;
    }
    modes.push(MovementMode {
        id: "SPECTRAL_DRIFT".into(),
        speed: 20.0,
        animation_id: drift.id.clone(),
        category: "AERIAL".into(),
        modifiers: vec!["LEVITATING".into(), "SPECTRAL".into()],
    });
    clips.push(drift);
}

pub const SHADE_EXTENSION_EVENT: &str = "V2_SHADE_TENDRIL_EXTENSION_MAX_1_8";

fn shade_attacks(
    spec: &MonsterSpec,
    body: &Body,
    seed: u64,
    clips: &mut Vec<Animation>,
    attacks: &mut Vec<Attack>,
    poses: &mut Vec<Pose>,
) {
    if !enabled(spec, "SHADE") && spec.affinity != "SHADE" {
        return;
    }
    // Offensive clips are replaced as a unit: a shade never falls back to the
    // gun/bite/caudal attacks associated with its original silhouette.
    let replaced: std::collections::BTreeSet<_> =
        attacks.iter().map(|a| a.animation_id.clone()).collect();
    clips.retain(|c| !replaced.contains(&c.id));
    attacks.clear();
    let chains: Vec<Vec<usize>> = (0..3)
        .map(|chain| {
            (0..5)
                .filter_map(|segment| {
                    body.nodes
                        .iter()
                        .position(|n| n.id == format!("v2_shade_tendril_{chain}_{segment}"))
                })
                .collect()
        })
        .collect();
    if chains.iter().any(|chain| chain.len() != 5) {
        return;
    }
    for (lead, name, primitive) in [(0, "lash", "SWIPE"), (1, "thrust", "THRUST")] {
        let id = format!("v2_shade_tendril_{name}");
        let clip_id = format!("attack_{id}");
        let mut neutral = poses[0].clone();
        neutral.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
        let mut clip = visual_clip(&clip_id, "ATTACK", 14, 50, false, &[neutral], poses);
        for (time, kind) in [
            (0, "TELEGRAPH"),
            (0, SHADE_EXTENSION_EVENT),
            (150, PHASE_BEGIN_EVENT),
            (450, "ATTACK"),
            (1350, PHASE_END_EVENT),
            (650, "ATTACK_HIT"),
        ] {
            clip.events.push(AnimationEvent {
                time_ms: time,
                kind: kind.into(),
                reference_id: id.clone(),
            });
        }
        let phase = (seed % 65521) as f32 / 65521.0 * std::f32::consts::TAU;
        for (frame_index, frame) in clip.frames.iter().enumerate() {
            let pose = &mut poses[frame.sprite_frame_id as usize];
            let time = frame_index as f32 * 50.0;
            let drive = pulse(time, 650.0, 1350.0);
            let noise_envelope = (std::f32::consts::PI * time / 1350.0).sin().max(0.0);
            pose.state = "SHADE_UNWIND".into();
            pose.phase = frame_index as f32 / 13.0;
            for (chain_index, chain) in chains.iter().enumerate() {
                for (segment, &i) in chain.iter().enumerate() {
                    let node = &body.nodes[i];
                    let parent = node.parent.expect("shade tendril has an anatomical parent");
                    let anchor = pose.physical_positions[parent];
                    let rest = (node.x - body.nodes[parent].x, node.y - body.nodes[parent].y);
                    let length = rest.0.hypot(rest.1);
                    let base = rest.1.atan2(rest.0);
                    let coherent_noise =
                        ((time / 260.0 + phase + segment as f32 * 0.85 + chain_index as f32 * 1.7)
                            .sin()
                            * 0.32
                            + (time / 470.0 + phase * 0.7 + segment as f32 * 1.3).sin() * 0.16)
                            * noise_envelope;
                    let aimed = if name == "lash" {
                        ((time - 650.0) / 210.0).sin() * 0.55
                    } else {
                        0.0
                    };
                    // A traveling S-curve survives full extension, avoiding a
                    // straight barrel silhouette at the actual impact frame.
                    let curl = (segment as f32 * 1.35 - time / 430.0 + phase).sin() * 0.8 * drive;
                    let angle = if chain_index == lead {
                        base * (1.0 - drive) + aimed * drive + coherent_noise + curl
                    } else {
                        let fan = if (chain_index + 3 - lead) % 3 == 1 {
                            -0.8
                        } else {
                            0.8
                        };
                        base * (1.0 - 0.9 * drive)
                            + fan * 0.9 * drive
                            + coherent_noise
                            + curl * 0.65
                    };
                    let scale = 1.0
                        + if chain_index == lead {
                            0.8 * drive
                        } else {
                            0.55 * drive
                        };
                    pose.physical_positions[i] = (
                        anchor.0 + length * scale * angle.cos(),
                        anchor.1 + length * scale * angle.sin(),
                        angle,
                    );
                }
            }
        }
        // Recovery is literally the same baked images in reverse, not a second
        // noisy evaluation. The contact image holds through the sole hit window.
        clip.frames.last_mut().unwrap().duration_ms = 100;
        let recovery: Vec<_> = clip.frames[..13].iter().rev().cloned().collect();
        clip.frames.extend(recovery);
        let chain = &chains[lead];
        let reach: f32 = chain
            .iter()
            .map(|&i| {
                let n = &body.nodes[i];
                let p = &body.nodes[n.parent.unwrap()];
                (n.x - p.x).hypot(n.y - p.y)
            })
            .sum();
        attacks.push(Attack {
            id,
            delivery: "MELEE".into(),
            element: "SHADOW".into(),
            origin_node: body.nodes[*chain.last().unwrap()].id.clone(),
            damage: if name == "lash" { 18 } else { 23 },
            telegraph_ms: 450,
            cooldown_ms: if name == "lash" { 1500 } else { 1800 },
            range: reach * 1.8 + 20.0,
            animation_id: clip_id,
            steps: vec![
                AttackStep {
                    primitive: "TELEGRAPH".into(),
                    value: 1.0,
                    ..Default::default()
                },
                AttackStep {
                    primitive: primitive.into(),
                    at_ms: 650,
                    value: 1.0,
                    target: "TARGET_ACTOR".into(),
                    count: 1,
                    radius: 20.0,
                    duration_ms: 100,
                    ..Default::default()
                },
                AttackStep {
                    primitive: "COOLDOWN".into(),
                    at_ms: 1150,
                    value: 1.0,
                    ..Default::default()
                },
            ],
            ..Default::default()
        });
        clips.push(clip);
    }
    let mut remap = std::collections::BTreeMap::new();
    let mut used = Vec::new();
    for clip in clips {
        for frame in &mut clip.frames {
            let old = frame.sprite_frame_id;
            let new = *remap.entry(old).or_insert_with(|| {
                let id = used.len() as u32;
                used.push(poses[old as usize].clone());
                id
            });
            frame.sprite_frame_id = new;
        }
    }
    *poses = used;
}

/// Speed and clip cadence change together; per-cycle root travel stays fixed.
fn ninja_movement(spec: &MonsterSpec, clips: &mut [Animation], modes: &mut [MovementMode]) {
    if !enabled(spec, "NINJA") {
        return;
    }
    let moving: std::collections::BTreeSet<_> = modes
        .iter()
        .filter(|m| m.speed > 0.0)
        .map(|m| m.animation_id.clone())
        .collect();
    let mut factors = std::collections::BTreeMap::new();
    for clip in clips.iter_mut().filter(|c| moving.contains(&c.id)) {
        let old: u32 = clip.frames.iter().map(|f| f.duration_ms).sum();
        for frame in &mut clip.frames {
            frame.duration_ms = ((frame.duration_ms as f32 / 1.55).round() as u32).max(1);
        }
        let now: u32 = clip.frames.iter().map(|f| f.duration_ms).sum();
        if old == 0 || now == 0 {
            continue;
        }
        for event in &mut clip.events {
            event.time_ms = (event.time_ms as f32 * now as f32 / old as f32).round() as u32;
        }
        factors.insert(clip.id.clone(), old as f32 / now as f32);
    }
    for mode in modes {
        if let Some(factor) = factors.get(&mode.animation_id) {
            mode.speed *= factor;
            if !mode.modifiers.iter().any(|m| m == "NINJA_AGILITY") {
                mode.modifiers.push("NINJA_AGILITY".into());
            }
        }
    }
}

/// A dance is a coordinated rigid-joint loop, not random node jitter. Hands,
/// props, facial details and planted appendages inherit their actual parents.
fn dance_pose(body: &Body, pose: &mut Pose) {
    let phase = pose.phase * std::f32::consts::TAU;
    let beat = (phase * 2.0).sin();
    let hip_count = body.nodes.iter().filter(|n| n.kind == "HIP").count();
    pose.root_x = if hip_count >= 4 { 2.0 } else { 3.0 } * phase.sin();
    pose.root_y = -if hip_count >= 4 { 1.25 } else { 2.1 } * beat.abs();
    pose.root_angle = if hip_count == 0 {
        0.24
    } else if hip_count >= 4 {
        0.10
    } else {
        0.13
    } * phase.sin();
    let shoulders: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == "SHOULDER")
        .map(|(i, _)| i)
        .collect();
    for (rank, shoulder) in shoulders.iter().enumerate() {
        // Alternate full overhead reaches with a bent-elbow chest accent.
        // A half-cycle offset makes the left/right silhouettes clearly unequal.
        let accent = (phase + rank as f32 * std::f32::consts::PI).sin().max(0.0);
        transform_subtree(body, pose, *shoulder, -0.35 - 2.0 * accent, 1.0);
        if let Some(elbow) = body
            .nodes
            .iter()
            .position(|n| n.parent == Some(*shoulder) && n.kind == "ARM")
        {
            transform_subtree(body, pose, elbow, 0.40 - 0.85 * accent, 1.0);
        }
    }
    let hips: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == "HIP")
        .map(|(i, _)| i)
        .collect();
    for (rank, hip) in hips.iter().enumerate() {
        let offset = if rank % 2 == 0 {
            0.0
        } else {
            std::f32::consts::PI
        };
        let angle = if hip_count >= 4 { 0.43 } else { 0.38 } * (phase * 2.0 + offset).sin();
        transform_subtree(body, pose, *hip, angle, 1.0);
        if hip_count >= 4 {
            if let Some(upper) = body
                .nodes
                .iter()
                .position(|n| n.parent == Some(*hip) && n.kind == "LIMB")
            {
                transform_subtree(
                    body,
                    pose,
                    upper,
                    -0.40 * (phase * 2.0 + offset).sin().max(0.0),
                    1.0,
                );
            }
        }
    }
    for (i, node) in body.nodes.iter().enumerate() {
        if node.kind == "NECK" {
            transform_subtree(body, pose, i, 0.30 * (phase * 2.0 + 0.4).sin(), 1.0);
        } else if ["TAIL", "BODY_SEGMENT", "TENTACLE", "BRANCH", "ROOT"]
            .contains(&node.kind.as_str())
        {
            let amplitude = if shoulders.is_empty() && hips.is_empty() {
                0.17
            } else {
                0.07
            };
            transform_subtree(
                body,
                pose,
                i,
                amplitude * (phase * 2.0 + i as f32 * 0.45).sin(),
                1.0,
            );
        } else if node.kind == "WING" {
            transform_subtree(body, pose, i, 0.16 * (phase * 2.0 + i as f32).sin(), 1.0);
        }
    }
}

fn dancer_behavior(
    spec: &MonsterSpec,
    body: &Body,
    clips: &mut Vec<Animation>,
    modes: &mut Vec<MovementMode>,
    attacks: &mut Vec<Attack>,
    poses: &mut Vec<Pose>,
) {
    if !enabled(spec, "DANCER") {
        return;
    }
    let action_clips: std::collections::BTreeSet<_> =
        attacks.iter().map(|a| a.animation_id.clone()).collect();
    clips.retain(|c| !action_clips.contains(&c.id));
    modes.retain(|m| clips.iter().any(|c| c.id == m.animation_id));
    let neutral = poses[0].clone();
    for clip in clips.iter_mut().filter(|c| c.looped || c.id == LURE_CLIP) {
        let travel_x: f32 = clip.frames.iter().map(|f| f.root_dx).sum();
        let travel_y: f32 = clip.frames.iter().map(|f| f.root_dy).sum();
        clip.frames.clear();
        clip.events.clear();
        for frame in 0..16 {
            let mut pose = neutral.clone();
            pose.clip_id = clip.id.clone();
            pose.state = clip.semantic_state.clone();
            pose.phase = frame as f32 / 16.0;
            pose.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
            dance_pose(body, &mut pose);
            clip.frames.push(AnimationFrame {
                sprite_frame_id: poses.len() as u32,
                duration_ms: 80,
                root_dx: travel_x / 16.0,
                root_dy: travel_y / 16.0,
            });
            poses.push(pose);
        }
        for time_ms in [0, 320, 640, 960] {
            clip.events.push(AnimationEvent {
                time_ms,
                kind: "DANCE_BEAT".into(),
                reference_id: "dance".into(),
            });
        }
        // The groove has its own fixed cadence. Retain each mode's original
        // stride, but derive speed from the rewritten clip rather than keeping
        // its old cadence's speed. Ninja retiming then scales both consistently.
        let duration_ms: u32 = clip.frames.iter().map(|f| f.duration_ms).sum();
        let distance: f32 = clip.frames.iter().map(|f| f.root_dx.hypot(f.root_dy)).sum();
        for mode in modes.iter_mut().filter(|m| m.animation_id == clip.id) {
            mode.speed = distance * 1000.0 / duration_ms.max(1) as f32;
        }
    }
    for mode in modes.iter_mut() {
        if !mode.modifiers.iter().any(|m| m == "DANCING") {
            mode.modifiers.push("DANCING".into());
        }
    }
    let origin = body
        .nodes
        .iter()
        .find(|n| n.kind == "TORSO")
        .unwrap_or(&body.nodes[0]);
    let idle = clips
        .iter()
        .find(|c| c.id == "idle")
        .expect("base anatomy has an idle loop");
    let duration = idle.frames.iter().map(|f| f.duration_ms).sum();
    attacks.clear();
    attacks.push(Attack {
        id: "v2_passive_contact".into(),
        delivery: "CONTACT".into(),
        element: "PHYSICAL".into(),
        origin_node: origin.id.clone(),
        damage: 6,
        telegraph_ms: 0,
        cooldown_ms: 500,
        range: origin.rx.min(origin.ry).max(1.0),
        animation_id: "idle".into(),
        steps: vec![AttackStep {
            primitive: "CONTACT".into(),
            at_ms: 0,
            value: 1.0,
            target: "OVERLAPPING_ACTOR".into(),
            count: 0,
            radius: origin.rx.min(origin.ry).max(1.0),
            duration_ms: duration,
            ..Default::default()
        }],
        ..Default::default()
    });
    // Removed attack clips must not leave unused fighting frames in the atlas.
    let mut remap = std::collections::BTreeMap::new();
    let mut kept = Vec::new();
    for clip in clips {
        for frame in &mut clip.frames {
            let old = frame.sprite_frame_id;
            frame.sprite_frame_id = *remap.entry(old).or_insert_with(|| {
                let new = kept.len() as u32;
                kept.push(poses[old as usize].clone());
                new
            });
        }
    }
    *poses = kept;
}

pub fn animate(
    spec: &MonsterSpec,
    body: &Body,
    seed: u64,
) -> (Vec<Animation>, Vec<MovementMode>, Vec<Attack>, Vec<Pose>) {
    let (mut clips, mut modes, mut attacks, mut poses) = animation::make(spec, body);
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
    expressive_attacks(spec, body, &mut clips, &mut attacks, &mut poses);
    spectral_attacks(spec, body, &mut clips, &mut modes, &mut attacks, &mut poses);
    shade_attacks(spec, body, seed, &mut clips, &mut attacks, &mut poses);
    super::spear_throw::apply(spec, body, seed, &mut clips, &mut attacks, &mut poses);
    if enabled(spec, "MUTANT_UNSTABLE") {
        awkward_coordination(body, seed, &mut clips, &mut attacks, &mut poses);
    }
    if enabled(spec, "FOUR_ARMED") {
        unfold_extra_arms(body, &mut clips, &attacks, &mut poses);
    }
    if enabled(spec, "EXTRA_TAIL") {
        splay_extra_tails(body, &mut poses);
    }
    super::grip::apply(spec, body, &mut poses);
    dancer_behavior(spec, body, &mut clips, &mut modes, &mut attacks, &mut poses);
    ninja_movement(spec, &mut clips, &mut modes);
    (clips, modes, attacks, poses)
}

/// Guards the V2 extension without imposing new rules on legacy attacks.
pub fn validate_patterns(animations: &[Animation], attacks: &[Attack]) -> Result<(), String> {
    for contact in attacks
        .iter()
        .filter(|a| a.id == "v2_passive_contact" || a.delivery == "CONTACT")
    {
        let clip = animations
            .iter()
            .find(|c| c.id == contact.animation_id)
            .ok_or("missing passive-contact dance loop")?;
        if contact.id != "v2_passive_contact"
            || contact.delivery != "CONTACT"
            || contact.element != "PHYSICAL"
            || contact.telegraph_ms != 0
            || contact.cooldown_ms < 300
            || contact.steps.len() != 1
            || contact.steps[0].primitive != "CONTACT"
            || contact.steps[0].target != "OVERLAPPING_ACTOR"
            || contact.steps[0].at_ms != 0
            || contact.steps[0].count != 0
            || contact.steps[0].speed != 0.0
            || contact.spawn.is_some()
            || !contact.projectile_id.is_empty()
            || !clip.looped
            || clip.events.iter().any(|e| {
                [
                    "ATTACK",
                    "ATTACK_HIT",
                    "TELEGRAPH",
                    "PROJECTILE_SPAWN",
                    "FIELD_SPAWN",
                    "MINION_SPAWN",
                ]
                .contains(&e.kind.as_str())
            })
        {
            return Err("invalid passive-contact dance contract".into());
        }
    }

    for clip in animations {
        let begin: Vec<_> = clip
            .events
            .iter()
            .filter(|e| e.kind == PHASE_BEGIN_EVENT)
            .collect();
        let end: Vec<_> = clip
            .events
            .iter()
            .filter(|e| e.kind == PHASE_END_EVENT)
            .collect();
        let phase_attack = attacks.iter().find(|a| {
            (a.id == SPECTRAL_LUNGE_ID || a.id.starts_with("v2_shade_tendril_"))
                && a.animation_id == clip.id
        });
        if !begin.is_empty() || !end.is_empty() || phase_attack.is_some() {
            let attack = phase_attack.ok_or("phase event belongs to unknown attack")?;
            let shade = attack.id.starts_with("v2_shade_tendril_");
            if begin.len() != 1
                || end.len() != 1
                || begin[0].reference_id != attack.id
                || end[0].reference_id != attack.id
                || attack.telegraph_ms.checked_sub(300) != Some(begin[0].time_ms)
                || attack
                    .telegraph_ms
                    .checked_add(if shade { 900 } else { 100 })
                    != Some(end[0].time_ms)
                || begin[0].time_ms >= end[0].time_ms
                || end[0].time_ms - begin[0].time_ms != if shade { 1200 } else { 400 }
            {
                return Err("invalid bounded phase-event contract".into());
            }
            let duration = clip
                .frames
                .iter()
                .try_fold(0u32, |n, f| n.checked_add(f.duration_ms))
                .ok_or("phase clip duration overflow")?;
            let shade = attack.id.starts_with("v2_shade_tendril_");
            let hit_kind = if attack.id == "v2_shade_tendril_thrust" {
                "THRUST"
            } else {
                "SWIPE"
            };
            if end[0].time_ms >= duration
                || attack.delivery != "MELEE"
                || attack.element != "SHADOW"
                || attack.steps.len() != if shade { 3 } else { 4 }
                || attack
                    .steps
                    .iter()
                    .filter(|s| {
                        s.primitive == hit_kind
                            && s.at_ms == attack.telegraph_ms + 200
                            && s.count == 1
                    })
                    .count()
                    != 1
                || !clip.events.iter().any(|e| {
                    e.kind == "ATTACK_HIT"
                        && e.reference_id == attack.id
                        && e.time_ms == attack.telegraph_ms + 200
                })
                || (!shade
                    && attack
                        .steps
                        .iter()
                        .filter(|s| {
                            s.primitive == "DASH"
                                && s.at_ms == end[0].time_ms
                                && s.speed == 100.0
                                && s.duration_ms == 100
                        })
                        .count()
                        != 1)
            {
                return Err("phase attack must reappear before its bounded hit".into());
            }
            if shade {
                let delay = attack
                    .telegraph_ms
                    .checked_sub(450)
                    .ok_or("shade telegraph precedes its unwind")?;
                let awkward = clip.events.iter().any(|e| e.kind == AWKWARD_EVENT);
                let offset = usize::from(awkward);
                if (awkward && ![90, 120, 150].contains(&delay))
                    || (!awkward && delay != 0)
                    || clip.frames.len() != 27 + offset
                {
                    return Err("shade requires a bounded forward/reverse frame sequence".into());
                }
                let frames = &clip.frames[offset..];
                if awkward
                    && (clip.frames[0].duration_ms != delay
                        || clip.frames[0].sprite_frame_id != frames[0].sprite_frame_id)
                {
                    return Err("shade mutant windup must hold the rest frame".into());
                }
                let unique: std::collections::BTreeSet<_> =
                    frames[..14].iter().map(|f| f.sprite_frame_id).collect();
                if unique.len() != 14
                    || frames.iter().enumerate().any(|(i, frame)| {
                        let duration = if i == 13 {
                            100
                        } else if i == 26 && awkward {
                            230
                        } else {
                            50
                        };
                        frame.duration_ms != duration
                            || frame.sprite_frame_id != frames[26 - i].sprite_frame_id
                    })
                {
                    return Err(
                        "shade recovery must reuse the exact outward images in reverse".into(),
                    );
                }
                let tip = match attack.id.as_str() {
                    "v2_shade_tendril_lash" => "v2_shade_tendril_0_4",
                    "v2_shade_tendril_thrust" => "v2_shade_tendril_1_4",
                    _ => return Err("unknown shade tendril recipe".into()),
                };
                if attack.origin_node != tip
                    || attack
                        .steps
                        .iter()
                        .filter(|s| s.primitive == "TELEGRAPH" && s.at_ms == 0)
                        .count()
                        != 1
                    || attack
                        .steps
                        .iter()
                        .filter(|s| {
                            s.primitive == "COOLDOWN" && s.at_ms == attack.telegraph_ms + 700
                        })
                        .count()
                        != 1
                    || attack
                        .steps
                        .iter()
                        .filter(|s| {
                            s.primitive == hit_kind
                                && s.radius == 20.0
                                && s.duration_ms == 100
                                && s.target == "TARGET_ACTOR"
                        })
                        .count()
                        != 1
                    || attack.steps.iter().any(|s| {
                        !["TELEGRAPH", "COOLDOWN", hit_kind].contains(&s.primitive.as_str())
                    })
                    || !clip
                        .events
                        .iter()
                        .any(|e| e.kind == SHADE_EXTENSION_EVENT && e.reference_id == attack.id)
                {
                    return Err("shade must attack through its declared tendril tip".into());
                }
            }
        }
    }
    for clip in animations {
        let mut combat_events = std::collections::BTreeSet::new();
        for event in clip.events.iter().filter(|e| {
            [
                "TELEGRAPH",
                "ATTACK",
                "ATTACK_HIT",
                "PROJECTILE_SPAWN",
                "FIELD_SPAWN",
                "MINION_SPAWN",
            ]
            .contains(&e.kind.as_str())
        }) {
            let attack = attacks
                .iter()
                .find(|a| a.id == event.reference_id && a.animation_id == clip.id)
                .ok_or("dangling combat event")?;
            if !combat_events.insert((
                event.kind.as_str(),
                event.time_ms,
                event.reference_id.as_str(),
            )) {
                return Err("duplicate combat event".into());
            }
            let bound = match event.kind.as_str() {
                "TELEGRAPH" => event.time_ms == 0,
                "ATTACK" => event.time_ms == attack.telegraph_ms,
                "PROJECTILE_SPAWN" => {
                    attack.delivery == "PROJECTILE"
                        && attack
                            .steps
                            .iter()
                            .any(|s| s.primitive == "SPAWN_PROJECTILE" && s.at_ms == event.time_ms)
                }
                "FIELD_SPAWN" => {
                    attack.delivery == "FIELD"
                        && attack
                            .steps
                            .iter()
                            .any(|s| s.primitive == "SPAWN_FIELD" && s.at_ms == event.time_ms)
                }
                "MINION_SPAWN" => {
                    attack.delivery == "SUMMON"
                        && attack
                            .steps
                            .iter()
                            .any(|s| s.primitive == "SPAWN_MINION" && s.at_ms == event.time_ms)
                }
                _ => attack.steps.iter().any(|s| {
                    ["SWIPE", "BITE", "THRUST", "SLAM"].contains(&s.primitive.as_str())
                        && s.at_ms == event.time_ms
                }),
            };
            if !bound {
                return Err(format!(
                    "combat event disagrees with executable attack step: {} {} at {} for {}",
                    clip.id, event.kind, event.time_ms, attack.id
                ));
            }
        }
        let mut modifiers = std::collections::BTreeSet::new();
        if clip.events.iter().any(|e| {
            e.kind.starts_with("V2_")
                && ![
                    ARM_EXTENSION_EVENT,
                    AWKWARD_EVENT,
                    UNFOLD_EVENT,
                    SHADE_EXTENSION_EVENT,
                ]
                .contains(&e.kind.as_str())
        }) {
            return Err("unknown bounded animation modifier".into());
        }
        for event in clip.events.iter().filter(|e| {
            [
                ARM_EXTENSION_EVENT,
                AWKWARD_EVENT,
                UNFOLD_EVENT,
                SHADE_EXTENSION_EVENT,
            ]
            .contains(&e.kind.as_str())
        }) {
            if !modifiers.insert(event.kind.as_str())
                || event.time_ms != 0
                || !["ATTACK", "ATTACK_SECONDARY", "SPECIAL_ATTACK"]
                    .contains(&clip.semantic_state.as_str())
                || !attacks
                    .iter()
                    .any(|a| a.id == event.reference_id && a.animation_id == clip.id)
                || (event.kind == SHADE_EXTENSION_EVENT
                    && (!event.reference_id.starts_with("v2_shade_tendril_")
                        || clip.id != format!("attack_{}", event.reference_id)))
                || (event.kind == ARM_EXTENSION_EVENT
                    && (event.reference_id != "v2_tree_bilateral_smash"
                        || clip.id != "attack_v2_tree_bilateral_smash"))
            {
                return Err("invalid bounded animation modifier linkage".into());
            }
        }
    }
    for attack in attacks.iter().filter(|a| {
        [
            "v2_mermaid_lunge",
            "v2_mermaid_tail_whip",
            "v2_tree_bilateral_smash",
        ]
        .contains(&a.id.as_str())
    }) {
        let clip = animations
            .iter()
            .find(|c| c.id == attack.animation_id)
            .ok_or("missing expressive attack clip")?;
        let hit = attack
            .steps
            .iter()
            .find(|s| ["SLAM", "SWIPE"].contains(&s.primitive.as_str()))
            .ok_or("missing expressive hit")?;
        let lunge = attack.id == "v2_mermaid_lunge";
        if attack.steps.len() != if lunge { 4 } else { 3 }
            || attack
                .steps
                .iter()
                .filter(|s| s.primitive == "DASH")
                .count()
                != if lunge { 1 } else { 0 }
            || attack
                .steps
                .iter()
                .filter(|s| s.primitive == "TELEGRAPH" && s.at_ms == 0)
                .count()
                != 1
            || attack
                .steps
                .iter()
                .filter(|s| s.primitive == "COOLDOWN" && s.at_ms == attack.telegraph_ms + 550)
                .count()
                != 1
        {
            return Err("expressive attack must match its bounded primitive sequence".into());
        }
        if lunge
            && !attack.steps.iter().any(|s| {
                s.primitive == "DASH"
                    && s.at_ms == attack.telegraph_ms
                    && s.duration_ms == 200
                    && s.speed == 80.0
                    && s.target == "TARGET_ACTOR"
            })
        {
            return Err("mermaid lunge requires its bounded full-body dash step".into());
        }
        if attack.delivery != "MELEE"
            || attack.element != "PHYSICAL"
            || hit.at_ms != attack.telegraph_ms + 200
            || !clip.events.iter().any(|e| {
                e.kind == "ATTACK_HIT" && e.reference_id == attack.id && e.time_ms == hit.at_ms
            })
            || (attack.id == "v2_tree_bilateral_smash"
                && (hit.count != 2
                    || hit.primitive != "SLAM"
                    || !clip
                        .events
                        .iter()
                        .any(|e| e.kind == ARM_EXTENSION_EVENT && e.reference_id == attack.id)))
        {
            return Err("invalid expressive attack timing or growth contract".into());
        }
    }
    for attack in attacks.iter().filter(|a| pattern_name(a).is_some()) {
        let name = pattern_name(attack).unwrap();
        if ![
            "aimed",
            "burst",
            "fan",
            "acid_spray",
            "fire_spray",
            "spectral_burst",
        ]
        .contains(&name)
            || attack.delivery != "PROJECTILE"
            || attack.spawn.is_some()
            || (name == "acid_spray" && attack.element != "ACID")
            || (name == "fire_spray" && attack.element != "FIRE")
            || (name == "spectral_burst" && attack.element != "SHADOW")
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
            "spectral_burst" => (2, 3, 24.0, 150),
            _ => unreachable!(),
        };
        if shots.len() != expected_volleys
            || shots.iter().any(|s| {
                s.count != expected_count
                    || s.spread_deg != expected_spread
                    || !["TARGET_ACTOR", "RANDOM_DIRECTION"].contains(&s.target.as_str())
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
    for clip in animations.iter().filter(|c| {
        c.id == "attack_v2_spectral_phase_lunge" || c.id.starts_with("attack_v2_shade_tendril_")
    }) {
        let start = clip
            .events
            .iter()
            .find(|e| e.kind == PHASE_BEGIN_EVENT)
            .ok_or("missing phase begin")?
            .time_ms;
        let end = clip
            .events
            .iter()
            .find(|e| e.kind == PHASE_END_EVENT)
            .ok_or("missing phase end")?
            .time_ms;
        let mut time = 0u32;
        for frame in &clip.frames {
            let pose = poses
                .get(frame.sprite_frame_id as usize)
                .ok_or("missing phase pose")?;
            if clip.id.starts_with("attack_v2_shade_tendril_") {
                let hit = clip
                    .events
                    .iter()
                    .find(|e| e.kind == "ATTACK_HIT")
                    .ok_or("missing shade contact")?
                    .time_ms;
                let delay = hit - 650;
                let elapsed = time.saturating_sub(delay);
                let expected = if elapsed <= 650 {
                    elapsed as f32 / 650.0
                } else {
                    (1400u32.saturating_sub(elapsed + 50)) as f32 / 650.0
                };
                if pose.state != "SHADE_UNWIND"
                    || !pose.phase.is_finite()
                    || (pose.phase - expected).abs() > 0.001
                {
                    return Err("invalid shade surface-unwind envelope".into());
                }
            } else if (pose.state == "PHASE") != (time >= start && time < end) {
                return Err("phase visuals disagree with phase interval".into());
            }
            time = time
                .checked_add(frame.duration_ms)
                .ok_or("phase duration overflow")?;
        }
    }
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
    fn expressive_fixture(prompt: &str, mutant: bool) -> (MonsterSpec, Body) {
        let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
        crate::v2::prepare(&mut spec);
        if mutant {
            spec.features
                .extend(["MUTANT_UNSTABLE".into(), "FOUR_ARMED".into()]);
            spec.arm_count = 4;
        }
        let mut body =
            crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
        crate::v2::anatomy::refine(&spec, &mut body, 42);
        crate::v2::grip::prepare_body(&spec, &mut body);
        (spec, body)
    }
    #[test]
    fn expressive_mermaid_and_bilateral_tree_attacks_are_bounded() {
        for prompt in ["mermaid", "tree monster"] {
            let (spec, body) = expressive_fixture(prompt, false);
            let (clips, _, attacks, poses) = animate(&spec, &body, 42);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            let id = if prompt == "mermaid" {
                "v2_mermaid_lunge"
            } else {
                "v2_tree_bilateral_smash"
            };
            let attack = attacks.iter().find(|a| a.id == id).unwrap();
            let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
            assert!(clip
                .events
                .iter()
                .any(|e| e.kind == "ATTACK_HIT" && e.time_ms == 650));
            if prompt == "mermaid" {
                assert!(poses[clip.frames[13].sprite_frame_id as usize].root_x >= 15.0);
            } else {
                let mut malformed = poses.clone();
                let frame = clip.frames[13].sprite_frame_id as usize;
                let hand = body.nodes.iter().position(|n| n.kind == "HAND").unwrap();
                malformed[frame].physical_positions[hand].0 += 100.0;
                assert!(validate_rig_extensions(&body, &clips, &malformed).is_err());
                let mut unmarked = clips.clone();
                unmarked
                    .iter_mut()
                    .find(|c| c.id == clip.id)
                    .unwrap()
                    .events
                    .retain(|e| e.kind != ARM_EXTENSION_EVENT);
                assert!(validate_rig_extensions(&body, &unmarked, &poses).is_err());
            }
        }
    }
    #[test]
    fn awkward_mutant_preserves_emissions_and_declares_unfolding() {
        let (mut spec, body) = expressive_fixture("mermaid", true);
        spec.attack.delivery = "PROJECTILE".into();
        spec.attack.element = "ACID".into();
        let (clips, _, attacks, poses) = animate(&spec, &body, 42);
        validate(&clips, &attacks, &poses).unwrap();
        validate_rig_extensions(&body, &clips, &poses).unwrap();
        let primary = attacks.iter().find(|a| pattern_name(a).is_some()).unwrap();
        assert!((450..=510).contains(&primary.telegraph_ms));
        let clip = clips.iter().find(|c| c.id == primary.animation_id).unwrap();
        assert!(clip.events.iter().any(|e| e.kind == AWKWARD_EVENT));
        assert!(clip.events.iter().any(|e| e.kind == UNFOLD_EVENT));
        assert_eq!(
            primary
                .steps
                .iter()
                .filter(|s| s.primitive == "SPAWN_PROJECTILE")
                .count(),
            4
        );
        let mut bad = clips.clone();
        let c = bad.iter_mut().find(|c| c.id == clip.id).unwrap();
        c.events.push(
            c.events
                .iter()
                .find(|e| e.kind == "PROJECTILE_SPAWN")
                .unwrap()
                .clone(),
        );
        assert!(validate_patterns(&bad, &attacks).is_err());
    }
    #[test]
    fn full_specialty_packages_validate() {
        for (prompt, mutant) in [
            ("mermaid", false),
            ("tree monster", false),
            ("mermaid", true),
        ] {
            let (spec, _) = expressive_fixture(prompt, mutant);
            let dir = tempfile::tempdir().unwrap();
            let monster = crate::generate_from_spec_with_profile(
                spec,
                42,
                dir.path(),
                crate::PackageFormat::Protobuf,
                crate::GenerationProfile::V2,
            )
            .unwrap();
            let expected = if prompt == "mermaid" {
                "v2_mermaid_lunge"
            } else {
                "v2_tree_bilateral_smash"
            };
            assert!(monster.attacks.iter().any(|a| a.id == expected));
            let profile = monster.v2_profile.as_ref().unwrap();
            if mutant {
                assert!(profile
                    .modifiers
                    .iter()
                    .any(|m| m == "AWKWARD_COORDINATION"));
            }
            if prompt == "tree monster" {
                assert!(profile
                    .modifiers
                    .iter()
                    .any(|m| m == "BOUNDED_ARM_EXTENSION_MAX_1_8"));
            }
        }
    }
    #[test]
    fn hybrid_lower_body_controls_modes_and_upper_origins_remain_attached() {
        use crate::v2::hybrid::{HeadKind as H, HybridRecipe, LowerBody as L, UpperBody as U};
        for (upper, lower, head, legs, category) in [
            (U::Humanoid, L::Horse, H::Humanoid, 6, "GROUNDED"),
            (U::Humanoid, L::Spider, H::Humanoid, 8, "GROUNDED"),
            (U::Humanoid, L::Humanoid, H::Fish, 2, "GROUNDED"),
            (U::Stormtrooper, L::Mermaid, H::Stormtrooper, 0, "SWIMMING"),
            (U::Tree, L::Tree, H::Tree, 2, "GROUNDED"),
        ] {
            let recipe = HybridRecipe {
                schema_version: 1,
                upper_body: upper,
                lower_body: lower,
                head,
                lower_limb_count: legs,
                upper_arm_count: 2,
            };
            let mut spec = crate::parser::parse_vocabulary("humanoid");
            crate::v2::hybrid::apply_spec(&recipe, &mut spec);
            crate::v2::prepare(&mut spec);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
            crate::v2::anatomy::refine(&spec, &mut body, 42);
            crate::v2::grip::prepare_body(&spec, &mut body);
            let (clips, modes, attacks, poses) = animate(&spec, &body, 42);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            assert_eq!(modes[0].category, category, "{recipe:?}");
            assert!(
                attacks
                    .iter()
                    .all(|a| body.nodes.iter().any(|n| n.id == a.origin_node)),
                "{recipe:?}"
            );
            if lower == L::Spider {
                assert!(modes[0].modifiers.iter().any(|m| m == "MULTILEGGED"));
            }
            if lower == L::Mermaid {
                assert!(attacks
                    .iter()
                    .any(|a| a.id == "v2_mermaid_lunge" && a.origin_node == "torso"));
                assert!(attacks.iter().any(|a| a.id == "v2_mermaid_tail_whip"
                    && a.origin_node.starts_with("v2_fish_tail_")));
                assert!(attacks
                    .iter()
                    .any(|a| a.delivery == "PROJECTILE" && a.origin_node == "weapon"));
            }
            if lower == L::Tree {
                assert!(attacks.iter().any(|a| a.id == "v2_tree_bilateral_smash"));
            }
        }
    }
    #[test]
    fn spectral_phase_burst_and_drift_are_visually_bound() {
        for prompt in ["ghost", "ghoul"] {
            let (spec, body) = expressive_fixture(prompt, false);
            let (clips, modes, attacks, poses) = animate(&spec, &body, 42);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            let attack = attacks.iter().find(|a| a.id == SPECTRAL_LUNGE_ID).unwrap();
            let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
            assert_eq!(
                clip.events
                    .iter()
                    .find(|e| e.kind == PHASE_BEGIN_EVENT)
                    .unwrap()
                    .time_ms,
                150
            );
            assert_eq!(
                clip.events
                    .iter()
                    .find(|e| e.kind == PHASE_END_EVENT)
                    .unwrap()
                    .time_ms,
                550
            );
            assert_eq!(
                poses[clip.frames[3].sprite_frame_id as usize].state,
                "PHASE"
            );
            assert_eq!(
                poses[clip.frames[11].sprite_frame_id as usize].state,
                "ATTACK"
            );
            assert!(modes
                .iter()
                .any(|m| m.id == "SPECTRAL_DRIFT" && m.speed == 20.0));
            let burst = attacks
                .iter()
                .find(|a| a.id == "v2_pattern_spectral_burst")
                .unwrap();
            assert_eq!(crate::projectile::recipe_kind(&spec, burst), "SHADOW_ORB");
            let mut bad = clips.clone();
            bad.iter_mut()
                .find(|c| c.id == clip.id)
                .unwrap()
                .events
                .retain(|e| e.kind != PHASE_END_EVENT);
            assert!(validate_patterns(&bad, &attacks).is_err());
            let mut bad = poses.clone();
            bad[clip.frames[3].sprite_frame_id as usize].state = "ATTACK".into();
            assert!(validate(&clips, &attacks, &bad).is_err());
        }
    }
    #[test]
    fn awkward_spectral_phase_window_tracks_delayed_visuals() {
        let (mut spec, body) = expressive_fixture("ghost", false);
        spec.features.push("MUTANT_UNSTABLE".into());
        let (clips, _, attacks, poses) = animate(&spec, &body, 42);
        validate(&clips, &attacks, &poses).unwrap();
        validate_rig_extensions(&body, &clips, &poses).unwrap();
        let attack = attacks.iter().find(|a| a.id == SPECTRAL_LUNGE_ID).unwrap();
        assert!(attack.telegraph_ms > 450);
        let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
        let start = clip
            .events
            .iter()
            .find(|e| e.kind == PHASE_BEGIN_EVENT)
            .unwrap()
            .time_ms;
        let end = clip
            .events
            .iter()
            .find(|e| e.kind == PHASE_END_EVENT)
            .unwrap()
            .time_ms;
        assert_eq!(end - start, 400);
        assert_eq!(end, attack.telegraph_ms + 100);
    }
    #[test]
    fn full_spectral_packages_validate_interaction_windows() {
        for prompt in ["shade", "ghost", "ghoul"] {
            let generated =
                crate::generate_in_memory_with_profile(prompt, 42, crate::GenerationProfile::V2)
                    .unwrap();
            let monster = &generated.packages.last().unwrap().monster;
            let spectral = monster
                .v2_profile
                .as_ref()
                .unwrap()
                .spectral
                .as_ref()
                .unwrap();
            assert_eq!(
                spectral.phase_windows.len(),
                if prompt == "shade" { 2 } else { 1 }
            );
            if prompt == "shade" {
                assert!(monster
                    .attacks
                    .iter()
                    .all(|a| a.id.starts_with("v2_shade_tendril_")));
            } else {
                assert!(monster
                    .attacks
                    .iter()
                    .any(|a| a.id == "v2_pattern_spectral_burst"));
            }
            assert_eq!(
                spectral.phase_windows[0].end_ms - spectral.phase_windows[0].start_ms,
                if prompt == "shade" { 1200 } else { 400 }
            );
            if prompt == "ghoul" {
                assert_eq!(spectral.opacity, 1.0);
                assert_eq!(spectral.default_damage_multiplier, 1.0);
            }
        }
    }
    #[test]
    fn universal_shade_replaces_original_offense_with_coherent_tendrils() {
        for prompt in [
            "shade humanoid",
            "shade wolf",
            "shade tree monster",
            "shade mermaid",
            "shade stormtrooper top on a mermaid bottom",
        ] {
            let (spec, body) = expressive_fixture(prompt, false);
            let (clips, _, attacks, poses) = animate(&spec, &body, 42);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            assert_eq!(attacks.len(), 2, "{prompt}");
            assert!(attacks.iter().all(|a| a.id.starts_with("v2_shade_tendril_")
                && a.delivery == "MELEE"
                && a.projectile_id.is_empty()));
            assert!(clips
                .iter()
                .all(|c| !c.events.iter().any(|e| e.kind == "PROJECTILE_SPAWN")));
            assert!(poses
                .iter()
                .all(|p| !p.clip_id.starts_with("attack_primary")));
            let attack = &attacks[0];
            let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
            let tip = body
                .nodes
                .iter()
                .position(|n| n.id == attack.origin_node)
                .unwrap();
            let start = poses[clip.frames[0].sprite_frame_id as usize].physical_positions[tip];
            let hit = poses[clip.frames[13].sprite_frame_id as usize].physical_positions[tip];
            let peak = &poses[clip.frames[13].sprite_frame_id as usize];
            let root = peak.physical_positions[0];
            let chord = (hit.0 - root.0, hit.1 - root.1);
            let mut max_curve = 0.0f32;
            for segment in 0..4 {
                let i = body
                    .nodes
                    .iter()
                    .position(|n| n.id == format!("v2_shade_tendril_0_{segment}"))
                    .unwrap();
                let p = peak.physical_positions[i];
                let distance = ((p.0 - root.0) * chord.1 - (p.1 - root.1) * chord.0).abs()
                    / chord.0.hypot(chord.1).max(0.1);
                max_curve = max_curve.max(distance);
            }
            assert!(
                max_curve > 1.0,
                "shade tendril became a straight barrel at impact: {prompt}"
            );
            let end =
                poses[clip.frames.last().unwrap().sprite_frame_id as usize].physical_positions[tip];
            assert!((hit.0 - start.0).hypot(hit.1 - start.1) > 10.0, "{prompt}");
            assert!((end.0 - start.0).hypot(end.1 - start.1) < 0.01);
            for pair in clip.frames.windows(2) {
                let a = poses[pair[0].sprite_frame_id as usize].physical_positions[tip];
                let b = poses[pair[1].sprite_frame_id as usize].physical_positions[tip];
                assert!(
                    (b.0 - a.0).hypot(b.1 - a.1) < attack.range * 0.35,
                    "discontinuous shade noise: {prompt}"
                );
            }
            let mut malformed = poses.clone();
            malformed[clip.frames[13].sprite_frame_id as usize].physical_positions[tip].0 += 100.0;
            assert!(validate_rig_extensions(&body, &clips, &malformed).is_err());
        }
    }
    #[test]
    fn shade_recovery_is_exact_reversal_and_never_damages() {
        for mutant in [false, true] {
            let (mut spec, body) = expressive_fixture("shade humanoid", false);
            if mutant {
                spec.features.push("MUTANT_UNSTABLE".into());
            }
            let (clips, _, attacks, poses) = animate(&spec, &body, 42);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            for attack in &attacks {
                let clip = clips.iter().find(|c| c.id == attack.animation_id).unwrap();
                let offset = usize::from(mutant);
                let frames = &clip.frames[offset..];
                assert_eq!(frames.len(), 27);
                assert_eq!(frames[13].duration_ms, 100);
                for i in 0..14 {
                    assert_eq!(frames[i].sprite_frame_id, frames[26 - i].sprite_frame_id);
                }
                let hit = attack.telegraph_ms + 200;
                assert!(attack
                    .steps
                    .iter()
                    .filter(|s| !["COOLDOWN", "TELEGRAPH"].contains(&s.primitive.as_str()))
                    .all(|s| s.at_ms == hit && s.duration_ms == 100));
                assert!(!clip.events.iter().any(|e| e.time_ms >= hit + 100
                    && ["ATTACK", "ATTACK_HIT", "PROJECTILE_SPAWN"].contains(&e.kind.as_str())));
                let mut malformed = clips.clone();
                let bad = malformed.iter_mut().find(|c| c.id == clip.id).unwrap();
                bad.frames[offset + 25].sprite_frame_id = bad.frames[offset + 24].sprite_frame_id;
                assert!(validate_patterns(&malformed, &attacks).is_err());
                let mut malformed = clips.clone();
                malformed
                    .iter_mut()
                    .find(|c| c.id == clip.id)
                    .unwrap()
                    .events
                    .push(AnimationEvent {
                        time_ms: hit + 150,
                        kind: "ATTACK_HIT".into(),
                        reference_id: attack.id.clone(),
                    });
                assert!(validate_patterns(&malformed, &attacks).is_err());
            }
        }
    }

    #[test]
    fn full_shade_source_families_have_only_tendril_offense() {
        for prompt in [
            "shade humanoid",
            "shade wolf",
            "shade tree monster",
            "shade mermaid",
            "shade stormtrooper top on a mermaid bottom",
        ] {
            let generated =
                crate::generate_in_memory_with_profile(prompt, 42, crate::GenerationProfile::V2)
                    .unwrap();
            let root = &generated.packages.last().unwrap().monster;
            assert_eq!(root.attacks.len(), 2, "{prompt}");
            assert!(root
                .attacks
                .iter()
                .all(|a| a.id.starts_with("v2_shade_tendril_")));
            assert!(root.projectiles.is_empty());
            assert_eq!(
                root.v2_profile
                    .as_ref()
                    .unwrap()
                    .spectral
                    .as_ref()
                    .unwrap()
                    .phase_windows
                    .len(),
                2
            );
        }
    }
    #[test]
    fn extra_mermaid_tails_splay_without_stretching_or_stealing_primary_origin() {
        for seed in [10, 42] {
            let mut spec = crate::v2::interpret(
                crate::parser::parse_vocabulary("friendly mermaid"),
                "friendly mermaid",
            );
            crate::v2::prepare(&mut spec);
            crate::v2::anatomy::prepare_seeded(&mut spec, seed);
            assert!(enabled(&spec, "EXTRA_TAIL"));
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(seed));
            crate::v2::anatomy::refine(&spec, &mut body, seed);
            let (clips, _, attacks, poses) = animate(&spec, &body, seed);
            validate(&clips, &attacks, &poses).unwrap();
            validate_rig_extensions(&body, &clips, &poses).unwrap();
            let whip = attacks
                .iter()
                .find(|a| a.id == "v2_mermaid_tail_whip")
                .unwrap();
            assert!(whip.origin_node.starts_with("v2_fish_tail_"));
            let clip = clips.iter().find(|c| c.id == whip.animation_id).unwrap();
            let mut unchanged = spec.clone();
            unchanged.features.retain(|f| f != "EXTRA_TAIL");
            let (_, _, _, baseline) = animate(&unchanged, &body, seed);
            let tip = body
                .nodes
                .iter()
                .rposition(|n| n.id.starts_with("v2_extra_tail_") && n.kind == "TAIL")
                .unwrap();
            let main = body
                .nodes
                .iter()
                .position(|n| n.id == whip.origin_node)
                .unwrap();
            let mut separated = false;
            for frame in &clip.frames {
                let pose = &poses[frame.sprite_frame_id as usize];
                let original = &baseline[frame.sprite_frame_id as usize];
                let p = pose.physical_positions[tip];
                let q = original.physical_positions[tip];
                separated |= (p.0 - q.0).hypot(p.1 - q.1) > 2.0;
                assert_eq!(
                    pose.physical_positions[main],
                    original.physical_positions[main]
                );
            }
            assert!(
                separated,
                "extra tail remains visually stacked at seed{seed}"
            );
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
    #[test]
    fn dancer_suppresses_active_attacks_and_preserves_every_anatomical_edge() {
        use rand::SeedableRng;
        for prompt in [
            "orc with shotgun",
            "wolf",
            "cobra",
            "jellyfish",
            "robot",
            "shade",
            "ghoul",
            "friendly mermaid",
        ] {
            let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            crate::v2::prepare(&mut spec);
            spec.features.push("DANCER".into());
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
            crate::v2::anatomy::refine(&spec, &mut body, 42);
            crate::v2::grip::prepare_body(&spec, &mut body);
            let (clips, _, attacks, poses) = animate(&spec, &body, 42);
            validate_patterns(&clips, &attacks).unwrap();
            assert_eq!(attacks.len(), 1, "{prompt}");
            assert_eq!(attacks[0].delivery, "CONTACT", "{prompt}");
            assert!(
                clips.iter().all(|c| c.events.iter().all(|e| ![
                    "ATTACK",
                    "ATTACK_HIT",
                    "PROJECTILE_SPAWN",
                    "FIELD_SPAWN",
                    "MINION_SPAWN"
                ]
                .contains(&e.kind.as_str()))),
                "{prompt}"
            );
            let idle = clips.iter().find(|c| c.id == "idle").unwrap();
            assert_eq!(idle.frames.len(), 16);
            assert!(
                poses[idle.frames[0].sprite_frame_id as usize].physical_positions
                    != poses[idle.frames[4].sprite_frame_id as usize].physical_positions
                    || poses[idle.frames[0].sprite_frame_id as usize].root_x
                        != poses[idle.frames[4].sprite_frame_id as usize].root_x
            );
            for frame in &idle.frames {
                let p = &poses[frame.sprite_frame_id as usize];
                for (i, n) in body.nodes.iter().enumerate() {
                    if let Some(parent) = n.parent {
                        let rest = (n.x - body.nodes[parent].x).hypot(n.y - body.nodes[parent].y);
                        let now = (p.physical_positions[i].0 - p.physical_positions[parent].0)
                            .hypot(p.physical_positions[i].1 - p.physical_positions[parent].1);
                        assert!((rest - now).abs() < 0.01, "{prompt} {}", n.id);
                    }
                }
            }
        }
    }

    #[test]
    fn dancer_movement_speed_matches_every_frame_with_and_without_ninja() {
        for prompt in ["orc", "wolf", "cobra", "jellyfish"] {
            for ninja in [false, true] {
                let mut spec =
                    crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
                crate::v2::prepare(&mut spec);
                spec.features.push("DANCER".into());
                if ninja {
                    spec.features.push("NINJA".into());
                }
                let mut body =
                    crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
                crate::v2::anatomy::refine(&spec, &mut body, 42);
                let (clips, modes, _, _) = animate(&spec, &body, 42);
                for mode in &modes {
                    let clip = clips.iter().find(|c| c.id == mode.animation_id).unwrap();
                    if !clip.looped {
                        continue;
                    }
                    for frame in &clip.frames {
                        let expected = mode.speed * frame.duration_ms as f32 / 1000.0;
                        let actual = frame.root_dx.hypot(frame.root_dy);
                        assert!(
                            (actual - expected).abs() < 0.0001,
                            "{prompt} ninja={ninja} {}: root {actual} speed-derived {expected}",
                            mode.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn ninja_speed_matches_animation_cadence_without_changing_root_stride() {
        use rand::SeedableRng;
        let mut spec = crate::parser::parse_vocabulary("humanoid");
        crate::v2::prepare(&mut spec);
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
        let (base_clips, base_modes, _, _) = animate(&spec, &body, 42);
        spec.features.push("NINJA".into());
        let (clips, modes, _, _) = animate(&spec, &body, 42);
        for old in base_modes.iter().filter(|m| m.speed > 0.0) {
            let mode = modes.iter().find(|m| m.id == old.id).unwrap();
            let a = base_clips
                .iter()
                .find(|c| c.id == old.animation_id)
                .unwrap();
            let b = clips.iter().find(|c| c.id == mode.animation_id).unwrap();
            let da: f32 = a.frames.iter().map(|f| f.duration_ms as f32).sum();
            let db: f32 = b.frames.iter().map(|f| f.duration_ms as f32).sum();
            assert!((mode.speed / old.speed - da / db).abs() < 0.0001);
            assert!(mode.speed > old.speed * 1.5);
            assert_eq!(
                a.frames.iter().map(|f| f.root_dx).sum::<f32>(),
                b.frames.iter().map(|f| f.root_dx).sum::<f32>()
            );
        }
    }
}
