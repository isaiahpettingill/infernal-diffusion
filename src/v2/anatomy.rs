//! Species landmarks for the opt-in pipeline. Rest geometry is finalized before
//! animation/IK builds its fixed-length constraints; no frame-dependent scaling.
use crate::anatomy::{Body, Node};
use crate::parser::MonsterSpec;
#[path = "explicit_appendages.rs"]
mod explicit_appendages;
#[path = "species_anatomy.rs"]
mod species_anatomy;

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
    if matches!(
        spec.affinity.as_str(),
        "CENTAUR" | "NAGA" | "LAMIA" | "ECHIDNA" | "GARUDA" | "VALKYRIE"
    ) && spec.arm_count == 0
        && !has(spec, "EXPLICIT_ARMS")
    {
        spec.arm_count = 2;
    }
    if matches!(spec.affinity.as_str(), "MERMAID" | "MERMAN") && !has(spec, "EXPLICIT_LEGS") {
        spec.limb_count = 0;
    }
}

/// Sample topology before the base builder so native arm chains, attack origins
/// and rig solvers agree on their count. An independent integer stream means
/// adding a material draw can never change whether a specimen is a mutant.
pub fn prepare_seeded(spec: &mut MonsterSpec, seed: u64) {
    if !has(spec, "PROFILE_V2") {
        return;
    }
    // Encounter silhouettes are authored; random dancer/mutant rolls must not
    // silently replace their engine-facing interaction contracts.
    if super::encounter_visuals::enabled(spec) {
        return;
    }
    let mut bits = seed ^ 0x9e3779b97f4a7c15;
    for byte in spec.affinity.bytes() {
        bits = (bits ^ byte as u64).wrapping_mul(0x100000001b3);
    }
    bits ^= bits >> 30;
    bits = bits.wrapping_mul(0xbf58476d1ce4e5b9);
    bits ^= bits >> 27;
    if bits.is_multiple_of(1000) && !has(spec, "DANCER") {
        spec.features
            .extend(["DANCER".into(), "RARE_DANCER".into()]);
    }
    if spec.mount.is_some() || has(spec, "TOPOLOGY_LOCKED_INTERNAL") {
        return;
    }
    if has(spec, "ZOMBIFIED") {
        if spec.arm_count > 0 && !has(spec, "EXPLICIT_ARMS") {
            spec.arm_count -= 1;
        } else if spec.limb_count > 0 && !has(spec, "EXPLICIT_LEGS") {
            spec.limb_count -= 1;
        }
    }
    let humanoid = spec.body_plan == "HUMANOID"
        || matches!(
            spec.affinity.as_str(),
            "TREE_MONSTER"
                | "HARPY"
                | "SIREN"
                | "GARUDA"
                | "VALKYRIE"
                | "CENTAUR"
                | "NAGA"
                | "LAMIA"
                | "ECHIDNA"
        );
    let organic =
        !matches!(spec.affinity.as_str(), "ROBOT" | "STORMTROOPER") && !has(spec, "ZOMBIFIED");
    if humanoid
        && organic
        && spec.arm_count == 2
        && !has(spec, "EXPLICIT_ARMS")
        && bits.is_multiple_of(8)
    {
        spec.arm_count = 4;
        for tag in ["FOUR_ARMED", "MUTANT_UNSTABLE"] {
            if !has(spec, tag) {
                spec.features.push(tag.into());
            }
        }
    }
    if matches!(spec.affinity.as_str(), "ORC" | "TROLL") && (bits >> 16).is_multiple_of(4) {
        spec.features.push("AI_ANATOMY_ECHO".into());
    }
    let merfolk = matches!(spec.affinity.as_str(), "MERMAID" | "MERMAN");
    let tail_family = merfolk
        || matches!(spec.body_plan.as_str(), "QUADRUPED" | "SERPENT")
        || (humanoid && organic && spec.affinity != "TREE_MONSTER");
    let chance = if merfolk { 6 } else { 12 };
    if tail_family && !has(spec, "EXPLICIT_TAILS") && (bits >> 32).is_multiple_of(chance) {
        for tag in ["MUTATION_EXTRA_TAIL", "EXTRA_TAIL", "MUTANT_UNSTABLE"] {
            if !has(spec, tag) {
                spec.features.push(tag.into());
            }
        }
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

/// A mounted graph is assembled by the legacy builder before V2 refinement.
/// Its unprefixed nodes belong to the deepest mount; rider_, mount_rider_, etc.
/// belong to successive rider clauses. Apply the ordinary no-invented-hybrids
/// rule to each owner, preserving a mount's explicitly requested secondary parts.
fn remove_unrequested_mounted_hybrids(spec: &MonsterSpec, body: &mut Body) {
    let mut explicit_hybrids = vec![!spec.secondary_affinities.is_empty()];
    let mut owner = spec;
    while let Some(mount) = &owner.mount {
        if let Some(lower) = mount.spec.as_deref() {
            explicit_hybrids.push(!lower.secondary_affinities.is_empty());
            owner = lower;
        } else {
            // The legacy fallback mount spec has no requested secondary parts.
            explicit_hybrids.push(false);
            break;
        }
    }
    remove_where(body, |node| {
        let mut id = node.id.as_str();
        let mut depth = 0;
        while let Some(remainder) = id.strip_prefix("mount_") {
            depth += 1;
            id = remainder;
        }
        if let Some(remainder) = id.strip_prefix("rider_") {
            id = remainder;
        } else {
            depth = explicit_hybrids.len() - 1;
        }
        id.starts_with("hybrid_") && explicit_hybrids.get(depth) == Some(&false)
    });
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
    body.nodes[t].rx *= 0.43 + spec.bulk * 0.12;
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
            [4.6 * s, 2.6 * s, 3.7 * s],
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

/// A narrow waist and a weight-bearing pelvis unify humanoid hybrids without
/// replacing their animal heads, horns, explicit limbs, materials or equipment.
fn humanoid_hybrid(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let bird = matches!(spec.affinity.as_str(), "HARPY" | "SIREN" | "GARUDA");
    let upright = spec.body_plan == "HUMANOID" || bird || spec.affinity == "VALKYRIE";
    if !upright {
        return;
    }
    let Some(t) = index(body, "torso") else {
        return;
    };
    let torso = body.nodes[t].clone();
    if bird {
        body.nodes[t].rx *= 0.59 + spec.bulk * 0.12;
        body.nodes[t].ry *= 1.08;
        body.nodes[t].rz = Some(body.nodes[t].rx * 0.61);
    }
    if spec.limb_count > 0 {
        landmark(
            body,
            t,
            "v2_pelvis",
            "LOBE",
            [torso.x, torso.y + torso.ry * 0.69, 0.0],
            [
                torso.rx * if bird { 0.55 } else { 0.66 },
                torso.ry * 0.38,
                torso.rx * if bird { 0.36 } else { 0.48 },
            ],
            &torso.material,
        );
    }
    for h in 0..spec.heads.min(5) {
        if let Some(i) = index(body, &format!("head_{h}")) {
            if bird {
                body.nodes[i].rx *= 0.88;
                body.nodes[i].ry *= 1.08;
                if body.nodes[i].material.starts_with("FEATHER")
                    && matches!(spec.affinity.as_str(), "HARPY" | "SIREN")
                {
                    body.nodes[i].material = "FLESH".into();
                }
            } else if matches!(
                spec.affinity.as_str(),
                "JACKAL"
                    | "XOLOTL"
                    | "NAGUAL"
                    | "CROCODILE"
                    | "MINOTAUR"
                    | "LION"
                    | "CAT"
                    | "RAKSHASA"
            ) {
                body.nodes[i].rx *= 1.12;
                body.nodes[i].ry *= 0.94;
            }
        }
    }
    if bird {
        for wing in 0..2 {
            if let Some(i) = index(body, &format!("wing_{wing}")) {
                let sign = if wing == 0 { 1.0 } else { -1.0 };
                move_branch(
                    body,
                    i,
                    torso.x - 1.2 * s,
                    torso.y - 4.3 * s,
                    sign * 3.8 * s,
                );
                body.nodes[i].rx *= 1.06;
            }
        }
    }
    if bird || matches!(spec.affinity.as_str(), "MERMAID" | "MERMAN") {
        for arm in 0..spec.arm_count.min(8) {
            if let Some(i) = index(body, &format!("shoulder_{arm}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x * 0.8, n.y, n.z.unwrap_or(0.0) * 0.9);
            }
        }
        for n in &mut body.nodes {
            if matches!(n.kind.as_str(), "SHOULDER" | "ARM") {
                n.rx *= 0.76;
                n.rz = n.rz.map(|r| r * 0.8);
            }
            if n.kind == "HAND" {
                n.rx *= 0.86;
            }
        }
    }
    if bird
        || matches!(
            spec.affinity.as_str(),
            "FAUN" | "SATYR" | "MINOTAUR" | "JACKAL" | "XOLOTL" | "NAGUAL"
        )
    {
        for leg in 0..spec.limb_count.min(12) {
            if let Some(i) = index(body, &format!("shin_{leg}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x - 1.8 * s, n.y - 0.6 * s, n.z.unwrap_or(0.0));
                body.nodes[i].rx *= 0.75;
            }
            if let Some(i) = index(body, &format!("foot_{leg}")) {
                let n = body.nodes[i].clone();
                move_branch(body, i, n.x + 2.0 * s, n.y, n.z.unwrap_or(0.0));
                body.nodes[i].rx *= if bird { 1.24 } else { 1.08 };
                body.nodes[i].ry *= 0.8;
            }
        }
    }
    // Four arms emerge from two distinct shoulder levels, not one overlapping
    // socket. The ordinary ARM/HAND IDs retain support for attack and IK code.
    if spec.arm_count >= 4 {
        for arm in 2..spec.arm_count.min(8) {
            if let Some(i) = index(body, &format!("shoulder_{arm}")) {
                let n = body.nodes[i].clone();
                let sign = if arm % 2 == 0 { 1.0 } else { -1.0 };
                move_branch(body, i, n.x * 0.9, n.y + 1.2 * s, sign * 4.0 * s);
            }
        }
    }
}

/// Human upper body joined to a horse or serpent substrate. The original
/// locomotor graph stays intact; all raised head/arm details move together.
fn composite_hybrid(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if !matches!(
        spec.affinity.as_str(),
        "CENTAUR" | "NAGA" | "LAMIA" | "ECHIDNA"
    ) {
        return;
    }
    let Some(t) = index(body, "torso") else {
        return;
    };
    let mat = body.nodes[t].material.clone();
    landmark(
        body,
        t,
        "v2_hybrid_chest",
        "LOBE",
        [5.0 * s, -10.0 * s, 0.0],
        [4.8 * s, 8.0 * s, 3.4 * s],
        &mat,
    );
    for h in 0..spec.heads.min(5) {
        let spread = (h as f32 - (spec.heads.min(5) as f32 - 1.0) * 0.5) * 5.0 * s;
        if let Some(i) = index(body, &format!("neck_{h}")) {
            move_branch(body, i, 5.5 * s + spread, -17.0 * s, 0.0);
            body.nodes[i].rx *= 0.8;
            body.nodes[i].ry = 3.2 * s;
        }
        if let Some(i) = index(body, &format!("head_{h}")) {
            move_branch(body, i, 6.5 * s + spread, -22.0 * s, 0.0);
            body.nodes[i].ry *= 1.25;
        }
    }
    for arm in 0..spec.arm_count.min(8) {
        if let Some(i) = index(body, &format!("shoulder_{arm}")) {
            let sign = if arm % 2 == 0 { 1.0 } else { -1.0 };
            move_branch(
                body,
                i,
                5.0 * s + sign * 3.4 * s,
                (-13.0 + (arm / 2) as f32 * 3.8) * s,
                sign * 3.3 * s,
            );
        }
    }
}

/// A secondary tail is a full tapered rig with its own descendants. Clone only
/// native tail anatomy, never weapons, hybrid heads, or unrelated appendages.
fn extra_tail(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if !has(spec, "MUTATION_EXTRA_TAIL") || has(spec, "EXPLICIT_TAILS") {
        return;
    }
    let Some(t) = index(body, "torso") else {
        return;
    };
    let merfolk = matches!(spec.affinity.as_str(), "MERMAID" | "MERMAN");
    let source = if merfolk {
        index(body, "v2_fish_tail_0")
    } else {
        body.nodes
            .iter()
            .position(|n| n.kind == "TAIL" && n.parent == Some(t))
    };
    if let Some(root) = source {
        let old = body.nodes.clone();
        let mut remap = vec![None; old.len()];
        for (i, n) in old.iter().enumerate().skip(root) {
            let selected = i == root || n.parent.and_then(|p| remap[p]).is_some();
            if !selected
                || !matches!(
                    n.kind.as_str(),
                    "TAIL" | "FIN" | "LOBE" | "SHELL" | "STINGER"
                )
            {
                continue;
            }
            // At most twelve segments/details keep topology and bake cost bounded.
            if remap.iter().filter(|v| v.is_some()).count() >= 12 {
                break;
            }
            let mut node = n.clone();
            node.id = format!("v2_extra_tail_{}", n.id);
            node.parent = Some(n.parent.and_then(|p| remap[p]).unwrap_or(t));
            node.x = old[t].x + (n.x - old[t].x) * 0.88;
            node.y = old[t].y + (n.y - old[t].y) * 0.92;
            node.z = Some(n.z.unwrap_or(0.0) + 3.8 * s);
            node.rx *= 0.82;
            node.ry *= 0.82;
            node.rz = Some(n.rz.unwrap_or(n.rx.min(n.ry) * 0.85) * 0.82);
            remap[i] = Some(body.nodes.len());
            body.nodes.push(node);
        }
        // Separate paired roots while leaving every child's branch offset intact.
        let n = body.nodes[root].clone();
        move_branch(body, root, n.x, n.y, n.z.unwrap_or(0.0) - 1.8 * s);
    } else {
        let mat = body.nodes[t].material.clone();
        let mut parent = t;
        for i in 0..5 {
            let taper = 1.0 - i as f32 * 0.16;
            parent = landmark(
                body,
                parent,
                &format!("v2_extra_tail_{i}"),
                "TAIL",
                [
                    -3.0 * s - i as f32 * 3.0 * s,
                    4.0 * s + i as f32 * 0.7 * s,
                    2.0 * s,
                ],
                [3.0 * s * taper, 2.3 * s * taper, 2.0 * s * taper],
                &mat,
            );
        }
    }
}

/// Broken-looking off-axis bone growth is decorative; the functional joint
/// centers and bone lengths stay intact so the skeleton remains animatable.
fn skeletal_spurs(spec: &MonsterSpec, body: &mut Body, seed: u64, s: f32) {
    if !matches!(spec.affinity.as_str(), "SKELETON" | "TZITZIMITL")
        && !spec.materials.iter().any(|m| m == "BONE")
    {
        return;
    }
    let candidates: Vec<_> = body
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| matches!(n.kind.as_str(), "ARM" | "LIMB" | "SHIN"))
        .map(|(i, _)| i)
        .collect();
    let mut added = 0;
    for (rank, parent) in candidates.into_iter().enumerate() {
        if added >= 4 {
            break;
        }
        let bits = seed
            .wrapping_add(rank as u64 * 0x9e37)
            .wrapping_mul(0xbf58476d1ce4e5b9);
        if (bits >> 27).is_multiple_of(3) {
            continue;
        }
        let n = body.nodes[parent].clone();
        let sign = if rank % 2 == 0 { 1.0 } else { -1.0 };
        let spur = landmark(
            body,
            parent,
            &format!("v2_bone_spur_{added}"),
            "BONE_SPUR",
            [
                n.x + sign * 0.45 * s,
                n.y - 0.5 * s,
                n.z.unwrap_or(0.0) + sign * 0.6 * s,
            ],
            [0.38 * s, (1.2 + (bits % 7) as f32 * 0.1) * s, 0.32 * s],
            "BONE",
        );
        body.nodes[spur].angle = sign * (0.25 + (bits % 5) as f32 * 0.12);
        body.nodes[spur].feature = true;
        body.nodes[spur].density = 1.35;
        added += 1;
    }
}

/// Three continuous, bounded shade tendrils coexist with the base silhouette.
/// Their rest skeleton is fixed; combat supplies the smooth lash deformation.
fn shade_tendrils(spec: &MonsterSpec, body: &mut Body, s: f32) {
    if !has(spec, "PROFILE_V2")
        || !has(spec, "SHADE")
        || body.nodes.is_empty()
        || body
            .nodes
            .iter()
            .any(|n| n.id.starts_with("v2_shade_tendril_"))
    {
        return;
    }
    remove_where(body, |n| n.kind == "WEAPON");
    let torso = body.nodes[0].clone();
    for chain in 0..3 {
        let lateral = match chain {
            0 => -1.0,
            1 => 1.0,
            _ => 0.0,
        };
        let mut parent = 0;
        for segment in 0..5 {
            let i = segment as f32;
            let radius = (1.4 - i * 0.23) * s;
            parent = landmark(
                body,
                parent,
                &format!("v2_shade_tendril_{chain}_{segment}"),
                "TENTACLE",
                [
                    torso.x - torso.rx * 0.4 - i * 2.7 * s,
                    torso.y + ((chain as f32 - 1.0) * 1.6 + (i * 0.7).sin() * 1.5) * s,
                    lateral * (2.5 + i * 0.7) * s,
                ],
                [radius, radius, radius],
                "SPECTRAL",
            );
            body.nodes[parent].density = 0.12;
        }
    }
}

fn recompute_mass(body: &mut Body) {
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

/// Apply modest, coherent shape changes across all families, then defining
/// species landmarks. Explicit modifiers and counts remain on the same graph.
pub fn refine(spec: &MonsterSpec, body: &mut Body, _seed: u64) {
    if body.nodes.is_empty() {
        return;
    }
    if super::encounter_visuals::refine(spec, body) {
        super::encounter_visuals::vary_rest_shape(body, _seed);
        recompute_mass(body);
        return;
    }
    // Composite mounts retain their separately built anatomy and attachment graph.
    // Do not reshape the mount as though it were the rider's species.
    if spec.mount.is_some() {
        remove_unrequested_mounted_hybrids(spec, body);
        species_anatomy::mounted_modifiers(spec, body, crate::anatomy::morphology_scale(spec.size));
        shade_tendrils(spec, body, crate::anatomy::morphology_scale(spec.size));
        recompute_mass(body);
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
    humanoid_hybrid(spec, body, s);
    composite_hybrid(spec, body, s);
    match spec.affinity.as_str() {
        "FROG" | "TOAD" => frog(spec, body, s),
        "MERMAID" | "MERMAN" => mermaid(spec, body, s),
        "TREE_MONSTER" => tree(body, s),
        _ => {}
    }
    species_anatomy::refine(spec, body, _seed, s);
    extra_tail(spec, body, s);
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
    super::hybrid::assemble(spec, body, _seed);
    explicit_appendages::refine(spec, body, s);
    skeletal_spurs(spec, body, _seed, s);
    shade_tendrils(spec, body, s);
    // Recompute the same 2D mass convention used by V1 after shape changes.
    recompute_mass(body);
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
    fn mounted_identity_keeps_exact_requested_heads_across_seeds() {
        let mut witnessed_legacy_hybrid = false;
        for (prompt, heads, hands) in [
            ("orc riding lion", 2, 2),
            (
                "three-headed eight-armed orc riding three-headed eight-armed dinosaur",
                6,
                16,
            ),
            ("orc riding wolf riding lion", 3, 2),
            ("orc riding pig riding wolf riding lion", 4, 2),
        ] {
            let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            crate::v2::prepare(&mut spec);
            for seed in 0..128 {
                let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(seed));
                witnessed_legacy_hybrid |= body.nodes.iter().any(|n| n.id.contains("hybrid_"));
                refine(&spec, &mut body, seed);
                assert!(
                    !body.nodes.iter().any(|n| n.id.contains("hybrid_")),
                    "{prompt}, seed {seed}"
                );
                assert_eq!(
                    body.nodes.iter().filter(|n| n.kind == "HEAD").count(),
                    heads,
                    "{prompt}, seed {seed}"
                );
                assert_eq!(
                    body.nodes.iter().filter(|n| n.kind == "HAND").count(),
                    hands,
                    "{prompt}, seed {seed}"
                );
                assert!(body
                    .nodes
                    .iter()
                    .enumerate()
                    .all(|(i, n)| n.parent.is_none_or(|parent| parent < i)));
            }
        }
        assert!(
            witnessed_legacy_hybrid,
            "seed sweep must exercise the legacy random hybrid path"
        );
    }

    #[test]
    fn mounted_identity_preserves_explicit_hybrid_parts_on_their_owner() {
        for prompt in [
            "orc riding lion with goat",
            "orc riding wolf riding lion with goat",
        ] {
            let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
            crate::v2::prepare(&mut spec);
            for seed in 0..128 {
                let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(seed));
                refine(&spec, &mut body, seed);
                assert!(
                    body.nodes.iter().any(|n| n.id == "hybrid_head_0"),
                    "{prompt}, seed {seed}"
                );
                assert_eq!(
                    body.nodes
                        .iter()
                        .filter(|n| n.id.contains("hybrid_head_"))
                        .count(),
                    1,
                    "{prompt}, seed {seed}"
                );
                assert!(
                    body.nodes.iter().any(|n| n.id == "hybrid_horn_0"),
                    "{prompt}, seed {seed}"
                );
                assert!(body
                    .nodes
                    .iter()
                    .enumerate()
                    .all(|(i, n)| n.parent.is_none_or(|parent| parent < i)));
            }
        }
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
    fn seeded(prompt: &str, seed: u64) -> (MonsterSpec, Body) {
        let mut spec = crate::parser::parse_vocabulary(prompt);
        spec.features.push("PROFILE_V2".into());
        prepare_spec(&mut spec);
        prepare_seeded(&mut spec, seed);
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(seed));
        refine(&spec, &mut body, seed);
        (spec, body)
    }
    #[test]
    fn topology_novelty_is_occasional_seeded_and_bounded() {
        let mut arms = 0;
        let mut tails = 0;
        for seed in 0..128 {
            let (spec, body) = seeded("mermaid", seed);
            let (again, copy) = seeded("mermaid", seed);
            assert_eq!(spec.arm_count, again.arm_count);
            assert_eq!(format!("{body:?}"), format!("{copy:?}"));
            assert_eq!(
                body.nodes.iter().filter(|n| n.kind == "HAND").count(),
                spec.arm_count as usize
            );
            if has(&spec, "FOUR_ARMED") {
                arms += 1;
                assert_eq!(spec.arm_count, 4);
            }
            if has(&spec, "EXTRA_TAIL") {
                tails += 1;
                assert_eq!(
                    body.nodes
                        .iter()
                        .filter(|n| n.id.starts_with("v2_extra_tail_") && n.kind == "TAIL")
                        .count(),
                    5
                );
                assert_eq!(
                    body.nodes
                        .iter()
                        .filter(|n| n.id.starts_with("v2_extra_tail_") && n.kind == "FIN")
                        .count(),
                    2
                );
            }
            let mut ids = std::collections::HashSet::new();
            for (i, n) in body.nodes.iter().enumerate() {
                assert!(ids.insert(&n.id));
                assert!(n.parent.map(|p| p < i).unwrap_or(i == 0));
                assert!(n.x.is_finite() && n.y.is_finite() && n.rx > 0.0 && n.ry > 0.0);
            }
            assert!(body.nodes.len() < 100);
        }
        assert!((4..40).contains(&arms), "four-arm samples: {arms}");
        assert!((6..48).contains(&tails), "extra-tail samples: {tails}");
    }
    #[test]
    fn explicit_counts_and_legacy_specs_never_receive_random_topology() {
        for seed in 0..128 {
            let mut spec = crate::parser::parse_vocabulary("mermaid");
            let original = format!("{spec:?}");
            prepare_seeded(&mut spec, seed);
            assert_eq!(original, format!("{spec:?}"), "V1 changed");
            spec.features
                .extend(["PROFILE_V2", "EXPLICIT_ARMS", "EXPLICIT_TAILS"].map(str::to_owned));
            prepare_seeded(&mut spec, seed);
            assert_eq!(spec.arm_count, 2);
            assert!(!has(&spec, "FOUR_ARMED") && !has(&spec, "EXTRA_TAIL"));
        }
    }
    #[test]
    fn companion_topology_lock_preserves_native_counts_for_all_seeds() {
        for seed in 0..128 {
            let mut spec = crate::parser::parse_vocabulary("orc");
            spec.features
                .extend(["PROFILE_V2", "TOPOLOGY_LOCKED_INTERNAL"].map(str::to_owned));
            let original = format!("{spec:?}");
            prepare_seeded(&mut spec, seed);
            assert_eq!(original, format!("{spec:?}"));
        }
    }
    #[test]
    fn humanoid_hybrids_keep_native_landmarks_and_functional_rigs() {
        for prompt in [
            "harpy", "siren", "garuda", "satyr", "faun", "minotaur", "anubis", "sobek", "mermaid",
            "merman", "goblin", "asura",
        ] {
            let (spec, body) = seeded(prompt, 19);
            if spec.limb_count > 0 && spec.affinity != "CENTAUR" {
                assert!(index(&body, "v2_pelvis").is_some(), "{prompt}");
            }
            if matches!(spec.affinity.as_str(), "HARPY" | "SIREN" | "GARUDA") {
                assert_eq!(body.nodes.iter().filter(|n| n.kind == "WING").count(), 2);
                assert!(body.nodes[index(&body, "wing_0").unwrap()].z.unwrap() > 0.0);
                assert!(body.nodes[index(&body, "wing_1").unwrap()].z.unwrap() < 0.0);
            }
            let (_, _, _, poses) = crate::animation::make(&spec, &body);
            for pose in poses {
                assert_eq!(pose.physical_positions.len(), body.nodes.len());
                assert!(
                    pose.physical_positions
                        .iter()
                        .all(|(x, y, a)| x.is_finite() && y.is_finite() && a.is_finite()),
                    "{prompt}"
                );
            }
        }
    }
    #[test]
    fn mutated_merfolk_tails_remain_connected_through_swimming() {
        let seed = (0..128)
            .find(|&seed| has(&seeded("mermaid", seed).0, "EXTRA_TAIL"))
            .unwrap();
        let (spec, body) = seeded("mermaid", seed);
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        for pose in poses
            .iter()
            .filter(|p| matches!(p.clip_id.as_str(), "move" | "fast_move" | "water_hover"))
        {
            for (i, n) in body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.id.starts_with("v2_extra_tail_"))
            {
                let parent = n.parent.unwrap();
                let rest = (n.x - body.nodes[parent].x).hypot(n.y - body.nodes[parent].y);
                let a = pose.physical_positions[i];
                let b = pose.physical_positions[parent];
                assert!(((a.0 - b.0).hypot(a.1 - b.1) - rest).abs() < 0.01);
            }
        }
    }
    #[test]
    fn tree_and_named_humanoids_can_mutate_without_losing_identity() {
        let (spec, tree) = seeded("tree monster", 1);
        assert!(has(&spec, "FOUR_ARMED"));
        assert_eq!(tree.nodes.iter().filter(|n| n.kind == "HAND").count(), 4);
        assert_eq!(
            tree.nodes
                .iter()
                .filter(|n| n.id.starts_with("canopy_"))
                .count(),
            5
        );
        assert!(!has(&spec, "EXTRA_TAIL"));
        for prompt in ["anime girl", "hitler"] {
            assert!(
                (0..128).any(|seed| has(&seeded(prompt, seed).0, "FOUR_ARMED")),
                "{prompt}"
            );
        }
    }
    #[test]
    fn jumbled_skeleton_bones_are_bounded_attached_and_seeded() {
        let (_, a) = seeded("skeleton", 13);
        let (_, b) = seeded("skeleton", 13);
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let spurs: Vec<_> = a.nodes.iter().filter(|n| n.kind == "BONE_SPUR").collect();
        assert!((1..=4).contains(&spurs.len()));
        assert!(spurs
            .iter()
            .all(|n| n.parent.is_some() && n.material == "BONE" && n.rx > 0.0));
        assert_eq!(a.nodes.iter().filter(|n| n.kind == "FOOT").count(), 2);
    }
    #[test]
    fn merfolk_and_bird_humanoids_keep_a_slender_baseline() {
        for prompt in ["mermaid", "merman", "harpy", "siren"] {
            let (_, body) = build(prompt);
            let torso = &body.nodes[index(&body, "torso").unwrap()];
            assert!(torso.rx / torso.ry < 0.95, "{prompt}: broad baseline");
        }
    }
    #[test]
    fn shade_has_three_connected_tendrils_for_every_body_family() {
        for prompt in [
            "goblin",
            "wolf",
            "spider",
            "snake",
            "mermaid",
            "tree monster",
            "ghost",
            "slime",
            "tank",
        ] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            spec.features.extend(["PROFILE_V2".into(), "SHADE".into()]);
            prepare_spec(&mut spec);
            let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(7));
            refine(&spec, &mut body, 7);
            assert_eq!(
                body.nodes
                    .iter()
                    .filter(|n| n.id.starts_with("v2_shade_tendril_"))
                    .count(),
                15,
                "{prompt}"
            );
            for chain in 0..3 {
                for segment in 0..5 {
                    let i = index(&body, &format!("v2_shade_tendril_{chain}_{segment}")).unwrap();
                    assert_eq!(
                        body.nodes[i].parent,
                        Some(if segment == 0 {
                            0
                        } else {
                            index(&body, &format!("v2_shade_tendril_{chain}_{}", segment - 1))
                                .unwrap()
                        })
                    );
                }
            }
        }
    }
}
