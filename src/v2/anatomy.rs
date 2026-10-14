//! Species landmarks for the opt-in pipeline. Rest geometry is finalized before
//! animation/IK builds its fixed-length constraints; no frame-dependent scaling.
use crate::anatomy::{Body, Node};
use crate::parser::MonsterSpec;

fn has(spec: &MonsterSpec, feature: &str) -> bool {
    spec.features.iter().any(|f| f == feature)
}

pub fn prepare_spec(spec: &mut MonsterSpec) {
    if matches!(spec.affinity.as_str(), "FROG" | "TOAD") {
        spec.body_plan = "QUADRUPED".into();
        for feature in ["WEBBED", "HOP"] {
            if !has(spec, feature) {
                spec.features.push(feature.into());
            }
        }
    }
    if spec.affinity == "MERMAID" && !has(spec, "EXPLICIT_LEGS") {
        spec.limb_count = 0;
    }
}

/// Remove whole attachment subtrees while remapping every surviving parent.
fn remove_where(body: &mut Body, predicate: impl Fn(&Node) -> bool) {
    let mut removed = vec![false; body.nodes.len()];
    for (i, n) in body.nodes.iter().enumerate() {
        removed[i] = predicate(n) || n.parent.map(|p| removed[p]).unwrap_or(false);
    }
    let mut remap = vec![0; body.nodes.len()];
    let mut kept = Vec::new();
    for (i, n) in body.nodes.iter().enumerate() {
        if !removed[i] {
            let mut n = n.clone();
            n.parent = n.parent.map(|p| remap[p]);
            remap[i] = kept.len();
            kept.push(n);
        }
    }
    body.nodes = kept;
}

/// Move an attachment with all of its details, so eyes and equipment stay seated.
fn move_branch(body: &mut Body, index: usize, x: f32, y: f32, z: f32) {
    let dx = x - body.nodes[index].x;
    let dy = y - body.nodes[index].y;
    let dz = z - body.nodes[index].z.unwrap_or(0.0);
    let mut affected = vec![false; body.nodes.len()];
    for i in index..body.nodes.len() {
        affected[i] = i == index || body.nodes[i].parent.map(|p| affected[p]).unwrap_or(false);
        if affected[i] {
            let n = &mut body.nodes[i];
            n.x += dx;
            n.y += dy;
            n.z = Some(n.z.unwrap_or(0.0) + dz);
        }
    }
}

fn landmark(
    body: &mut Body,
    parent: usize,
    id: &str,
    kind: &str,
    position: [f32; 3],
    radii: [f32; 3],
    material: &str,
) -> usize {
    let index = body.nodes.len();
    body.nodes.push(Node {
        id: id.into(),
        kind: kind.into(),
        parent: Some(parent),
        x: position[0],
        y: position[1],
        z: Some(position[2]),
        rx: radii[0],
        ry: radii[1],
        rz: Some(radii[2]),
        angle: 0.0,
        material: material.into(),
        density: body.nodes[parent].density,
        feature: false,
    });
    index
}

fn index(body: &Body, id: &str) -> Option<usize> {
    body.nodes.iter().position(|n| n.id == id)
}

