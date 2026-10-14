//! V2 two-handed rigid gun grip. Pose coordinates are world XY (positive Y down).
use crate::{anatomy::Body, animation::Pose, parser::MonsterSpec};

type Point = (f32, f32);
fn distance(a: Point, b: Point) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
fn rotate(p: Point, angle: f32) -> Point {
    let (s, c) = angle.sin_cos();
    (p.0 * c - p.1 * s, p.0 * s + p.1 * c)
}
/// Mesh anchors from part_weapon.json, converted to the pose's Y-down plane.
pub fn anchor_offset(body: &Body, weapon: usize, angle: f32, secondary: bool) -> Point {
    let n = &body.nodes[weapon];
    let (x, y) = if secondary {
        (0.36, 0.20)
    } else {
        (-0.54, 0.40)
    };
    rotate((x * n.rx, y * n.ry * 1.2), angle)
}
fn project(p: Point, center: Point, min: f32, max: f32) -> Point {
    let d = distance(p, center);
    let r = d.clamp(min, max);
    if d < 1e-6 {
        (center.0 + r, center.1)
    } else {
        (
            center.0 + (p.0 - center.0) * r / d,
            center.1 + (p.1 - center.1) * r / d,
        )
    }
}
fn solve(shoulder: Point, target: Point, upper: f32, lower: f32, previous: Point) -> Point {
    let d = distance(shoulder, target).max(1e-6);
    let along = (upper * upper - lower * lower + d * d) / (2.0 * d);
    let height = (upper * upper - along * along).max(0.0).sqrt();
    let direction = ((target.0 - shoulder.0) / d, (target.1 - shoulder.1) / d);
    let base = (
        shoulder.0 + along * direction.0,
        shoulder.1 + along * direction.1,
    );
    let a = (base.0 - height * direction.1, base.1 + height * direction.0);
    let b = (base.0 + height * direction.1, base.1 - height * direction.0);
    if distance(a, previous) <= distance(b, previous) {
        a
    } else {
        b
    }
}

/// Establish the shared mesh grip plane before rest lengths are frozen.
pub fn prepare_body(spec: &MonsterSpec, body: &mut Body) {
    let has = |tag: &str| spec.features.iter().any(|f| f == tag);
    if !has("PROFILE_V2")
        || spec.affinity == "SHADE"
        || has("SHADE")
        || !matches!(
            spec.attack.concept.as_str(),
            "SHOTGUN" | "LASER" | "RAILGUN" | "CANNON" | "ROCKET" | "MISSILE"
        )
        || !(spec.affinity == "STORMTROOPER" || has("HYBRID_UPPER_STORMTROOPER"))
    {
        return;
    }
    let Some(weapon) = body.nodes.iter().position(|n| n.id == "weapon") else {
        return;
    };
    let Some(primary) = body.nodes[weapon].parent else {
        return;
    };
    let offset = anchor_offset(body, weapon, body.nodes[weapon].angle, false);
    let dx = body.nodes[primary].x - offset.0 - body.nodes[weapon].x;
    let dy = body.nodes[primary].y - offset.1 - body.nodes[weapon].y;
    let mut attached = vec![false; body.nodes.len()];
    for (i, node) in body.nodes.iter_mut().enumerate() {
        if i == weapon || node.parent.is_some_and(|p| attached[p]) {
            node.x += dx;
            node.y += dy;
            attached[i] = true;
        }
    }
    let plane = body
        .nodes
        .iter()
        .find(|n| n.id == "torso")
        .map(|n| n.z.unwrap_or(0.0) + n.rz.unwrap_or(n.rx * 0.57))
        .unwrap_or(0.0)
        + body.nodes[weapon].ry * 1.2 * 0.3;
    // Keep the shared grip plane outside the torso's near-side depth bound.
    // Shoulders/elbows retain their lateral separation and therefore real depth.
    let mut carried = vec![false; body.nodes.len()];
    for (i, node) in body.nodes.iter_mut().enumerate() {
        if i == primary
            || node.id == "hand_0"
            || node.id == "hand_1"
            || i == weapon
            || node.parent.is_some_and(|p| carried[p])
        {
            node.z = Some(plane);
            carried[i] = true;
        }
    }
}

