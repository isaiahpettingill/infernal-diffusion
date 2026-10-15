//! V2 dinosaur rest poses. Preserve the native attachment graph and exact
//! topology while giving each family its characteristic skeletal proportions.
use super::{has, index, landmark, move_branch, remove_where, Body, MonsterSpec};

fn place(body: &mut Body, id: &str, p: [f32; 3], r: [f32; 3], s: f32) {
    if let Some(i) = index(body, id) {
        move_branch(body, i, p[0] * s, p[1] * s, p[2] * s);
        let n = &mut body.nodes[i];
        n.rx = r[0] * s;
        n.ry = r[1] * s;
        n.rz = Some(r[2] * s);
        n.angle = 0.0;
    }
}

pub(super) fn refine(spec: &MonsterSpec, body: &mut Body, s: f32) {
    let family = match spec.affinity.as_str() {
        "TREX" => 0,
        "RAPTOR" => 1,
        "LONGNECK_DINO" => 2,
        "STEGO" => 3,
        "TRICERATOPS" => 4,
        "ANKYLOSAUR" => 5,
        _ => return,
    };
    let theropod = family < 2;
    let raptor = family == 1;
    let longneck = family == 2;
    let (torso_r, torso_y) = match family {
        0 => ([14.5, 8.6, 7.0], -3.1),
        1 => ([10.8, 5.8, 4.2], -3.0),
        2 => ([16.4, 9.0, 8.0], -3.6),
        3 => ([14.6, 7.1, 6.7], -1.9),
        4 => ([14.4, 7.6, 7.6], -2.8),
        _ => ([14.1, 5.1, 7.8], -1.7),
    };
    place(body, "torso", [0.0, torso_y, 0.0], torso_r, s);
    // Keep explicit material recipes (metal, ice, etc.) and seeded palette
    // choices intact. Species form is independent of surface color.
    for h in 0..spec.heads.min(5) {
        let side = h as f32 - (spec.heads.min(5) as f32 - 1.0) * 0.5;
        let separation = if spec.heads > 1 { 6.6 } else { 0.0 };
        let z = side * separation;
        let count_scale = if spec.heads > 1 { 0.86 } else { 1.0 };
        let (neck_p, neck_r, head_p, head_r) = match family {
            0 => (
                [10.0, -6.0, z * 0.55],
                [5.8, 5.6, 3.7],
                [17.2, -10.8, z],
                [8.8, 5.9, 4.8],
            ),
            1 => (
                [8.1, -5.6, z * 0.55],
                [4.6, 4.6, 2.3],
                [14.3, -9.4, z],
                [6.2, 3.6, 3.2],
            ),
            2 => (
                [12.6, -17.7, z * 0.55],
                [4.6, 13.6, 3.8],
                [16.1, -32.3, z],
                [4.1, 2.8, 2.7],
            ),
            3 => (
                [12.6, -0.1, z * 0.55],
                [4.1, 2.7, 2.8],
                [17.5, 0.2, z],
                [3.3, 2.4, 2.3],
            ),
            4 => (
                [12.0, -3.8, z * 0.55],
                [4.8, 4.0, 4.4],
                [17.0, -4.6, z],
                [6.8, 5.3, 5.6],
            ),
            _ => (
                [12.6, -1.3, z * 0.55],
                [3.6, 2.5, 3.9],
                [17.3, -1.9, z],
                [4.2, 2.6, 4.0],
            ),
        };
        place(body, &format!("neck_{h}"), neck_p, neck_r, s);
        place(
            body,
            &format!("head_{h}"),
            head_p,
            head_r.map(|r| r * count_scale),
            s,
        );
    }

    // Dinosaur hips sit under the rear of a horizontal trunk. Native legacy
    // theropods used left/right x offsets like a human, producing upright legs.
    for i in 0..spec.limb_count.min(12) {
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        let rank = (i / 2) as f32;
        let lane = side
            * (if theropod {
                if raptor {
                    3.4
                } else {
                    5.0
                }
            } else {
                torso_r[2] * 0.76
            } + rank * 0.28);
        let (hip, knee, ankle, foot, thick) = if theropod {
            let x = -4.5 - rank * 3.2;
            (
                [x - 1.1, -0.6, lane],
                [x + 2.4, if raptor { 4.0 } else { 4.8 }, lane * 1.06],
                [x - 0.8, if raptor { 9.6 } else { 11.3 }, lane * 1.08],
                [x + 1.4, if raptor { 13.1 } else { 15.2 }, lane * 1.10],
                if raptor { 2.35 } else { 3.55 },
            )
        } else {
            let front = rank == 0.0;
            let x = if front {
                9.0
            } else {
                -8.4 - (rank - 1.0) * 3.5
            };
            let short_front = family == 3 && front;
            let ground = if longneck {
                15.0
            } else if family == 5 {
                9.2
            } else {
                12.0
            };
            (
                [x, if short_front { 1.0 } else { 0.1 }, lane],
                [
                    x + if front { -0.5 } else { 1.3 },
                    ground * 0.40,
                    lane * 1.04,
                ],
                [
                    x + if front { 0.3 } else { -0.7 },
                    ground * 0.72,
                    lane * 1.03,
                ],
                [x + 1.1, ground, lane * 1.06],
                if longneck {
                    3.3
                } else if family == 5 {
                    2.2
                } else {
                    2.65
                },
            )
        };
        place(
            body,
            &format!("leg_root_{i}"),
            hip,
            [thick * 1.16, thick * 1.22, thick * 0.88],
            s,
        );
        place(
            body,
            &format!("limb_{i}"),
            knee,
            [
                thick,
                if theropod { thick * 1.65 } else { thick * 1.35 },
                thick * 0.88,
            ],
            s,
        );
        place(
            body,
            &format!("shin_{i}"),
            ankle,
            [
                thick * 0.53,
                if theropod { thick * 1.12 } else { thick * 1.15 },
                thick * 0.54,
            ],
            s,
        );
        place(
            body,
            &format!("foot_{i}"),
            foot,
            [
                if theropod { thick * 1.06 } else { thick * 0.88 },
                if theropod { 1.45 } else { 1.70 },
                thick * 0.79,
            ],
            s,
        );
    }

    if theropod {
        // Two small, forward-facing arms remain recognizable. Additional pairs
        // retain their own chains; the explicit-appendage pass fans them out.
        for i in 0..spec.arm_count.min(8) {
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let rank = (i / 2) as f32;
            let z = side * (if raptor { 3.6 } else { 4.6 });
            let x = if raptor { 5.9 } else { 7.8 } - rank * 1.4;
            let y = -4.5 + rank * 1.6;
            let reach = if raptor { 3.4 } else { 2.1 };
            place(
                body,
                &format!("shoulder_{i}"),
                [x, y, z],
                [1.2, 1.4, 1.0],
                s,
            );
            place(
                body,
                &format!("arm_{i}"),
                [x - 0.7, y + reach, z * 1.23],
                [0.95, 1.65, 0.82],
                s,
            );
            place(
                body,
                &format!("hand_{i}"),
                [x + reach * 0.85, y + reach * 1.35, z * 1.32],
                [if raptor { 1.35 } else { 1.04 }, 1.1, 1.0],
                s,
            );
        }
        // Keep native claw IDs as small collision/attack landmarks at the
        // authored digits. The V2 renderer hides their old single-hook surface.
        for i in 0..body.nodes.len() {
            if body.nodes[i].id.starts_with("claw_") && body.nodes[i].kind == "CLAW" {
                if let Some(parent) = body.nodes[i].parent {
                    let p = body.nodes[parent].clone();
                    let n = &mut body.nodes[i];
                    n.x = p.x + p.rx;
                    n.y = p.y + p.ry * 0.3;
                    n.z = p.z;
                    n.rx = 0.50 * s;
                    n.ry = 0.60 * s;
                    n.rz = Some(0.40 * s);
                }
            }
        }
    }

    if let Some(tail) = index(body, "tail") {
        let (center, radius) = match family {
            0 => ([-18.6, -3.1, 0.0], [13.4, 5.2, 5.0]),
            1 => ([-16.5, -3.1, 0.0], [11.2, 3.2, 3.0]),
            2 => ([-19.2, -2.4, 0.0], [12.9, 5.2, 4.8]),
            3 => ([-19.0, -0.4, 0.0], [11.4, 3.9, 3.6]),
            4 => ([-16.3, -0.7, 0.0], [8.2, 4.0, 3.6]),
            _ => ([-18.0, -0.5, 0.0], [10.3, 3.0, 2.8]),
        };
        place(body, "tail", center, radius, s);
        if family == 3 {
            for row in 0..2 {
                for (j, side) in [-1.0, 1.0].into_iter().enumerate() {
                    landmark(
                        body,
                        tail,
                        &format!("v2_stego_tail_spike_{row}_{j}"),
                        "HORN",
                        [(-26.0 - row as f32 * 3.0) * s, -0.5 * s, side * 0.6 * s],
                        [1.7 * s, 2.9 * s, 1.7 * s],
                        "HORN",
                    );
                }
            }
        }
        if family == 5 {
            place(body, "tail_club", [-29.5, -1.0, 0.0], [3.4, 2.6, 3.6], s);
        }
    }
    if family == 3 {
        for (i, (x, y, h)) in [
            (-12.3, -9.2, 3.8),
            (-7.8, -11.0, 5.3),
            (-3.0, -12.2, 6.9),
            (2.0, -11.2, 6.2),
            (6.4, -8.8, 4.8),
            (10.0, -6.0, 3.1),
        ]
        .into_iter()
        .enumerate()
        {
            place(
                body,
                &format!("dorsal_plate_{i}"),
                [x, y, if i % 2 == 0 { -0.85 } else { 0.85 }],
                [2.65, h, 0.7],
                s,
            );
        }
    }
    if family == 5 {
        place(
            body,
            "armored_carapace",
            [-0.2, -5.0, 0.0],
            [13.4, 4.5, 8.0],
            s,
        );
        // Distributed osteoderms and lateral spikes belong to the shell mesh;
        // the old four giant vertical horns made this look like a spiked turtle.
        remove_where(body, |n| n.id.starts_with("armor_spike_"));
    }
    if family == 4 {
        triceratops_heads(spec, body, s);
    }
}

