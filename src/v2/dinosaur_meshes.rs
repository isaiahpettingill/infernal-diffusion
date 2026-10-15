//! Purpose-built dinosaur surfaces. A broad maxilla, open tooth-bearing jaw,
//! hip-heavy trunk and tapered counterweight replace the generic lizard forms.
//! All geometry is local to existing rig nodes, including duplicated heads.
use super::{blob, empty, eyes, lip, loft, sweep, MeshAsset};
use std::{f32::consts::PI, sync::OnceLock};

fn theropod_head(raptor: bool) -> MeshAsset {
    let mut m = empty();
    let breadth = if raptor { 0.74 } else { 1.0 };
    let stations: &[[f32; 4]] = if raptor {
        &[
            [-0.99, 0.12, -0.25, 0.23],
            [-0.72, 0.59, -0.38, 0.62],
            [-0.39, 0.71, -0.32, 0.75],
            [-0.03, 0.48, -0.19, 0.61],
            [0.44, 0.33, -0.16, 0.45],
            [0.99, 0.20, -0.13, 0.29],
            [1.24, 0.11, -0.14, 0.21],
        ]
    } else {
        &[
            [-1.00, 0.19, -0.31, 0.25],
            [-0.79, 0.64, -0.47, 0.63],
            [-0.49, 0.84, -0.38, 0.83],
            [-0.14, 0.74, -0.19, 0.75],
            [0.26, 0.57, -0.16, 0.64],
            [0.75, 0.46, -0.12, 0.59],
            [1.06, 0.35, -0.12, 0.48],
            [1.14, 0.21, -0.10, 0.37],
        ]
    };
    loft(&mut m, stations, "base");
    // The dark oral cavity is recessed behind two genuinely separated jaws.
    // Teeth project into empty space instead of being buried in a closed snout.
    loft(
        &mut m,
        &[
            [-0.54, -0.20, -0.59, 0.43],
            [-0.06, -0.15, -0.66, 0.46],
            [0.49, -0.13, -0.73, 0.38],
            [1.05, -0.13, -0.67, 0.22],
        ],
        "mouth",
    );
    loft(
        &mut m,
        &[
            [-0.78, -0.30, -0.57, 0.54],
            [-0.50, -0.43, -0.82, 0.67],
            [-0.04, -0.62, -0.95, 0.59],
            [0.46, -0.73, -0.98, 0.48],
            [0.97, -0.68, -0.87, 0.34],
            [1.09, -0.59, -0.78, 0.22],
        ],
        "jaw",
    );
    for side in [-1.0, 1.0] {
        // Heavy supraorbital ridge and rear jaw muscle are continuous with the
        // skull, giving the tyrannosaur its square, powerful profile.
        sweep(
            &mut m,
            &[
                ([-0.57, 0.62, side * 0.67], 0.16),
                ([-0.26, 0.66, side * 0.75], 0.17),
                ([-0.01, 0.51, side * 0.68], 0.09),
            ],
            "brow",
        );
        blob(
            &mut m,
            [-0.66, -0.12, side * 0.58],
            [0.23, 0.37, 0.22],
            "base",
        );
        blob(
            &mut m,
            [0.87, 0.22, side * 0.48],
            [0.10, 0.065, 0.026],
            "nose",
        );
        let count = if raptor { 9 } else { 8 };
        for i in 0..count {
            let t = i as f32 / (count - 1) as f32;
            let x = -0.10 + t * 1.10;
            let z = side * (0.64 - t * 0.25);
            let length = if raptor {
                0.22
            } else {
                0.29 + 0.09 * (t * PI).sin()
            };
            sweep(
                &mut m,
                &[
                    ([x, -0.13, z], if raptor { 0.043 } else { 0.060 }),
                    ([x - 0.01, -0.13 - length * 0.65, z], 0.030),
                    ([x - 0.045, -0.13 - length, z - side * 0.025], 0.003),
                ],
                "tooth",
            );
            if i < count - 1 {
                let lower = -0.66 - 0.07 * (t * PI).sin();
                sweep(
                    &mut m,
                    &[
                        ([x + 0.045, lower, z * 0.86], 0.041),
                        ([x + 0.025, lower + length * 0.65, z * 0.86], 0.023),
                        ([x, lower + length * 0.82, z * 0.84], 0.002),
                    ],
                    "tooth",
                );
            }
        }
    }
    eyes(&mut m, -0.23, 0.40, 0.75, if raptor { 0.15 } else { 0.12 });
    for v in &mut m.vertices {
        v[2] *= breadth;
        if raptor {
            v[1] *= 0.86;
        }
    }
    m
}