/// Apply after all clip/recoil adjustments. V1 and weaponless shades are untouched.
pub fn apply(spec: &MonsterSpec, body: &Body, poses: &mut [Pose]) {
    let has = |tag: &str| spec.features.iter().any(|f| f == tag);
    if !has("PROFILE_V2")
        || spec.affinity == "SHADE"
        || has("SHADE")
        || !matches!(
            spec.attack.concept.as_str(),
            "SHOTGUN" | "LASER" | "RAILGUN" | "CANNON" | "ROCKET" | "MISSILE"
        )
        || !(spec.affinity == "STORMTROOPER" || has("HYBRID_UPPER_STORMTROOPER"))
    {
        return;
    }
    let find = |id: &str| body.nodes.iter().position(|n| n.id == id);
    let Some(weapon) = find("weapon") else {
        return;
    };
    let Some(primary) = body.nodes[weapon].parent else {
        return;
    };
    let secondary = if body.nodes[primary].id == "hand_1" {
        find("hand_0")
    } else {
        find("hand_1")
    };
    let Some(secondary) = secondary else {
        return;
    };
    let mut chains = Vec::new();
    for hand in [primary, secondary] {
        let Some(elbow) = body.nodes[hand].parent else {
            return;
        };
        let Some(shoulder) = body.nodes[elbow].parent else {
            return;
        };
        let point = |i: usize| (body.nodes[i].x, body.nodes[i].y);
        chains.push((
            shoulder,
            elbow,
            hand,
            distance(point(shoulder), point(elbow)),
            distance(point(elbow), point(hand)),
        ));
    }
    for pose in poses {
        if pose.physical_positions.len() != body.nodes.len() {
            continue;
        }
        let old = pose.physical_positions.clone();
        let angle = old[weapon].2;
        let offsets = [
            anchor_offset(body, weapon, angle, false),
            anchor_offset(body, weapon, angle, true),
        ];
        let mut center = (old[weapon].0, old[weapon].1);
        // Project the rigid weapon into both arms' reachable annuli. This moves
        // the gun, never stretches a bone, and retains recoil when reachable.
        for _ in 0..128 {
            let before = center;
            for (j, &(s, _, _, a, b)) in chains.iter().enumerate() {
                center = project(
                    center,
                    (old[s].0 - offsets[j].0, old[s].1 - offsets[j].1),
                    (a - b).abs() + 1e-4,
                    a + b - 1e-4,
                );
            }
            if distance(before, center) < 1e-6 {
                break;
            }
        }
        // Extreme custom weapon dimensions can make a shared grip impossible.
        // Never fabricate a reachable solution by stretching anatomical bones.
        if chains.iter().enumerate().any(|(j, &(s, _, _, a, b))| {
            let d = distance(
                (old[s].0, old[s].1),
                (center.0 + offsets[j].0, center.1 + offsets[j].1),
            );
            d > a + b + 0.001 || d < (a - b).abs() - 0.001
        }) {
            continue;
        }
        for (j, &(s, e, h, a, b)) in chains.iter().enumerate() {
            let shoulder = (old[s].0, old[s].1);
            let target = (center.0 + offsets[j].0, center.1 + offsets[j].1);
            let elbow = solve(shoulder, target, a, b, (old[e].0, old[e].1));
            for (i, parent, p) in [(e, s, elbow), (h, e, target)] {
                let pp = pose.physical_positions[parent];
                let rest = &body.nodes[i];
                let rp = &body.nodes[parent];
                let delta = (p.1 - pp.1).atan2(p.0 - pp.0) - (rest.y - rp.y).atan2(rest.x - rp.x);
                pose.physical_positions[i] = (p.0, p.1, rest.angle + delta);
            }
        }
        pose.physical_positions[weapon] = (center.0, center.1, angle);
        // Carry attached decorations with their solved parent. The weapon gets
        // its own rigid transform below rather than the old hand-relative offset.
        for i in 0..body.nodes.len() {
            if i == weapon || chains.iter().any(|&(_, e, h, _, _)| i == e || i == h) {
                continue;
            }
            if let Some(p) = body.nodes[i].parent {
                if pose.physical_positions[p] != old[p] {
                    let now = pose.physical_positions[p];
                    let offset =
                        rotate((old[i].0 - old[p].0, old[i].1 - old[p].1), now.2 - old[p].2);
                    pose.physical_positions[i] = (
                        now.0 + offset.0,
                        now.1 + offset.1,
                        old[i].2 + now.2 - old[p].2,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    #[test]
    #[ignore = "writes real rendered grip inspection packages to a temporary directory"]
    fn native_visual_packages() {
        for (name, prompt) in [
            ("trooper", "stormtrooper"),
            ("mermaid", "stormtrooper top on a mermaid bottom"),
        ] {
            let path = std::path::Path::new("/tmp/infernal-grip").join(name);
            let monster = crate::generate_with_profile(
                prompt,
                42,
                &path,
                None,
                crate::PackageFormat::Cbor,
                crate::GenerationProfile::V2,
            )
            .unwrap();
            std::fs::write(
                path.join("metadata.json"),
                serde_json::to_vec_pretty(&monster).unwrap(),
            )
            .unwrap();
        }
    }

    #[test]
    fn legacy_and_shade_are_untouched() {
        for prompt in ["stormtrooper", "shade stormtrooper top on a mermaid bottom"] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            if prompt.starts_with("shade") {
                spec = crate::v2::interpret(spec, prompt);
                crate::v2::prepare(&mut spec);
            }
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
            let rest = format!("{body:?}");
            prepare_body(&spec, &mut body);
            assert_eq!(rest, format!("{body:?}"));
            let (_, _, _, mut poses) = crate::animation::make(&spec, &body);
            let before = format!("{poses:?}");
            apply(&spec, &body, &mut poses);
            assert_eq!(before, format!("{poses:?}"));
        }
    }

    #[test]
    fn both_hands_contact_rigid_grips_without_stretching_in_every_pose() {
        for prompt in [
            "stormtrooper",
            "stormtrooper top on a mermaid bottom",
            "stormtrooper top on a spider bottom",
        ] {
            let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            crate::v2::prepare(&mut spec);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
            crate::v2::anatomy::refine(&spec, &mut body, 42);
            prepare_body(&spec, &mut body);
            let (clips, _, _, poses) = crate::v2::combat::animate(&spec, &body, 42);
            crate::v2::combat::validate_rig_extensions(&body, &clips, &poses).unwrap();
            let weapon = body
                .nodes
                .iter()
                .position(|n| n.id == "weapon")
                .expect(prompt);
            assert!(body.nodes[weapon].z.is_some_and(|z| z > 0.0));
            let idle = poses
                .iter()
                .find(|p| p.clip_id == "idle")
                .unwrap()
                .physical_positions[weapon];
            assert!(
                poses
                    .iter()
                    .filter(|p| p.clip_id == "attack_primary")
                    .any(|p| distance(
                        (
                            p.physical_positions[weapon].0,
                            p.physical_positions[weapon].1
                        ),
                        (idle.0, idle.1)
                    ) > 0.1),
                "recoil must move the rigid gun"
            );
            for pose in &poses {
                let w = pose.physical_positions[weapon];
                for (id, secondary) in [("hand_1", false), ("hand_0", true)] {
                    let h = body.nodes.iter().position(|n| n.id == id).unwrap();
                    let offset = anchor_offset(&body, weapon, w.2, secondary);
                    let hp = pose.physical_positions[h];
                    assert!(
                        distance((hp.0, hp.1), (w.0 + offset.0, w.1 + offset.1)) < 0.001,
                        "{prompt} {} {id}",
                        pose.clip_id
                    );
                    let e = body.nodes[h].parent.unwrap();
                    for i in [h, e] {
                        let p = body.nodes[i].parent.unwrap();
                        let n = &body.nodes[i];
                        let pn = &body.nodes[p];
                        let ip = pose.physical_positions[i];
                        let pp = pose.physical_positions[p];
                        assert!(
                            (distance((ip.0, ip.1), (pp.0, pp.1))
                                - distance((n.x, n.y), (pn.x, pn.y)))
                            .abs()
                                < 0.005,
                            "{prompt} {} {}",
                            pose.clip_id,
                            n.id
                        );
                    }
                }
            }
        }
    }
}