fn frog(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if !has(spec, "TAIL") && !has(spec, "STINGER") && !has(spec, "TAIL_CLUB") {
        remove_where(body, |n| n.id == "tail" || n.id.starts_with("tail_"));
    }
    if let Some(t) = index(body, "torso") {
        body.nodes[t].rx *= 0.83;
        body.nodes[t].ry *= 0.88;
        body.nodes[t].rz = Some(6.0 * s);
    }
    for h in 0..spec.heads.min(5) {
        if let Some(i) = index(body, &format!("neck_{h}")) {
            body.nodes[i].rx *= 0.8;
            body.nodes[i].ry *= 0.65;
        }
        if let Some(i) = index(body, &format!("head_{h}")) {
            let z = body.nodes[i].z.unwrap_or(match h {
                0 => 0.0,
                1 => -7.0 * s,
                2 => 7.0 * s,
                3 => -13.0 * s,
                _ => 13.0 * s,
            });
            move_branch(body, i, 8.3 * s, -3.0 * s, z);
            body.nodes[i].rx *= 1.2;
            body.nodes[i].ry *= 0.82;
            body.nodes[i].rz = Some(5.0 * s);
            let mat = body.nodes[i].material.clone();
            // Raised orbital masses are silhouette landmarks, not extra eyes.
            for side in [-1.0, 1.0] {
                landmark(
                    body,
                    i,
                    &format!("v2_orbit_{h}_{}", if side > 0.0 { 0 } else { 1 }),
                    "LOBE",
                    [8.4 * s, -5.2 * s, z + side * 3.4 * s],
                    [1.9 * s, 1.6 * s, 1.7 * s],
                    &mat,
                );
            }
        }
    }
    for leg in 0..spec.limb_count.min(12) {
        let side = if leg % 2 == 0 { 1.0 } else { -1.0 };
        let rear = leg >= 2;
        let rank = (leg / 2) as f32;
        let x = if rear {
            -5.0 * s - (rank - 1.0) * 1.5 * s
        } else {
            6.0 * s
        };
        for (name, px, py, pz) in [
            ("leg_root", x, 1.0 * s, side * 4.5 * s),
            (
                "limb",
                if rear { x - 2.0 * s } else { x + 1.5 * s },
                if rear { 2.5 * s } else { 5.0 * s },
                side * 6.0 * s,
            ),
            (
                "shin",
                if rear { x + 2.8 * s } else { x + 2.2 * s },
                6.0 * s,
                side * 6.3 * s,
            ),
            (
                "foot",
                if rear { x + 0.5 * s } else { x + 3.4 * s },
                8.0 * s,
                side * 7.0 * s,
            ),
        ] {
            if let Some(i) = index(body, &format!("{name}_{leg}")) {
                move_branch(body, i, px, py, pz);
                if name == "limb" && rear {
                    body.nodes[i].rx *= 1.7;
                    body.nodes[i].ry *= 0.8;
                }
                if name == "foot" {
                    body.nodes[i].rx *= 1.3;
                    body.nodes[i].ry *= 0.55;
                    body.nodes[i].rz = Some(2.4 * s);
                }
            }
        }
    }
}

fn mermaid(spec: &MonsterSpec, body: &mut Body, s: f32) {
    // Replace the default fish dorsal fins with the native caudal fan. Explicit
    // non-native details (back plates, wings and equipment) remain untouched.
    remove_where(body, |n| {
        matches!(n.id.as_str(), "fin_0" | "fin_1" | "fin_2")
    });
    if spec.limb_count == 0 {
        remove_where(body, |n| n.kind == "HIP" && !n.id.starts_with("rider_"));
    }
    let Some(t) = index(body, "torso") else {
        return;
    };
    // Keep the humanoid shoulders/arms; the lower silhouette is a single fish tail.
    body.nodes[t].rx *= 0.65;
    body.nodes[t].rz = Some(body.nodes[t].rx * 0.58);
    let material = body.nodes[t].material.clone();
    let mut parent = t;
    for (i, (x, y, rx, ry, rz)) in [
        (0.0, 5.0, 4.7, 5.0, 3.3),
        (-1.0, 11.0, 3.7, 4.7, 2.8),
        (-3.5, 16.0, 2.7, 3.8, 2.0),
        (-7.0, 19.3, 1.7, 2.8, 1.3),
        (-10.5, 20.0, 1.0, 2.0, 0.9),
    ]
    .iter()
    .copied()
    .enumerate()
    {
        parent = landmark(
            body,
            parent,
            &format!("v2_fish_tail_{i}"),
            "TAIL",
            [x * s, y * s, 0.0],
            [rx * s, ry * s, rz * s],
            &material,
        );
    }
    for (i, side) in [-1.0, 1.0].iter().copied().enumerate() {
        let fin = landmark(
            body,
            parent,
            &format!("v2_fluke_{i}"),
            "FIN",
            [-12.3 * s, 20.0 * s, side * 2.7 * s],
            [3.8 * s, 1.2 * s, 3.3 * s],
            &material,
        );
        body.nodes[fin].angle = side * 0.45;
    }
}