fn herbivore_head(kind: &str) -> MeshAsset {
    let mut m = empty();
    let stations: &[[f32; 4]] = match kind {
        "sauropod" => &[
            [-0.92, 0.10, -0.21, 0.18],
            [-0.64, 0.56, -0.48, 0.48],
            [-0.16, 0.65, -0.54, 0.59],
            [0.29, 0.43, -0.51, 0.55],
            [0.77, 0.23, -0.43, 0.46],
            [1.14, 0.10, -0.32, 0.32],
        ],
        "stego" => &[
            [-0.95, 0.11, -0.18, 0.19],
            [-0.55, 0.59, -0.46, 0.47],
            [-0.10, 0.51, -0.51, 0.52],
            [0.43, 0.18, -0.39, 0.34],
            [0.93, -0.01, -0.25, 0.19],
            [1.17, -0.11, -0.25, 0.12],
        ],
        "ankylo" => &[
            [-0.98, 0.12, -0.21, 0.27],
            [-0.67, 0.55, -0.43, 0.75],
            [-0.18, 0.47, -0.51, 0.90],
            [0.37, 0.21, -0.45, 0.67],
            [0.83, 0.02, -0.32, 0.45],
            [1.09, -0.09, -0.27, 0.32],
        ],
        _ => &[
            [-0.95, 0.15, -0.25, 0.27],
            [-0.69, 0.64, -0.51, 0.67],
            [-0.21, 0.63, -0.64, 0.75],
            [0.23, 0.35, -0.55, 0.62],
            [0.63, 0.04, -0.45, 0.40],
            [1.00, -0.10, -0.35, 0.25],
        ],
    };
    loft(&mut m, stations, "base");
    loft(
        &mut m,
        &[
            [-0.10, -0.36, -0.65, 0.50],
            [0.56, -0.36, -0.56, 0.37],
            [1.01, -0.30, -0.42, 0.22],
        ],
        "jaw",
    );
    if kind != "sauropod" {
        sweep(
            &mut m,
            &[
                ([0.87, -0.08, 0.0], 0.26),
                ([1.16, -0.25, 0.0], 0.18),
                ([1.13, -0.50, 0.0], 0.014),
            ],
            "beak",
        );
    }
    eyes(
        &mut m,
        -0.03,
        0.27,
        if kind == "ankylo" { 0.78 } else { 0.61 },
        0.12,
    );
    lip(&mut m, 0.23, 1.02, -0.36, 0.49, 0.23);
    for side in [-1.0, 1.0] {
        blob(
            &mut m,
            [0.77, 0.06, side * 0.35],
            [0.08, 0.045, 0.034],
            "nose",
        );
        if kind == "ankylo" {
            sweep(
                &mut m,
                &[
                    ([-0.60, 0.12, side * 0.67], 0.22),
                    ([-1.00, 0.21, side * 0.94], 0.13),
                    ([-1.28, 0.31, side * 1.03], 0.003),
                ],
                "horn",
            );
        }
    }
    m
}

