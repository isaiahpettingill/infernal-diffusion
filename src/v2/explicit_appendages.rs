//! Requested appendages are native articulated chains, never surface ornaments.
//! Arrange their rest sockets before IK records lengths. Elevation and lateral
//! separation both matter: depth alone disappears in one of the baked views.
use super::{has, index, move_branch};
use crate::{anatomy::Body, parser::MonsterSpec};

pub(super) fn refine(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if has(spec, "EXPLICIT_HEADS") && spec.heads > 1 {
        heads(spec, body, s);
    }
    if has(spec, "EXPLICIT_ARMS") && spec.arm_count > 2 {
        arms(spec, body, s);
    }
}

fn heads(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let (Some(first_head), Some(first_neck)) = (index(body, "head_0"), index(body, "neck_0"))
    else {
        return;
    };
    let head = body.nodes[first_head].clone();
    let neck = body.nodes[first_neck].clone();
    let width = (head.rx * 2.5 + 2.0 * s).max(9.0 * s);
    let height = (head.ry * 0.65).max(1.5 * s);
    let depth = (head.rz.unwrap_or(head.ry) * 2.4 + 2.0 * s).max(8.0 * s);
    let center_z = body.nodes[first_neck]
        .parent
        .and_then(|p| body.nodes[p].z)
        .unwrap_or(0.0);
    // Species passes may already place head_0 at the far side of their crown.
    // Recenter that socket before deriving symmetric additional head positions.
    move_branch(body, first_neck, neck.x, neck.y, center_z);
    move_branch(body, first_head, head.x, head.y, center_z);
    // Keep skulls at a similar height and separate both forward and lateral
    // lanes. Stacking heads vertically lets an upper sauropod's long neck mesh
    // hide the lower skull even though the skull centers themselves are apart.
    for h in 1..spec.heads.min(5) {
        let (Some(hi), Some(ni)) = (
            index(body, &format!("head_{h}")),
            index(body, &format!("neck_{h}")),
        ) else {
            continue;
        };
        let side = if h % 2 == 1 { -1.0 } else { 1.0 };
        let tier = (h.div_ceil(2)) as f32;
        let dx = side * width * tier;
        let dy = -height * tier;
        let dz = side * depth * tier;
        move_branch(
            body,
            ni,
            neck.x + dx * 0.65,
            neck.y + dy * 0.45,
            center_z + dz * 0.65,
        );
        move_branch(body, hi, head.x + dx, head.y + dy, center_z + dz);
    }
}