fn tree(body: &mut Body, s: f32) {
    let Some(t) = index(body, "torso") else {
        return;
    };
    body.nodes[t].rz = Some(body.nodes[t].rx * 0.85);
    let roots: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.kind == "ROOT")
        .map(|(i, _)| i)
        .collect();
    for (rank, i) in roots.iter().copied().enumerate() {
        let a = rank as f32 * std::f32::consts::TAU / roots.len().max(1) as f32;
        move_branch(body, i, a.cos() * 7.0 * s, 10.0 * s, a.sin() * 6.0 * s);
        body.nodes[i].rx *= 1.4;
        body.nodes[i].rz = Some(1.5 * s);
    }
    let canopies: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.id.starts_with("canopy_"))
        .map(|(i, _)| i)
        .collect();
    for (rank, i) in canopies.iter().copied().enumerate() {
        let side = rank as f32 - (canopies.len() as f32 - 1.0) * 0.5;
        let x = side * 5.0 * s;
        let y = (-26.0 - (2.0 - side.abs()) * 3.5) * s;
        let z = if rank % 2 == 0 { -2.7 * s } else { 2.7 * s };
        move_branch(body, i, x, y, z);
        body.nodes[i].rz = Some(4.8 * s);
        // Add branching wood under the existing crown without new leaf clusters.
        let branch = landmark(
            body,
            t,
            &format!("v2_bough_{rank}"),
            "ROOT",
            [x * 0.62, y + 5.0 * s, z * 0.7],
            [1.4 * s, 5.0 * s, 1.2 * s],
            "WOOD",
        );
        // Parent indices remain topologically sorted: leave canopy's original
        // attachment intact, the visible bough meets the same crown spatially.
        body.nodes[branch].angle = -side * 0.24;
    }
}