fn torso(kind: &str) -> MeshAsset {
    let mut m = empty();
    let stations: &[[f32; 4]] = match kind {
        "trex" | "raptor" => &[
            [-1.05, 0.12, -0.12, 0.19],
            [-0.85, 0.53, -0.46, 0.55],
            [-0.55, 0.83, -0.71, 0.91],
            [-0.22, 0.83, -0.84, 1.0],
            [0.19, 0.68, -0.87, 0.90],
            [0.53, 0.57, -0.73, 0.69],
            [0.83, 0.50, -0.42, 0.46],
            [1.02, 0.31, -0.17, 0.25],
        ],
        "sauropod" => &[
            [-1.04, 0.08, -0.16, 0.23],
            [-0.78, 0.70, -0.67, 0.75],
            [-0.39, 0.99, -0.96, 1.0],
            [0.10, 0.93, -1.02, 0.98],
            [0.53, 0.83, -0.91, 0.85],
            [0.85, 0.66, -0.52, 0.57],
            [1.04, 0.37, -0.19, 0.29],
        ],
        "stego" => &[
            [-1.05, 0.15, -0.09, 0.22],
            [-0.80, 0.67, -0.55, 0.62],
            [-0.42, 1.09, -0.93, 0.94],
            [-0.05, 1.02, -0.98, 1.0],
            [0.38, 0.70, -0.85, 0.84],
            [0.75, 0.26, -0.51, 0.56],
            [1.04, 0.03, -0.23, 0.27],
        ],
        "ankylo" => &[
            [-1.03, 0.10, -0.17, 0.22],
            [-0.80, 0.56, -0.61, 0.70],
            [-0.41, 0.87, -0.92, 1.0],
            [0.10, 0.91, -1.00, 1.06],
            [0.54, 0.73, -0.84, 0.91],
            [0.87, 0.37, -0.46, 0.56],
            [1.04, 0.12, -0.18, 0.25],
        ],
        _ => &[
            [-1.05, 0.08, -0.12, 0.20],
            [-0.80, 0.61, -0.61, 0.69],
            [-0.43, 0.85, -0.90, 0.91],
            [0.02, 0.84, -1.0, 1.0],
            [0.45, 0.94, -0.91, 0.93],
            [0.80, 0.64, -0.61, 0.68],
            [1.03, 0.24, -0.21, 0.29],
        ],
    };
    loft(&mut m, stations, "base");
    m
}

fn neck(long: bool) -> MeshAsset {
    let mut m = empty();
    if long {
        sweep(
            &mut m,
            &[
                ([-0.31, -1.03, 0.0], 0.94),
                ([-0.29, -0.63, 0.0], 0.76),
                ([-0.03, -0.17, 0.0], 0.55),
                ([0.20, 0.35, 0.0], 0.40),
                ([0.31, 0.83, 0.0], 0.31),
                ([0.59, 1.09, 0.0], 0.30),
            ],
            "base",
        );
    } else {
        loft(
            &mut m,
            &[
                [-1.04, -0.15, -0.91, 0.74],
                [-0.65, 0.42, -0.80, 0.78],
                [-0.13, 0.82, -0.36, 0.65],
                [0.37, 0.95, -0.10, 0.51],
                [0.91, 0.74, -0.08, 0.37],
                [1.10, 0.47, 0.05, 0.24],
            ],
            "base",
        );
    }
    m
}

fn tail() -> MeshAsset {
    let mut m = empty();
    // The +X base enters the pelvis; the thin end is a long counterweight.
    loft(
        &mut m,
        &[
            [-1.15, 0.33, 0.31, 0.008],
            [-0.94, 0.31, 0.22, 0.055],
            [-0.70, 0.28, 0.04, 0.14],
            [-0.35, 0.35, -0.14, 0.27],
            [0.04, 0.48, -0.32, 0.43],
            [0.40, 0.68, -0.55, 0.64],
            [0.76, 0.86, -0.77, 0.83],
            [1.05, 0.91, -0.86, 0.92],
        ],
        "base",
    );
    m
}