fn triceratops_heads(spec: &MonsterSpec, body: &mut Body, s: f32) {
    // Rebuild the native landmarks for every requested head, with each shield
    // and horn parented to its own skull. No head_0-only special case survives.
    remove_where(body, |n| {
        n.id.starts_with("triceratops_horn_") || n.id.starts_with("head_frill")
    });
    for h in 0..spec.heads.min(5) {
        let Some(head) = index(body, &format!("head_{h}")) else {
            continue;
        };
        let n = body.nodes[head].clone();
        let z = n.z.unwrap_or(0.0);
        if has(spec, "TRIPLE_HORN") {
            for i in 0..3 {
                let id = if h == 0 {
                    format!("triceratops_horn_{i}")
                } else {
                    format!("triceratops_horn_{h}_{i}")
                };
                let brow = i < 2;
                landmark(
                    body,
                    head,
                    &id,
                    "HORN",
                    [
                        n.x + if brow { -0.8 * s } else { 4.1 * s },
                        n.y - if brow { 2.3 * s } else { 0.1 * s },
                        z + if i == 0 {
                            -2.8 * s
                        } else if i == 1 {
                            2.8 * s
                        } else {
                            0.0
                        },
                    ],
                    [
                        if brow { 4.0 * s } else { 1.4 * s },
                        if brow { 4.0 * s } else { 2.5 * s },
                        0.9 * s,
                    ],
                    "HORN",
                );
            }
        }
        if has(spec, "FRILL") {
            let id = if h == 0 {
                "head_frill".to_string()
            } else {
                format!("head_frill_{h}")
            };
            landmark(
                body,
                head,
                &id,
                "SHELL",
                [n.x - 4.1 * s, n.y - 3.0 * s, z],
                [3.5 * s, 8.2 * s, 6.8 * s],
                &n.material,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    fn build(prompt: &str) -> (MonsterSpec, Body) {
        let mut spec = crate::v2::interpret(crate::parser::parse_vocabulary(prompt), prompt);
        crate::v2::prepare(&mut spec);
        let mut body =
            crate::anatomy::build(&spec, &mut rand_chacha::ChaCha8Rng::seed_from_u64(42));
        crate::v2::anatomy::refine(&spec, &mut body, 42);
        (spec, body)
    }
    #[test]
    fn tyrannosaur_has_deep_skull_horizontal_hips_and_counterbalancing_tail() {
        let (_, b) = build("t rex");
        let head = &b.nodes[index(&b, "head_0").unwrap()];
        let torso = &b.nodes[index(&b, "torso").unwrap()];
        let tail = &b.nodes[index(&b, "tail").unwrap()];
        assert!(head.ry / head.rx > 0.55);
        assert!(torso.rx > torso.ry * 1.6);
        assert!(tail.x - tail.rx < -torso.rx * 2.0 && tail.x < -torso.rx);
        let hip = &b.nodes[index(&b, "leg_root_0").unwrap()];
        let knee = &b.nodes[index(&b, "limb_0").unwrap()];
        let ankle = &b.nodes[index(&b, "shin_0").unwrap()];
        assert!(hip.x < 0.0 && knee.x > hip.x && ankle.x < knee.x);
    }
    #[test]
    fn triceratops_landmarks_follow_every_requested_head() {
        let (_, b) = build("triceratops with three heads");
        let heads: Vec<_> = b
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "HEAD")
            .collect();
        assert_eq!(heads.len(), 3);
        for (h, _) in heads {
            assert_eq!(
                b.nodes
                    .iter()
                    .filter(|n| n.parent == Some(h) && n.id.starts_with("triceratops_horn_"))
                    .count(),
                3
            );
            assert_eq!(
                b.nodes
                    .iter()
                    .filter(|n| n.parent == Some(h) && n.id.starts_with("head_frill"))
                    .count(),
                1
            );
        }
    }
    #[test]
    fn dinosaur_families_keep_leg_counts_and_distinct_silhouettes() {
        for prompt in [
            "t rex",
            "velociraptor",
            "brachiosaurus",
            "stegosaurus",
            "triceratops",
            "ankylosaurus",
        ] {
            let (spec, b) = build(prompt);
            assert_eq!(
                b.nodes.iter().filter(|n| n.kind == "FOOT").count(),
                spec.limb_count as usize
            );
            assert!(b
                .nodes
                .iter()
                .enumerate()
                .all(|(i, n)| n.parent.is_none_or(|p| p < i)));
            assert!(b
                .nodes
                .iter()
                .all(|n| n.rx > 0.0 && n.ry > 0.0 && n.z.unwrap_or(0.0).is_finite()));
        }
        let (_, stego) = build("stegosaurus");
        assert_eq!(
            stego
                .nodes
                .iter()
                .filter(|n| n.id.starts_with("v2_stego_tail_spike_"))
                .count(),
            4
        );
    }
}