/// Apply modest, coherent shape changes across all families, then defining
/// species landmarks. Explicit modifiers and counts remain on the same graph.
pub fn refine(spec: &MonsterSpec, body: &mut Body, _seed: u64) {
    if body.nodes.is_empty() {
        return;
    }
    // Composite mounts retain their separately built anatomy and attachment graph.
    // Do not reshape the mount as though it were the rider's species.
    if spec.mount.is_some() {
        return;
    }
    if spec.secondary_affinities.is_empty() {
        remove_where(body, |n| n.id.starts_with("hybrid_"));
    }
    // V1 historically forced one leg for several plans. V2 honors zero and
    // exact requested counts without leaving detached shins, feet or claws.
    remove_where(body, |n| {
        n.kind == "HIP"
            && (n.id.starts_with("hip_") || n.id.starts_with("leg_root_"))
            && n.id
                .rsplit('_')
                .next()
                .and_then(|v| v.parse::<u8>().ok())
                .map(|rank| rank >= spec.limb_count.min(12))
                .unwrap_or(false)
    });
    let s = crate::anatomy::morphology_scale(spec.size);
    // Consistent lateral attachment dimensions for generic legacy limb chains.
    for n in &mut body.nodes {
        if n.z.is_none()
            && matches!(
                n.kind.as_str(),
                "HIP" | "LIMB" | "SHIN" | "FOOT" | "SHOULDER" | "ARM" | "HAND"
            )
        {
            let rank =
                n.id.rsplit('_')
                    .next()
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(0);
            n.z =
                Some(if rank % 2 == 0 { 1.0 } else { -1.0 } * (3.3 + (rank / 2) as f32 * 0.35) * s);
        }
        match (spec.body_plan.as_str(), n.kind.as_str()) {
            ("HUMANOID", "TORSO") => {
                n.rx *= 0.9;
                n.rz = Some(n.rx * 0.6);
            }
            ("QUADRUPED", "TORSO") => {
                n.rz = Some(n.ry * 0.94);
            }
            ("THEROPOD", "TORSO") => {
                n.rx *= 1.08;
                n.ry *= 0.92;
                n.rz = Some(n.ry * 0.8);
            }
            ("THEROPOD", "LIMB") => {
                n.rx *= 1.22;
            }
            ("ARACHNID", "ABDOMEN") => {
                n.ry *= 1.08;
            }
            ("INSECT", "TORSO") => {
                n.ry *= 1.06;
            }
            ("SERPENT", "TAIL") => {
                n.rz = Some(n.ry * 0.92);
            }
            ("GASTROPOD", "SOLE") => {
                n.rx *= 1.04;
                n.rz = Some(5.0 * s);
            }
            ("FLOATING", "TENTACLE") => {
                n.rx *= 0.74;
                n.ry *= 1.15;
            }
            ("AMORPHOUS", "LOBE") => {
                n.rz = Some(n.rx * 0.95);
            }
            ("FUNGUS", "HEAD") if spec.affinity != "TREE_MONSTER" => {
                n.rx *= 1.12;
                n.ry *= 0.85;
                n.rz = Some(n.rx * 0.85);
            }
            ("TANK", "CHASSIS") => {
                n.rz = Some(5.5 * s);
            }
            ("WINGED", "TORSO") if spec.is_bird() => {
                n.rx *= 0.9;
                n.ry *= 1.1;
                n.rz = Some(n.ry * 0.85);
            }
            ("WINGED", "TORSO") => {
                n.rz = Some(n.ry * 0.85);
            }
            _ => {}
        }
    }
    match spec.affinity.as_str() {
        "FROG" | "TOAD" => frog(spec, body, s),
        "MERMAID" => mermaid(spec, body, s),
        "TREE_MONSTER" => tree(body, s),
        _ => {}
    }
    if has(spec, "FRIENDLY_LURE") {
        for n in &mut body.nodes {
            if n.kind == "HEAD" {
                n.rx *= 1.05;
                n.ry *= 1.04;
            }
            if n.kind == "EYE" {
                n.rx *= 1.12;
                n.ry *= 1.12;
            }
        }
    }
    // Recompute the same 2D mass convention used by V1 after shape changes.
    let mut mass = 0.0;
    let mut weighted = (0.0, 0.0);
    for n in &body.nodes {
        let m = std::f32::consts::PI * n.rx * n.ry * n.density;
        mass += m;
        weighted.0 += m * n.x;
        weighted.1 += m * n.y;
    }
    body.mass = mass;
    if mass > 0.0 {
        body.com = (weighted.0 / mass, weighted.1 / mass);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    fn build(prompt: &str) -> (MonsterSpec, Body) {
        let mut spec = crate::parser::parse_vocabulary(prompt);
        if prompt == "frog" {
            spec.affinity = "FROG".into();
            spec.body_plan = "QUADRUPED".into();
            spec.limb_count = 4;
            spec.arm_count = 0;
        }
        prepare_spec(&mut spec);
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        refine(&spec, &mut body, 42);
        (spec, body)
    }
    #[test]
    fn all_families_are_deterministic_finite_connected() {
        for prompt in [
            "orc",
            "wolf",
            "t rex",
            "spider",
            "beetle",
            "snake",
            "slug",
            "ghost",
            "slime",
            "mushroom",
            "tank",
            "eagle",
            "dragon",
            "frog",
            "mermaid",
            "tree monster",
        ] {
            let (_, a) = build(prompt);
            let (_, b) = build(prompt);
            assert_eq!(format!("{a:?}"), format!("{b:?}"), "{prompt}");
            assert!(a.mass.is_finite() && a.mass > 0.0);
            for (i, n) in a.nodes.iter().enumerate() {
                assert!(
                    n.x.is_finite() && n.y.is_finite() && n.rx.is_finite() && n.ry.is_finite(),
                    "{prompt}: {}",
                    n.id
                );
                assert!(n.rx > 0.0 && n.ry > 0.0);
                assert!(n.z.map(f32::is_finite).unwrap_or(true));
                assert!(n.rz.map(|v| v.is_finite() && v > 0.0).unwrap_or(true));
                if i > 0 {
                    assert!(n.parent.is_some(), "{prompt}: {}", n.id);
                }
                assert!(n.parent.map(|p| p < i).unwrap_or(true));
            }
        }
    }
    #[test]
    fn frog_has_broad_head_folded_haunches_and_no_default_tail() {
        let (_, b) = build("frog");
        let h = &b.nodes[index(&b, "head_0").unwrap()];
        assert!(h.rz.unwrap() > h.ry);
        assert!(!b.nodes.iter().any(|n| n.kind == "TAIL"));
        let thigh = &b.nodes[index(&b, "limb_2").unwrap()];
        let shin = &b.nodes[index(&b, "shin_2").unwrap()];
        assert!(thigh.x < shin.x && thigh.rx > b.nodes[index(&b, "limb_0").unwrap()].rx);
        assert_eq!(b.nodes.iter().filter(|n| n.kind == "FOOT").count(), 4);
    }
    #[test]
    fn mermaid_has_single_tapered_tail_and_flukes() {
        let (_, b) = build("mermaid");
        assert!(!b.nodes.iter().any(|n| n.kind == "FOOT"));
        let tail: Vec<_> = b
            .nodes
            .iter()
            .filter(|n| n.id.starts_with("v2_fish_tail_"))
            .collect();
        assert_eq!(tail.len(), 5);
        assert!(tail.windows(2).all(|w| w[0].rx > w[1].rx));
        assert_eq!(
            b.nodes
                .iter()
                .filter(|n| n.id.starts_with("v2_fluke_"))
                .count(),
            2
        );
        assert_eq!(b.nodes.iter().filter(|n| n.kind == "HAND").count(), 2);
    }
    #[test]
    fn tree_has_radial_roots_and_branching_crown() {
        let (_, b) = build("tree monster");
        assert_eq!(
            b.nodes
                .iter()
                .filter(|n| n.id.starts_with("v2_bough_"))
                .count(),
            5
        );
        let roots: Vec<_> = b
            .nodes
            .iter()
            .filter(|n| n.id.starts_with("root_"))
            .collect();
        assert!(roots.iter().any(|n| n.z.unwrap() > 1.0));
        assert!(roots.iter().any(|n| n.z.unwrap() < -1.0));
    }
    #[test]
    fn friendly_lure_does_not_add_grotesque_accessories() {
        let (mut s, b) = build("rabbit");
        s.features.push("FRIENDLY_LURE".into());
        let mut friendly = b.clone();
        refine(&s, &mut friendly, 42);
        assert_eq!(b.nodes.len(), friendly.nodes.len());
        assert_eq!(
            b.nodes.iter().filter(|n| n.kind == "FANG").count(),
            friendly.nodes.iter().filter(|n| n.kind == "FANG").count()
        );
    }
    #[test]
    fn zero_legs_are_respected_in_every_limb_family() {
        for prompt in ["orc", "wolf", "spider", "beetle", "eagle"] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            spec.limb_count = 0;
            let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(7));
            refine(&spec, &mut body, 7);
            assert!(!body.nodes.iter().any(|n| n.kind == "FOOT"), "{prompt}");
        }
    }
    #[test]
    fn requested_mermaid_legs_and_extra_heads_survive() {
        let mut s = crate::parser::parse_vocabulary("mermaid with four legs and three heads");
        s.features.push("EXPLICIT_LEGS".into());
        s.limb_count = 4;
        s.heads = 3;
        prepare_spec(&mut s);
        let mut b = crate::anatomy::build(&s, &mut ChaCha8Rng::seed_from_u64(12));
        refine(&s, &mut b, 12);
        assert_eq!(b.nodes.iter().filter(|n| n.kind == "FOOT").count(), 4);
        assert_eq!(b.nodes.iter().filter(|n| n.kind == "HEAD").count(), 3);
    }
}