fn foot(raptor: bool, columnar: bool) -> MeshAsset {
    let mut m = empty();
    if columnar {
        loft(
            &mut m,
            &[
                [-0.69, 0.48, -0.38, 0.47],
                [-0.19, 0.74, -0.46, 0.76],
                [0.40, 0.41, -0.49, 0.78],
                [0.84, 0.04, -0.43, 0.55],
            ],
            "base",
        );
        for z in [-0.48, 0.0, 0.48] {
            blob(&mut m, [0.62, -0.19, z], [0.27, 0.20, 0.20], "hoof");
        }
        return m;
    }
    sweep(
        &mut m,
        &[
            ([-0.39, 0.71, 0.0], 0.29),
            ([-0.20, 0.17, 0.0], 0.36),
            ([0.07, -0.20, 0.0], 0.34),
        ],
        "base",
    );
    for (i, z) in [-0.44, 0.0, 0.44].into_iter().enumerate() {
        let end = if i == 1 { 1.34 } else { 1.04 };
        sweep(
            &mut m,
            &[
                ([-0.02, -0.21, z * 0.42], 0.19),
                ([0.43, -0.31, z], 0.16),
                ([end - 0.22, -0.32, z * 1.32], 0.11),
            ],
            "base",
        );
        sweep(
            &mut m,
            &[
                ([end - 0.30, -0.30, z * 1.32], 0.105),
                ([end, -0.35, z * 1.34], 0.075),
                ([end + 0.11, -0.46, z * 1.34], 0.003),
            ],
            "horn",
        );
    }
    if raptor {
        // Raised recurved killing claw, a key silhouette beyond the narrow skull.
        sweep(
            &mut m,
            &[
                ([0.13, -0.05, -0.35], 0.15),
                ([0.31, 0.43, -0.52], 0.15),
                ([0.64, 0.65, -0.55], 0.12),
                ([0.89, 0.46, -0.57], 0.075),
                ([0.90, 0.06, -0.57], 0.003),
            ],
            "horn",
        );
    }
    m
}

fn hand(raptor: bool) -> MeshAsset {
    let mut m = empty();
    blob(&mut m, [-0.08, 0.14, 0.0], [0.40, 0.45, 0.39], "base");
    let count = if raptor { 3 } else { 2 };
    for i in 0..count {
        let z = (i as f32 - (count - 1) as f32 / 2.0) * 0.48;
        let length = if i == 1 { 1.03 } else { 0.82 };
        sweep(
            &mut m,
            &[
                ([0.08, -0.02, z * 0.7], 0.15),
                ([0.45, -0.26, z], 0.12),
                ([length, -0.42, z], 0.08),
            ],
            "base",
        );
        sweep(
            &mut m,
            &[
                ([length, -0.38, z], 0.09),
                ([length + 0.18, -0.55, z], 0.07),
                ([length + 0.06, -0.85, z], 0.003),
            ],
            "horn",
        );
    }
    m
}

fn armor() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-1.03, 0.04, -0.11, 0.23],
            [-0.72, 0.68, -0.17, 0.70],
            [-0.24, 0.99, -0.21, 0.99],
            [0.28, 0.93, -0.19, 1.03],
            [0.75, 0.59, -0.15, 0.74],
            [1.03, 0.09, -0.10, 0.29],
        ],
        "shell",
    );
    for x in [-0.66_f32, -0.28, 0.12, 0.52] {
        for z in [-0.72_f32, -0.35, 0.0, 0.35, 0.72] {
            let y = (1.0 - z.abs() * 0.60) * (1.0 - x.abs() * 0.32);
            blob(&mut m, [x, y, z], [0.14, 0.115, 0.12], "horn");
        }
        for side in [-1.0, 1.0] {
            sweep(
                &mut m,
                &[
                    ([x, 0.28, side * 0.84], 0.12),
                    ([x - 0.12, 0.29, side * 1.18], 0.09),
                    ([x - 0.28, 0.39, side * 1.38], 0.003),
                ],
                "horn",
            );
        }
    }
    m
}

fn club() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-1.05, 0.08, -0.10, 0.22],
            [-0.72, 0.72, -0.65, 0.75],
            [-0.18, 0.91, -0.82, 1.04],
            [0.43, 0.72, -0.66, 0.93],
            [0.97, 0.17, -0.14, 0.35],
        ],
        "horn",
    );
    m
}

fn tail_spike(side: f32) -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, 0.0, 0.0], 0.34),
            ([-0.58, 0.31, side * 0.70], 0.27),
            ([-1.29, 0.73, side * 1.31], 0.13),
            ([-1.90, 1.03, side * 1.64], 0.003),
        ],
        "horn",
    );
    m
}