fn arms(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let count = spec.arm_count.min(8);
    let pairs = count.div_ceil(2) as f32;
    for a in 0..count {
        let (Some(shoulder), Some(elbow), Some(hand)) = (
            index(body, &format!("shoulder_{a}")),
            index(body, &format!("arm_{a}")),
            index(body, &format!("hand_{a}")),
        ) else {
            continue;
        };
        let Some(parent) = body.nodes[shoulder].parent else {
            continue;
        };
        let chest = body.nodes[parent].clone();
        let old_shoulder = body.nodes[shoulder].clone();
        let old_elbow = body.nodes[elbow].clone();
        let old_hand = body.nodes[hand].clone();
        // Tiny theropod arms need enough reach to clear the chest when the user
        // asks for a many-armed mutant. This is rest anatomy, never animation
        // stretching. Existing enlarged-arm proportions remain enlarged.
        let upper = (old_elbow.x - old_shoulder.x)
            .hypot(old_elbow.y - old_shoulder.y)
            .max(5.2 * s);
        let lower = (old_hand.x - old_elbow.x)
            .hypot(old_hand.y - old_elbow.y)
            .max(4.5 * s);
        let side = if a % 2 == 0 { 1.0 } else { -1.0 };
        let rank = (a / 2) as f32;
        let angle = -1.12 + 2.24 * a as f32 / (count - 1) as f32;
        let bend = side * 0.12;
        let front = chest.x + chest.rx * 0.72;
        let socket_y =
            chest.y - chest.ry * 0.68 + rank * (chest.ry * 0.90 / (pairs - 1.0).max(1.0));
        let socket_z = chest.z.unwrap_or(0.0) + side * chest.rz.unwrap_or(chest.rx * 0.65) * 0.92;
        // Near/far chains also use staggered forward lanes. Camera elevation
        // otherwise projects opposite lateral offsets onto the same hand, even
        // when the unprojected Y positions are distinct.
        let shoulder_pos = [front + side * 0.7 * s, socket_y, socket_z];
        let elbow_pos = [
            front + upper * angle.cos() + side * 2.1 * s,
            socket_y + upper * angle.sin(),
            socket_z + side * (2.0 + 1.6 * angle.cos()) * s,
        ];
        let hand_pos = [
            elbow_pos[0] + lower * (angle + bend).cos() + side * 2.1 * s,
            elbow_pos[1] + lower * (angle + bend).sin(),
            elbow_pos[2] + side * (2.0 + 2.1 * angle.cos()) * s,
        ];
        for (i, p) in [
            (shoulder, shoulder_pos),
            (elbow, elbow_pos),
            (hand, hand_pos),
        ] {
            // Whole subtrees carry fingers, claws, jewelry and held equipment.
            move_branch(body, i, p[0], p[1], p[2]);
        }
        body.nodes[elbow].angle = angle - std::f32::consts::FRAC_PI_2;
        body.nodes[hand].angle = angle + bend - std::f32::consts::FRAC_PI_2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn fixture(prompt: &str, heads: u8, arms: u8) -> (MonsterSpec, Body) {
        let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
        spec.heads = heads;
        spec.arm_count = arms;
        spec.features
            .extend(["EXPLICIT_HEADS".into(), "EXPLICIT_ARMS".into()]);
        crate::v2::prepare(&mut spec);
        let mut body =
            crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
        crate::v2::anatomy::refine(&spec, &mut body, 42);
        (spec, body)
    }

    #[test]
    fn explicit_chains_keep_native_counts_parents_and_distinct_views() {
        for prompt in ["dinosaur", "t rex", "skeleton mermaid"] {
            let (spec, body) = fixture(prompt, 3, 8);
            assert_eq!(body.nodes.iter().filter(|n| n.kind == "HEAD").count(), 3);
            for arm in 0..8 {
                let shoulder = index(&body, &format!("shoulder_{arm}")).unwrap();
                let elbow = index(&body, &format!("arm_{arm}")).unwrap();
                let hand = index(&body, &format!("hand_{arm}")).unwrap();
                assert_eq!(body.nodes[elbow].parent, Some(shoulder));
                assert_eq!(body.nodes[hand].parent, Some(elbow));
            }
            for prefix in ["head", "hand"] {
                let count = if prefix == "head" { 3 } else { 8 };
                for a in 0..count {
                    let ia = index(&body, &format!("{prefix}_{a}")).unwrap();
                    let av = crate::render3d::collider_views_for_body(&body, ia, 128);
                    for b in 0..a {
                        let ib = index(&body, &format!("{prefix}_{b}")).unwrap();
                        let bv = crate::render3d::collider_views_for_body(&body, ib, 128);
                        for view in 0..4 {
                            let separation =
                                (av[view].x - bv[view].x).hypot(av[view].y - bv[view].y);
                            assert!(
                                separation > (av[view].radius + bv[view].radius) * 0.80,
                                "{} {prefix} {a}/{b} overlap in view {view}: {separation}",
                                spec.affinity,
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn explicit_layout_preserves_attached_details_and_materials() {
        let (spec, mut body) = fixture("skeleton mermaid", 1, 8);
        let shoulder = index(&body, "shoulder_7").unwrap();
        let elbow = index(&body, "arm_7").unwrap();
        let hand = index(&body, "hand_7").unwrap();
        let detail = super::super::landmark(
            &mut body,
            hand,
            "test_finger",
            "CLAW",
            [20.0, 20.0, 20.0],
            [0.3, 0.7, 0.3],
            "BONE",
        );
        let offset = [
            body.nodes[detail].x - body.nodes[hand].x,
            body.nodes[detail].y - body.nodes[hand].y,
            body.nodes[detail].z.unwrap() - body.nodes[hand].z.unwrap(),
        ];
        let materials: Vec<_> = body.nodes.iter().map(|n| n.material.clone()).collect();
        arms(
            &spec,
            &mut body,
            crate::anatomy::morphology_scale(spec.size),
        );
        assert_eq!(body.nodes[elbow].parent, Some(shoulder));
        assert_eq!(body.nodes[detail].parent, Some(hand));
        for (a, b) in offset.into_iter().zip([
            body.nodes[detail].x - body.nodes[hand].x,
            body.nodes[detail].y - body.nodes[hand].y,
            body.nodes[detail].z.unwrap() - body.nodes[hand].z.unwrap(),
        ]) {
            assert!((a - b).abs() < 0.0001);
        }
        assert_eq!(
            materials,
            body.nodes
                .iter()
                .map(|n| n.material.clone())
                .collect::<Vec<_>>()
        );
    }
}
