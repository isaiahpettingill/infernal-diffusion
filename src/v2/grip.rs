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

fn is_gun(spec: &MonsterSpec) -> bool {
    matches!(
        spec.attack.concept.as_str(),
        "SHOTGUN" | "LASER" | "RAILGUN" | "CANNON" | "ROCKET" | "MISSILE"
    ) && spec.affinity != "SHADE"
        && !spec.features.iter().any(|f| f == "SHADE")
}

fn owner<'a>(spec: &'a MonsterSpec, id: &str) -> &'a MonsterSpec {
    let mut current = spec;
    let mut path = id;
    while let Some(rest) = path.strip_prefix("mount_") {
        let Some(child) = current.mount.as_ref().and_then(|m| m.spec.as_deref()) else {
            return current;
        };
        current = child;
        path = rest;
    }
    if path.starts_with("rider_") {
        return current;
    }
    while let Some(child) = current.mount.as_ref().and_then(|m| m.spec.as_deref()) {
        current = child;
    }
    current
}

fn arm_root(body: &Body, hand: usize) -> Option<usize> {
    if body.nodes.get(hand)?.kind != "HAND" {
        return None;
    }
    let elbow = body.nodes[hand].parent?;
    let shoulder = body.nodes.get(elbow)?.parent?;
    if !matches!(body.nodes.get(elbow)?.kind.as_str(), "ARM" | "LIMB")
        || body.nodes.get(shoulder)?.kind != "SHOULDER"
    {
        return None;
    }
    body.nodes[shoulder].parent
}

/// Resolve real anatomical hands belonging to the gun's own actor. A turret,
/// head-mounted cannon, or one-armed creature never acquires synthetic arms.
fn grips(spec: &MonsterSpec, body: &Body) -> Vec<(usize, usize, usize)> {
    if !spec.features.iter().any(|f| f == "PROFILE_V2")
        || spec.features.iter().any(|f| f == "DANCER")
    {
        return vec![];
    }
    body.nodes
        .iter()
        .enumerate()
        .filter_map(|(weapon, node)| {
            if node.kind != "WEAPON" || !is_gun(owner(spec, &node.id)) {
                return None;
            }
            let primary = node.parent?;
            let root = arm_root(body, primary)?;
            let secondary = body
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, n)| {
                    *i != primary && n.kind == "HAND" && arm_root(body, *i) == Some(root)
                })
                .min_by_key(|(_, n)| {
                    if n.id.ends_with("hand_0") || n.id.ends_with("hand_1") {
                        0
                    } else {
                        1
                    }
                })
                .map(|(i, _)| i)?;
            Some((weapon, primary, secondary))
        })
        .collect()
}

/// Establish the shared mesh grip plane before rest lengths are frozen.
pub fn prepare_body(spec: &MonsterSpec, body: &mut Body) {
    for (weapon, primary, secondary) in grips(spec, body) {
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
            .get(arm_root(body, primary).unwrap())
            .map(|n| n.z.unwrap_or(0.0) + n.rz.unwrap_or(n.rx * 0.57))
            .unwrap_or(0.0)
            + body.nodes[weapon].ry * 1.2 * 0.3;
        // Keep the shared grip plane outside the torso's near-side depth bound.
        // Shoulders/elbows retain their lateral separation and therefore real depth.
        let mut carried = vec![false; body.nodes.len()];
        for (i, node) in body.nodes.iter_mut().enumerate() {
            if i == primary
                || i == secondary
                || i == weapon
                || node.parent.is_some_and(|p| carried[p])
            {
                node.z = Some(plane);
                carried[i] = true;
            }
        }
    }
}

/// Apply after all clip/recoil adjustments. V1 and weaponless shades are untouched.
pub fn apply(spec: &MonsterSpec, body: &Body, poses: &mut [Pose]) {
    for (weapon, primary, secondary) in grips(spec, body) {
        apply_grip(body, poses, weapon, primary, secondary);
    }
}

fn apply_grip(body: &Body, poses: &mut [Pose], weapon: usize, primary: usize, secondary: usize) {
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
            "orc with laser cannon",
            "troll with shotgun",
            "humanoid with railgun",
            "robot with cannon",
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
    #[test]
    fn mounted_guns_and_missing_hands_are_not_given_fabricated_grips() {
        for prompt in ["wolf with laser cannon", "tank with cannon"] {
            let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            crate::v2::prepare(&mut spec);
            let mut body =
                crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
            crate::v2::anatomy::refine(&spec, &mut body, 42);
            let before = format!("{body:?}");
            assert!(grips(&spec, &body).is_empty(), "{prompt}");
            prepare_body(&spec, &mut body);
            assert_eq!(before, format!("{body:?}"), "{prompt}");
        }
        let mut spec = crate::parser::parse_vocabulary("orc with shotgun");
        spec.arm_count = 1;
        crate::v2::prepare(&mut spec);
        let body = crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
        assert!(grips(&spec, &body).is_empty());
    }
}