pub(super) fn mesh(affinity: &str, kind: &str, node_id: &str) -> Option<&'static MeshAsset> {
    let family = match affinity {
        "TREX" => 0,
        "RAPTOR" => 1,
        "LONGNECK_DINO" => 2,
        "STEGO" => 3,
        "TRICERATOPS" => 4,
        "ANKYLOSAUR" => 5,
        _ => return None,
    };
    let i = match kind {
        "HEAD" => family,
        "TORSO" => 6 + family,
        "NECK" if family == 2 => 13,
        "NECK" if family < 2 => 12,
        "TAIL" => 14,
        "FOOT" if family < 2 => 15 + family,
        "FOOT" => 17,
        "HAND" if family < 2 => 18 + family,
        "SHELL" if family == 5 && node_id.contains("armored_carapace") => 20,
        "SHELL" if family == 5 && node_id.contains("tail_club") => 21,
        "HORN" if family == 3 && node_id.contains("v2_stego_tail_spike_") => {
            if node_id.ends_with("_0") {
                22
            } else {
                23
            }
        }
        _ => return None,
    };
    static MESHES: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    Some(
        &MESHES.get_or_init(|| {
            vec![
                theropod_head(false),
                theropod_head(true),
                herbivore_head("sauropod"),
                herbivore_head("stego"),
                herbivore_head("ceratops"),
                herbivore_head("ankylo"),
                torso("trex"),
                torso("raptor"),
                torso("sauropod"),
                torso("stego"),
                torso("ceratops"),
                torso("ankylo"),
                neck(false),
                neck(true),
                tail(),
                foot(false, false),
                foot(true, false),
                foot(false, true),
                hand(false),
                hand(true),
                armor(),
                club(),
                tail_spike(-1.0),
                tail_spike(1.0),
            ]
        })[i],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn each_dinosaur_has_distinct_head_body_and_all_native_heads_share_them() {
        let families = [
            "TREX",
            "RAPTOR",
            "LONGNECK_DINO",
            "STEGO",
            "TRICERATOPS",
            "ANKYLOSAUR",
        ];
        for (i, family) in families.iter().enumerate() {
            for kind in ["HEAD", "TORSO", "TAIL", "FOOT"] {
                let m = mesh(family, kind, "head_0").unwrap();
                assert!(m.vertices.iter().flatten().all(|v| v.is_finite()));
                assert!(m
                    .faces
                    .iter()
                    .all(|(a, b, c, _)| [a, b, c].iter().all(|i| **i < m.vertices.len())));
                assert!(std::ptr::eq(m, mesh(family, kind, "head_4").unwrap()));
            }
            for other in &families[i + 1..] {
                assert!(!std::ptr::eq(
                    mesh(family, "HEAD", "head_0").unwrap(),
                    mesh(other, "HEAD", "head_0").unwrap()
                ));
                assert!(!std::ptr::eq(
                    mesh(family, "TORSO", "torso").unwrap(),
                    mesh(other, "TORSO", "torso").unwrap()
                ));
            }
        }
    }
    #[test]
    fn theropods_have_separate_jaws_exposed_teeth_and_raptor_sickle_claw() {
        for family in ["TREX", "RAPTOR"] {
            let m = mesh(family, "HEAD", "head_2").unwrap();
            assert!(m.faces.iter().filter(|f| f.3 == "tooth").count() > 500);
            for role in ["mouth", "jaw", "brow", "eye", "nose"] {
                assert!(m.faces.iter().any(|f| f.3 == role));
            }
            let lower_jaw = m
                .faces
                .iter()
                .filter(|f| f.3 == "jaw")
                .map(|f| m.vertices[f.0][1])
                .fold(f32::INFINITY, f32::min);
            assert!(lower_jaw < -0.70);
        }
        let foot = mesh("RAPTOR", "FOOT", "foot_0").unwrap();
        assert!(foot
            .faces
            .iter()
            .filter(|f| f.3 == "horn")
            .any(|f| foot.vertices[f.0][1] > 0.5));
    }
}
