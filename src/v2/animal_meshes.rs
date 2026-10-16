//! Anatomically shaped, V2-only animal surfaces. +X is the muzzle, +Y is up.
//!
//! The large forms are continuous lofts, not stacked ellipsoids. Tiny eye,
//! nostril and cheek surfaces are attached to those forms. Native rig nodes own
//! horns, tusks, trunks, limbs and tails: these meshes never invent rig joints.
use super::{v2_append_blob, MeshAsset, V3};
use std::{f32::consts::TAU, sync::OnceLock};
#[path = "dinosaur_meshes.rs"]
mod dinosaurs;

fn empty() -> MeshAsset {
    MeshAsset {
        vertices: Vec::new(),
        faces: Vec::new(),
    }
}
fn p(v: [f32; 3]) -> V3 {
    V3::new(v[0], v[1], v[2])
}
fn blob(m: &mut MeshAsset, center: [f32; 3], radius: [f32; 3], role: &str) {
    v2_append_blob(m, p(center), p(radius), role);
}

/// Each station is (forward, upper surface, lower surface, half breadth).
/// Independent dorsal and ventral contours allow a real skull, tucked belly,
/// withers or jaw rather than stretching the same sphere for every species.
fn loft(m: &mut MeshAsset, stations: &[[f32; 4]], role: &str) {
    const N: usize = 16;
    let start = m.vertices.len();
    for &[x, top, bottom, width] in stations {
        let center = (top + bottom) * 0.5;
        let radius = (top - bottom) * 0.5;
        for j in 0..N {
            let t = TAU * j as f32 / N as f32;
            m.vertices
                .push([x, center + radius * t.cos(), width * t.sin()]);
        }
    }
    for ring in 0..stations.len() - 1 {
        for j in 0..N {
            let a = start + ring * N + j;
            let b = start + ring * N + (j + 1) % N;
            m.faces
                .extend([(a, b, a + N, role.into()), (b, b + N, a + N, role.into())]);
        }
    }
    for (ring, reverse) in [(0, true), (stations.len() - 1, false)] {
        let [x, top, bottom, _] = stations[ring];
        let center = m.vertices.len();
        m.vertices.push([x, (top + bottom) * 0.5, 0.0]);
        for j in 0..N {
            let a = start + ring * N + j;
            let b = start + ring * N + (j + 1) % N;
            m.faces.push(if reverse {
                (center, b, a, role.into())
            } else {
                (center, a, b, role.into())
            });
        }
    }
}

/// Closed, curved tapered tube. Used for features whose silhouette matters at
/// sprite scale: tusks, trunks, ruffs, hooked beaks, claws and tail tips.
fn sweep(m: &mut MeshAsset, path: &[([f32; 3], f32)], role: &str) {
    const N: usize = 10;
    let start = m.vertices.len();
    for (i, &(center, r)) in path.iter().enumerate() {
        let tangent =
            (p(path[(i + 1).min(path.len() - 1)].0) - p(path[i.saturating_sub(1)].0)).normalized();
        let reference = if tangent.y.abs() < 0.88 {
            V3::new(0.0, 1.0, 0.0)
        } else {
            V3::new(0.0, 0.0, 1.0)
        };
        let u = tangent.cross(reference).normalized();
        let v = tangent.cross(u).normalized();
        for j in 0..N {
            let t = TAU * j as f32 / N as f32;
            let q = p(center) + (u * t.cos() + v * t.sin()) * r;
            m.vertices.push([q.x, q.y, q.z]);
        }
    }
    for ring in 0..path.len() - 1 {
        for j in 0..N {
            let a = start + ring * N + j;
            let b = start + ring * N + (j + 1) % N;
            m.faces
                .extend([(a, b, a + N, role.into()), (b, b + N, a + N, role.into())]);
        }
    }
    for (ring, reverse) in [(0, true), (path.len() - 1, false)] {
        let center = m.vertices.len();
        m.vertices.push(path[ring].0);
        for j in 0..N {
            let a = start + ring * N + j;
            let b = start + ring * N + (j + 1) % N;
            m.faces.push(if reverse {
                (center, b, a, role.into())
            } else {
                (center, a, b, role.into())
            });
        }
    }
}

/// A thick, folded pinna with a front-facing dark inner triangle. This remains
/// pointed in both side and front views, unlike a thin silhouette card.
fn pointed_ear(m: &mut MeshAsset, x: f32, z: f32, height: f32, breadth: f32) {
    let side = z.signum();
    let b = m.vertices.len();
    m.vertices.extend([
        [x - 0.19, 0.42, z - breadth],
        [x + 0.20, 0.43, z - breadth],
        [x + 0.20, 0.43, z + breadth],
        [x - 0.19, 0.42, z + breadth],
        [x - 0.06, height, z + side * 0.10],
        [x + 0.10, height - 0.09, z + side * 0.10],
    ]);
    for [a, c, d] in [
        [0, 1, 4],
        [1, 5, 4],
        [1, 2, 5],
        [2, 3, 5],
        [3, 4, 5],
        [3, 0, 4],
        [0, 3, 2],
        [0, 2, 1],
    ] {
        m.faces.push((b + a, b + c, b + d, "ear".into()));
    }
    let b = m.vertices.len();
    m.vertices.extend([
        [x + 0.205, 0.58, z - breadth * 0.60],
        [x + 0.205, 0.58, z + breadth * 0.60],
        [x + 0.113, height - 0.22, z + side * 0.09],
    ]);
    m.faces.push((b, b + 1, b + 2, "socket".into()));
}

fn rounded_ear(m: &mut MeshAsset, x: f32, y: f32, z: f32, size: f32) {
    blob(m, [x, y, z], [size * 0.83, size, size * 0.86], "ear");
    blob(
        m,
        [x + size * 0.75, y + size * 0.08, z],
        [size * 0.09, size * 0.58, size * 0.55],
        "socket",
    );
}

fn eyes(m: &mut MeshAsset, x: f32, y: f32, z: f32, radius: f32) {
    for side in [-1.0, 1.0] {
        blob(
            m,
            [x, y, side * z],
            [radius * 0.95, radius * 0.62, radius * 0.28],
            "socket",
        );
        blob(
            m,
            [
                x + radius * 0.15,
                y + radius * 0.04,
                side * (z + radius * 0.25),
            ],
            [radius * 0.46, radius * 0.42, radius * 0.15],
            "eye",
        );
        blob(
            m,
            [
                x + radius * 0.18,
                y + radius * 0.045,
                side * (z + radius * 0.38),
            ],
            [radius * 0.16, radius * 0.36, radius * 0.025],
            "eye_dark",
        );
    }
}

fn lip(m: &mut MeshAsset, start: f32, end: f32, y: f32, rear_z: f32, front_z: f32) {
    for side in [-1.0, 1.0] {
        sweep(
            m,
            &[
                ([start, y + 0.06, side * rear_z], 0.022),
                ([end, y, side * front_z], 0.024),
            ],
            "mouth",
        );
    }
}

fn canine(hyena: bool) -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.92, 0.15, -0.25, 0.17],
            [-0.70, 0.62, -0.47, 0.47],
            [-0.35, 0.69, -0.55, 0.62],
            [0.06, 0.48, -0.58, 0.60],
            [0.35, 0.22, -0.47, 0.43],
            [0.75, 0.05, -0.37, 0.30],
            [1.21, -0.04, -0.31, 0.21],
            [1.32, -0.09, -0.28, 0.15],
        ],
        "base",
    );
    loft(
        &mut m,
        &[
            [0.06, -0.30, -0.60, 0.40],
            [0.52, -0.32, -0.55, 0.33],
            [1.18, -0.29, -0.43, 0.20],
        ],
        "jaw",
    );
    blob(&mut m, [1.28, -0.12, 0.0], [0.11, 0.15, 0.23], "nose");
    for side in [-1.0, 1.0] {
        if hyena {
            rounded_ear(&mut m, -0.47, 0.83, side * 0.47, 0.34);
        } else {
            pointed_ear(&mut m, -0.43, side * 0.43, 1.43, 0.22);
        }
        // Swept cheek ruffs are small connected tapered surfaces, not a mane.
        sweep(
            &mut m,
            &[
                ([-0.19, -0.16, side * 0.47], 0.23),
                ([-0.62, -0.37, side * 0.61], 0.18),
                ([-0.89, -0.39, side * 0.54], 0.012),
            ],
            "base",
        );
    }
    eyes(&mut m, 0.15, 0.23, 0.55, 0.17);
    lip(&mut m, 0.33, 1.20, -0.33, 0.40, 0.20);
    m
}

fn feline(lion: bool, cat: bool) -> MeshAsset {
    let mut m = empty();
    if lion {
        // A ruff surrounds the face, leaving the facial plane exposed. Its
        // lobed outer contour reads as a mane from front, side and rear.
        const N: usize = 24;
        let start = m.vertices.len();
        for (ring, (x, ry, rz)) in [
            (-1.12, 0.30, 0.28),
            (-0.98, 1.06, 0.89),
            (-0.72, 1.64, 1.34),
            (-0.40, 1.90, 1.55),
            (-0.08, 1.94, 1.59),
            (0.23, 1.78, 1.46),
            (0.43, 1.43, 1.18),
            (0.58, 0.97, 0.85),
        ]
        .into_iter()
        .enumerate()
        {
            for j in 0..N {
                let t = TAU * j as f32 / N as f32;
                let jag = if (1..7).contains(&ring) {
                    if j % 2 == 0 {
                        1.04
                    } else {
                        0.94
                    }
                } else {
                    1.0
                };
                let lower = if t.cos() < 0.0 { 1.10 } else { 1.0 };
                m.vertices.push([
                    x * 1.60 - 0.20 - if j % 2 == 0 { 0.03 } else { 0.0 },
                    -0.08 + ry * 0.82 * t.cos() * jag * lower,
                    rz * t.sin() * jag,
                ]);
            }
        }
        for ring in 0..7 {
            for j in 0..N {
                let a = start + ring * N + j;
                let b = start + ring * N + (j + 1) % N;
                let role = if j % 5 == 0 { "base" } else { "mane" };
                m.faces
                    .extend([(a, b, a + N, role.into()), (b, b + N, a + N, role.into())]);
            }
        }
    }
    loft(
        &mut m,
        &[
            [-0.78, 0.18, -0.29, 0.27],
            [-0.57, 0.62, -0.49, 0.53],
            [-0.15, 0.75, -0.62, 0.72],
            [0.24, 0.60, -0.60, 0.68],
            [0.49, 0.30, -0.48, 0.51],
            [0.79, 0.02, -0.42, 0.39],
            [0.91, -0.11, -0.34, 0.25],
        ],
        "base",
    );
    for side in [-1.0, 1.0] {
        if cat {
            pointed_ear(&mut m, -0.34, side * 0.49, 1.22, 0.23);
        } else {
            rounded_ear(&mut m, -0.34, 0.76, side * 0.55, 0.28);
        }
        blob(
            &mut m,
            [0.62, -0.24, side * 0.23],
            [0.30, 0.24, 0.30],
            "jaw",
        );
    }
    loft(
        &mut m,
        &[
            [0.12, -0.37, -0.63, 0.44],
            [0.61, -0.40, -0.62, 0.40],
            [0.82, -0.37, -0.52, 0.21],
        ],
        "jaw",
    );
    // Broad triangular cat nose instead of a dog's protruding round nose.
    let n = m.vertices.len();
    m.vertices.extend([
        [0.925, -0.11, -0.19],
        [0.925, -0.11, 0.19],
        [0.94, -0.29, 0.0],
        [0.87, -0.20, 0.0],
    ]);
    m.faces.extend([
        (n, n + 1, n + 2, "nose".into()),
        (n, n + 3, n + 1, "nose".into()),
        (n, n + 2, n + 3, "nose".into()),
        (n + 1, n + 3, n + 2, "nose".into()),
    ]);
    eyes(&mut m, 0.34, 0.26, 0.60, 0.19);
    lip(&mut m, 0.41, 0.83, -0.43, 0.42, 0.23);
    m
}

fn bear() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.86, 0.30, -0.38, 0.39],
            [-0.61, 0.82, -0.66, 0.76],
            [-0.12, 0.96, -0.81, 0.87],
            [0.33, 0.77, -0.74, 0.79],
            [0.58, 0.43, -0.59, 0.58],
            [0.79, 0.15, -0.45, 0.40],
        ],
        "base",
    );
    loft(
        &mut m,
        &[
            [0.32, 0.17, -0.61, 0.49],
            [0.63, 0.09, -0.55, 0.45],
            [0.88, -0.01, -0.44, 0.36],
        ],
        "jaw",
    );
    for side in [-1.0, 1.0] {
        blob(
            &mut m,
            [-0.48, 0.94, side * 0.67],
            [0.36, 0.26, 0.31],
            "ear",
        );
        blob(
            &mut m,
            [-0.15, 0.96, side * 0.67],
            [0.035, 0.15, 0.17],
            "socket",
        );
    }
    blob(&mut m, [0.87, -0.08, 0.0], [0.11, 0.18, 0.34], "nose");
    eyes(&mut m, 0.31, 0.30, 0.70, 0.18);
    lip(&mut m, 0.40, 0.86, -0.40, 0.47, 0.34);
    for v in &mut m.vertices {
        if v[0] > 0.0 {
            v[0] *= 0.72;
        }
    }
    m
}

fn ungulate(horse: bool, goat: bool) -> MeshAsset {
    let mut m = empty();
    let bovine = !horse && !goat;
    let width = if horse {
        0.80
    } else if goat {
        0.73
    } else {
        1.0
    };
    loft(
        &mut m,
        if bovine {
            &[
                [-0.84, 0.31, -0.33, 0.31],
                [-0.56, 0.89, -0.58, 0.69],
                [-0.10, 0.90, -0.79, 0.80],
                [0.39, 0.51, -0.80, 0.71],
                [0.78, 0.13, -0.74, 0.65],
                [1.06, -0.02, -0.64, 0.58],
            ][..]
        } else {
            &[
                [-0.80, 0.27, -0.31, 0.25],
                [-0.58, 0.76, -0.47, 0.57],
                [-0.15, 0.80, -0.69, 0.68],
                [0.27, 0.46, -0.77, 0.57],
                [0.72, 0.06, -0.75, 0.42],
                [1.15, -0.20, -0.77, 0.44],
                [1.34, -0.32, -0.68, 0.35],
            ][..]
        },
        "base",
    );
    for v in &mut m.vertices {
        v[2] *= width;
    }
    for s in [-1.0, 1.0] {
        if horse {
            pointed_ear(&mut m, -0.51, s * 0.37, 1.34, 0.16);
        } else {
            sweep(
                &mut m,
                &[
                    ([-0.47, 0.45, s * 0.47 * width], 0.18),
                    ([-0.58, 0.56, s * 0.88 * width], 0.16),
                    ([-0.43, 0.68, s * 1.10 * width], 0.015),
                ],
                "ear",
            );
        }
        blob(
            &mut m,
            [
                if bovine { 1.095 } else { 1.28 },
                if bovine { -0.25 } else { -0.39 },
                s * if bovine { 0.40 } else { 0.27 * width },
            ],
            [0.075, 0.10, 0.12],
            "nose",
        );
    }
    let muzzle = if bovine {
        [
            [0.60, 0.15, -0.78, 0.61],
            [0.94, 0.04, -0.72, 0.66],
            [1.075, -0.04, -0.64, 0.60],
        ]
    } else {
        [
            [0.94, -0.22, -0.76, 0.42 * width],
            [1.25, -0.28, -0.76, 0.44 * width],
            [1.39, -0.42, -0.65, 0.32 * width],
        ]
    };
    loft(&mut m, &muzzle, "muzzle");
    eyes(&mut m, 0.07, 0.25, 0.60 * width, 0.17);
    if bovine {
        lip(&mut m, 0.62, 1.07, -0.62, 0.60, 0.56);
    } else {
        lip(&mut m, 0.90, 1.35, -0.64, 0.38 * width, 0.30 * width);
    }
    if goat {
        sweep(
            &mut m,
            &[
                ([0.32, -0.62, 0.0], 0.22),
                ([0.36, -1.04, 0.0], 0.15),
                ([0.22, -1.25, 0.0], 0.01),
            ],
            "mane",
        );
    }
    m
}

fn boar() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.99, 0.30, -0.25, 0.25],
            [-0.70, 0.74, -0.62, 0.66],
            [-0.25, 0.62, -0.69, 0.74],
            [0.29, 0.28, -0.58, 0.55],
            [0.82, -0.02, -0.48, 0.35],
            [1.26, -0.08, -0.45, 0.32],
        ],
        "base",
    );
    for s in [-1.0, 1.0] {
        pointed_ear(&mut m, -0.52, s * 0.50, 1.07, 0.27);
    }
    // A vertical, flat snout disc rather than a canine nose cap.
    blob(&mut m, [1.27, -0.26, 0.0], [0.065, 0.24, 0.35], "muzzle");
    for s in [-1.0, 1.0] {
        blob(
            &mut m,
            [1.333, -0.23, s * 0.15],
            [0.008, 0.062, 0.055],
            "nose",
        );
    }
    eyes(&mut m, 0.03, 0.22, 0.62, 0.15);
    lip(&mut m, 0.32, 1.20, -0.44, 0.48, 0.31);
    m
}

fn rabbit() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.81, 0.25, -0.23, 0.25],
            [-0.54, 0.73, -0.57, 0.59],
            [-0.10, 0.72, -0.65, 0.69],
            [0.34, 0.36, -0.56, 0.52],
            [0.72, 0.04, -0.35, 0.34],
            [0.86, -0.10, -0.25, 0.17],
        ],
        "base",
    );
    for s in [-1.0, 1.0] {
        blob(&mut m, [0.58, -0.24, s * 0.19], [0.27, 0.21, 0.24], "jaw");
    }
    blob(&mut m, [0.86, -0.12, 0.0], [0.075, 0.09, 0.13], "nose");
    eyes(&mut m, 0.16, 0.30, 0.57, 0.24);
    for s in [-1.0, 1.0] {
        blob(
            &mut m,
            [0.77, -0.38, s * 0.065],
            [0.05, 0.10, 0.055],
            "tooth",
        );
    }
    m
}

fn rodent() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.85, 0.21, -0.22, 0.23],
            [-0.53, 0.64, -0.51, 0.58],
            [-0.12, 0.57, -0.53, 0.60],
            [0.38, 0.18, -0.40, 0.37],
            [0.97, -0.10, -0.26, 0.12],
        ],
        "base",
    );
    for s in [-1.0, 1.0] {
        rounded_ear(&mut m, -0.46, 0.65, s * 0.51, 0.42);
    }
    eyes(&mut m, 0.20, 0.20, 0.48, 0.21);
    blob(&mut m, [0.97, -0.14, 0.0], [0.08, 0.09, 0.12], "nose");
    blob(&mut m, [0.81, -0.29, 0.0], [0.07, 0.13, 0.10], "tooth");
    m
}

fn reptile(kind: &str) -> MeshAsset {
    let mut m = empty();
    let croc = kind == "crocodile";
    let rex = kind == "tyrannosaur";
    let dragon = kind == "dragon";
    let end = if croc {
        1.63
    } else if rex {
        1.23
    } else {
        1.15
    };
    let brow = if rex {
        0.92
    } else if dragon {
        0.66
    } else {
        0.50
    };
    let jaw = if rex { -0.69 } else { -0.44 };
    loft(
        &mut m,
        &[
            [-0.89, 0.15, -0.27, 0.24],
            [-0.57, brow * 0.88, jaw, 0.66],
            [-0.13, brow, jaw - 0.06, 0.78],
            [0.29, brow * 0.70, jaw, 0.73],
            [
                0.70,
                if croc { 0.17 } else { brow * 0.46 },
                jaw + 0.08,
                0.55,
            ],
            [end, 0.05, jaw + 0.14, if croc { 0.45 } else { 0.34 }],
        ],
        "base",
    );
    loft(
        &mut m,
        &[
            [-0.10, jaw + 0.14, jaw - 0.16, 0.63],
            [0.69, jaw + 0.09, jaw - 0.12, 0.52],
            [
                end - 0.05,
                jaw + 0.13,
                jaw - 0.01,
                if croc { 0.44 } else { 0.32 },
            ],
        ],
        "jaw",
    );
    for s in [-1.0, 1.0] {
        blob(
            &mut m,
            [-0.02, brow * 0.73, s * 0.63],
            [0.33, 0.12, 0.23],
            "brow",
        );
        blob(
            &mut m,
            [end - 0.13, 0.07, s * 0.30],
            [0.09, 0.055, 0.075],
            "nose",
        );
        for i in 0..if croc { 6 } else { 4 } {
            let x = 0.37 + (end - 0.49) * i as f32 / if croc { 5.0 } else { 3.0 };
            sweep(
                &mut m,
                &[
                    ([x, jaw + 0.13, s * (0.60 - (x / end) * 0.20)], 0.055),
                    ([x + 0.03, jaw - 0.09, s * (0.59 - (x / end) * 0.20)], 0.004),
                ],
                "tooth",
            );
        }
        if dragon {
            sweep(
                &mut m,
                &[
                    ([-0.51, 0.26, s * 0.59], 0.20),
                    ([-0.84, 0.38, s * 0.91], 0.17),
                    ([-1.12, 0.50, s * 1.07], 0.005),
                ],
                "base",
            );
            sweep(
                &mut m,
                &[
                    ([-0.48, 0.56, s * 0.42], 0.16),
                    ([-0.72, 0.88, s * 0.50], 0.10),
                    ([-1.00, 1.06, s * 0.57], 0.005),
                ],
                "horn",
            );
        }
    }
    eyes(
        &mut m,
        0.10,
        brow * 0.58,
        0.72,
        if rex { 0.17 } else { 0.18 },
    );
    lip(
        &mut m,
        0.24,
        end - 0.03,
        jaw + 0.07,
        0.69,
        if croc { 0.45 } else { 0.34 },
    );
    m
}

fn herbivore_dino() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.86, 0.20, -0.24, 0.23],
            [-0.56, 0.61, -0.54, 0.56],
            [-0.13, 0.63, -0.58, 0.65],
            [0.25, 0.36, -0.45, 0.55],
            [0.66, 0.08, -0.34, 0.43],
            [1.03, -0.03, -0.25, 0.35],
        ],
        "base",
    );
    loft(
        &mut m,
        &[[0.76, 0.07, -0.34, 0.40], [1.12, -0.08, -0.30, 0.29]],
        "beak",
    );
    eyes(&mut m, 0.12, 0.27, 0.57, 0.19);
    m
}

fn bird(kind: &str) -> MeshAsset {
    let mut m = empty();
    if kind == "owl" {
        loft(
            &mut m,
            &[
                [-0.75, 0.24, -0.32, 0.27],
                [-0.41, 0.83, -0.70, 0.70],
                [0.02, 0.85, -0.73, 0.82],
                [0.43, 0.58, -0.59, 0.68],
                [0.55, 0.23, -0.35, 0.41],
            ],
            "base",
        );
        for s in [-1.0, 1.0] {
            blob(&mut m, [0.44, 0.07, s * 0.40], [0.13, 0.43, 0.37], "jaw");
            blob(
                &mut m,
                [0.55, 0.12, s * 0.40],
                [0.045, 0.20, 0.18],
                "socket",
            );
            blob(&mut m, [0.592, 0.12, s * 0.40], [0.015, 0.14, 0.12], "eye");
            blob(
                &mut m,
                [0.609, 0.12, s * 0.40],
                [0.006, 0.11, 0.06],
                "eye_dark",
            );
        }
        sweep(
            &mut m,
            &[
                ([0.50, 0.08, 0.0], 0.16),
                ([0.73, -0.17, 0.0], 0.10),
                ([0.67, -0.43, 0.0], 0.008),
            ],
            "beak",
        );
    } else {
        loft(
            &mut m,
            &[
                [-0.81, 0.14, -0.23, 0.21],
                [-0.53, 0.68, -0.56, 0.49],
                [-0.10, 0.76, -0.59, 0.61],
                [0.33, 0.42, -0.39, 0.50],
                [0.60, 0.10, -0.21, 0.26],
            ],
            "base",
        );
        let long = matches!(kind, "heron" | "pelican");
        let end = if long {
            1.77
        } else if kind == "songbird" {
            0.99
        } else {
            1.14
        };
        loft(
            &mut m,
            &[
                [0.42, 0.20, -0.21, 0.28],
                [0.78, 0.12, -0.22, 0.23],
                [end, -0.06, -0.18, 0.015],
            ],
            "beak",
        );
        if !long && kind != "songbird" {
            sweep(
                &mut m,
                &[
                    ([1.01, -0.08, 0.0], 0.10),
                    ([1.12, -0.24, 0.0], 0.075),
                    ([1.02, -0.40, 0.0], 0.004),
                ],
                "beak",
            );
        }
        if kind == "pelican" {
            loft(
                &mut m,
                &[
                    [0.45, -0.20, -0.31, 0.24],
                    [1.08, -0.22, -0.67, 0.25],
                    [1.64, -0.17, -0.30, 0.07],
                ],
                "jaw",
            );
        }
        eyes(&mut m, 0.25, 0.22, 0.49, 0.17);
    }
    m
}

fn elephant(mammoth: bool) -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.86, 0.22, -0.31, 0.30],
            [-0.57, if mammoth { 1.12 } else { 0.84 }, -0.65, 0.67],
            [-0.14, if mammoth { 1.22 } else { 0.96 }, -0.83, 0.84],
            [0.28, 0.76, -0.77, 0.73],
            [0.67, 0.35, -0.58, 0.43],
            [0.84, 0.02, -0.52, 0.26],
        ],
        "base",
    );
    for s in [-1.0, 1.0] {
        // Thin scalloped ear fans sweep back rather than being round side balls.
        let b = m.vertices.len();
        let extent = if mammoth { 1.05 } else { 1.54 };
        let outline = [
            [-0.28, 0.55, s * 0.57],
            [-0.57, 0.75, s * 0.84],
            [-0.78, 0.41, s * extent],
            [-0.79, -0.20, s * (extent + 0.03)],
            [-0.62, -0.81, s * (extent * 0.84)],
            [-0.30, -0.82, s * 0.64],
        ];
        m.vertices.push([-0.18, -0.08, s * 0.66]);
        m.vertices.extend(outline);
        for j in 0..6 {
            m.faces
                .push((b, b + 1 + j, b + 1 + (j + 1) % 6, "ear".into()));
        }
        let back = m.vertices.len();
        for q in outline {
            m.vertices.push([q[0] - 0.10, q[1], q[2]]);
        }
        for j in 0..6 {
            let k = (j + 1) % 6;
            m.faces.extend([
                (b + 1 + j, back + j, back + k, "ear".into()),
                (b + 1 + j, back + k, b + 1 + k, "ear".into()),
            ]);
        }
        for j in 1..5 {
            m.faces.push((back, back + j + 1, back + j, "ear".into()));
        }
        if mammoth {
            for j in 0..4 {
                let x = -0.65 + j as f32 * 0.27;
                sweep(
                    &mut m,
                    &[
                        ([x, -0.38, s * 0.59], 0.20),
                        ([x - 0.10, -0.94, s * 0.63], 0.12),
                        ([x - 0.22, -1.15, s * 0.53], 0.005),
                    ],
                    "base",
                );
            }
        }
    }
    eyes(&mut m, 0.39, 0.14, 0.61, 0.14);
    m
}

fn rhino() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.94, 0.31, -0.31, 0.29],
            [-0.64, 0.77, -0.65, 0.70],
            [-0.18, 0.63, -0.76, 0.81],
            [0.32, 0.34, -0.72, 0.70],
            [0.80, 0.12, -0.61, 0.51],
            [1.24, -0.04, -0.49, 0.37],
        ],
        "base",
    );
    for s in [-1.0, 1.0] {
        pointed_ear(&mut m, -0.58, s * 0.50, 1.04, 0.18);
        blob(&mut m, [1.11, -0.20, s * 0.31], [0.10, 0.07, 0.07], "nose");
    }
    eyes(&mut m, 0.06, 0.23, 0.70, 0.14);
    lip(&mut m, 0.51, 1.21, -0.49, 0.60, 0.35);
    m
}

fn body(kind: &str) -> MeshAsset {
    let mut m = empty();
    let stations: &[[f32; 4]] = match kind {
        "canine" => &[
            [-1.0, 0.12, -0.15, 0.24],
            [-0.78, 0.69, -0.64, 0.74],
            [-0.45, 0.75, -0.43, 0.72],
            [-0.08, 0.78, -0.53, 0.72],
            [0.34, 0.91, -0.90, 0.94],
            [0.64, 0.98, -0.87, 0.87],
            [0.94, 0.51, -0.47, 0.43],
            [1.04, 0.12, -0.13, 0.14],
        ],
        "feline" => &[
            [-1.0, 0.17, -0.22, 0.25],
            [-0.76, 0.81, -0.63, 0.78],
            [-0.38, 0.72, -0.54, 0.78],
            [0.04, 0.73, -0.63, 0.79],
            [0.40, 0.83, -0.86, 0.91],
            [0.70, 0.96, -0.72, 0.85],
            [0.98, 0.43, -0.37, 0.40],
        ],
        "bear" => &[
            [-1.12, 0.12, -0.20, 0.28],
            [-0.85, 0.60, -0.60, 0.82],
            [-0.30, 0.72, -0.66, 0.96],
            [0.23, 0.90, -0.72, 1.0],
            [0.56, 1.18, -0.66, 0.91],
            [0.89, 0.70, -0.45, 0.67],
            [1.12, 0.20, -0.18, 0.24],
        ],
        "boar" => &[
            [-1.0, 0.10, -0.17, 0.22],
            [-0.73, 0.67, -0.70, 0.73],
            [-0.26, 1.01, -0.91, 0.92],
            [0.20, 1.17, -0.94, 1.0],
            [0.61, 1.05, -0.77, 0.85],
            [0.95, 0.42, -0.36, 0.35],
        ],
        "rabbit" => &[
            [-1.0, 0.10, -0.20, 0.28],
            [-0.72, 0.96, -0.82, 1.0],
            [-0.26, 1.02, -0.94, 0.96],
            [0.20, 0.71, -0.78, 0.74],
            [0.67, 0.47, -0.56, 0.54],
            [1.0, 0.16, -0.17, 0.20],
        ],
        "bird" => &[
            [-1.0, 0.0, -0.1, 0.10],
            [-0.65, 0.41, -0.58, 0.51],
            [-0.18, 0.72, -0.91, 0.86],
            [0.30, 0.85, -1.04, 0.99],
            [0.67, 0.73, -0.86, 0.86],
            [1.0, 0.17, -0.27, 0.23],
        ],
        "reptile" => &[
            [-1.0, 0.15, -0.19, 0.26],
            [-0.70, 0.59, -0.66, 0.80],
            [-0.16, 0.72, -0.78, 1.0],
            [0.38, 0.70, -0.72, 0.95],
            [0.80, 0.41, -0.50, 0.63],
            [1.0, 0.12, -0.16, 0.22],
        ],
        _ => &[
            [-1.0, 0.18, -0.16, 0.27],
            [-0.73, 0.78, -0.73, 0.82],
            [-0.30, 0.81, -0.85, 0.96],
            [0.18, 0.86, -0.99, 1.0],
            [0.55, 1.00, -0.94, 0.92],
            [0.84, 0.70, -0.62, 0.63],
            [1.0, 0.23, -0.20, 0.27],
        ],
    };
    loft(&mut m, stations, "base");
    m
}

fn tail(kind: &str) -> MeshAsset {
    let mut m = empty();
    match kind {
        "canine" => loft(
            &mut m,
            &[
                [-1.16, -0.17, -0.24, 0.025],
                [-0.89, 0.16, -0.50, 0.32],
                [-0.48, 0.56, -0.67, 0.54],
                [0.03, 0.77, -0.59, 0.59],
                [0.55, 0.61, -0.45, 0.45],
                [1.05, 0.31, -0.26, 0.26],
            ],
            "base",
        ),
        "feline" => loft(
            &mut m,
            &[
                [-1.10, 0.24, 0.18, 0.035],
                [-0.88, 0.36, 0.11, 0.13],
                [-0.50, 0.22, -0.13, 0.17],
                [0.0, 0.15, -0.27, 0.20],
                [0.55, 0.21, -0.28, 0.23],
                [1.05, 0.28, -0.28, 0.26],
            ],
            "base",
        ),
        "reptile" => loft(
            &mut m,
            &[
                [-1.13, 0.02, -0.04, 0.015],
                [-0.75, 0.16, -0.16, 0.19],
                [-0.26, 0.37, -0.31, 0.39],
                [0.40, 0.67, -0.53, 0.63],
                [1.02, 0.82, -0.69, 0.79],
            ],
            "base",
        ),
        "tuft" => loft(
            &mut m,
            &[
                [-1.10, 0.0, -0.04, 0.02],
                [-0.59, 0.58, -0.60, 0.51],
                [0.0, 0.85, -0.83, 0.78],
                [0.56, 0.60, -0.56, 0.57],
                [1.0, 0.18, -0.18, 0.18],
            ],
            "mane",
        ),
        "feather" => loft(
            &mut m,
            &[
                [-1.10, 0.0, -0.02, 0.02],
                [-0.88, 0.18, -0.16, 0.59],
                [-0.30, 0.24, -0.23, 0.74],
                [0.44, 0.16, -0.17, 0.50],
                [1.0, 0.04, -0.06, 0.10],
            ],
            "base",
        ),
        _ => loft(
            &mut m,
            &[
                [-1.08, 0.0, -0.04, 0.01],
                [-0.60, 0.20, -0.20, 0.17],
                [0.1, 0.27, -0.27, 0.26],
                [1.0, 0.43, -0.42, 0.37],
            ],
            "base",
        ),
    }
    m
}

fn paw() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.77, 0.12, -0.22, 0.24],
            [-0.32, 0.38, -0.34, 0.59],
            [0.24, 0.19, -0.35, 0.65],
            [0.71, 0.02, -0.31, 0.45],
        ],
        "base",
    );
    for z in [-0.43, -0.14, 0.14, 0.43] {
        blob(&mut m, [0.50, -0.11, z], [0.29, 0.21, 0.18], "base");
    }
    m
}

fn hoof() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.60, 0.33, -0.39, 0.35],
            [-0.23, 0.54, -0.42, 0.49],
            [0.45, 0.17, -0.42, 0.53],
            [0.77, -0.02, -0.39, 0.33],
        ],
        "hoof",
    );
    sweep(
        &mut m,
        &[([0.78, -0.25, 0.0], 0.025), ([0.58, 0.12, 0.0], 0.025)],
        "mouth",
    );
    m
}

fn trunk() -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, 0.61, 0.0], 0.90),
            ([0.12, 0.30, 0.0], 0.76),
            ([0.22, -0.10, 0.0], 0.57),
            ([0.27, -0.47, 0.0], 0.43),
            ([0.54, -0.88, 0.0], 0.34),
            ([1.09, -0.90, 0.0], 0.27),
            ([1.36, -0.65, 0.0], 0.17),
        ],
        "base",
    );
    m
}

fn tusk(mammoth: bool) -> MeshAsset {
    let mut m = empty();
    let path: &[([f32; 3], f32)] = if mammoth {
        &[
            ([0.0, 0.26, 0.0], 0.29),
            ([0.52, 0.02, 0.0], 0.27),
            ([1.15, -0.25, 0.0], 0.22),
            ([1.79, -0.27, 0.0], 0.17),
            ([2.21, 0.0, 0.0], 0.12),
            ([2.24, 0.42, 0.0], 0.07),
            ([1.98, 0.72, 0.0], 0.004),
        ]
    } else {
        &[
            ([0.0, 0.22, 0.0], 0.27),
            ([0.53, 0.0, 0.0], 0.23),
            ([1.15, -0.17, 0.0], 0.18),
            ([1.81, -0.12, 0.0], 0.115),
            ([2.29, 0.18, 0.0], 0.004),
        ]
    };
    sweep(&mut m, path, "tooth");
    m
}

fn nose_horn() -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, -0.18, 0.0], 0.68),
            ([0.04, 0.19, 0.0], 0.50),
            ([0.14, 0.68, 0.0], 0.31),
            ([0.28, 1.09, 0.0], 0.12),
            ([0.40, 1.31, 0.0], 0.003),
        ],
        "horn",
    );
    m
}

fn shell(lobster: bool) -> MeshAsset {
    let mut m = empty();
    if lobster {
        loft(
            &mut m,
            &[
                [-1.05, 0.02, -0.19, 0.22],
                [-0.74, 0.55, -0.39, 0.66],
                [-0.19, 0.76, -0.48, 0.82],
                [0.43, 0.68, -0.43, 0.73],
                [0.94, 0.29, -0.22, 0.39],
            ],
            "shell",
        );
    } else {
        loft(
            &mut m,
            &[
                [-0.87, 0.10, -0.20, 0.40],
                [-0.57, 0.60, -0.29, 0.91],
                [-0.10, 0.73, -0.34, 1.15],
                [0.39, 0.56, -0.31, 1.04],
                [0.73, 0.25, -0.25, 0.62],
            ],
            "shell",
        );
    }
    m
}

fn crustacean_head() -> MeshAsset {
    let mut m = shell(false);
    for s in [-1.0, 1.0] {
        sweep(
            &mut m,
            &[
                ([0.35, 0.28, s * 0.49], 0.11),
                ([0.67, 0.70, s * 0.68], 0.085),
            ],
            "base",
        );
        blob(&mut m, [0.69, 0.73, s * 0.69], [0.17, 0.14, 0.14], "socket");
    }
    m
}

fn pincer() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.91, 0.16, -0.20, 0.15],
            [-0.40, 0.48, -0.45, 0.45],
            [0.05, 0.51, -0.44, 0.52],
            [0.36, 0.32, -0.32, 0.42],
        ],
        "shell",
    );
    for s in [-1.0, 1.0] {
        sweep(
            &mut m,
            &[
                ([0.11, s * 0.24, s * 0.27], 0.25),
                ([0.59, s * 0.45, s * 0.43], 0.19),
                ([1.05, s * 0.30, s * 0.32], 0.11),
                ([1.30, s * 0.08, s * 0.08], 0.006),
            ],
            "shell",
        );
    }
    m
}

fn snake() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.84, 0.17, -0.18, 0.20],
            [-0.55, 0.39, -0.31, 0.51],
            [-0.07, 0.43, -0.36, 0.71],
            [0.43, 0.28, -0.30, 0.58],
            [0.92, 0.06, -0.23, 0.35],
            [1.12, -0.01, -0.19, 0.23],
        ],
        "base",
    );
    loft(
        &mut m,
        &[
            [0.02, -0.22, -0.40, 0.59],
            [0.70, -0.20, -0.32, 0.40],
            [1.07, -0.17, -0.25, 0.24],
        ],
        "jaw",
    );
    eyes(&mut m, 0.21, 0.20, 0.62, 0.22);
    lip(&mut m, 0.12, 1.06, -0.25, 0.64, 0.24);
    for s in [-1.0, 1.0] {
        blob(
            &mut m,
            [0.97, 0.025, s * 0.21],
            [0.045, 0.034, 0.042],
            "nose",
        );
    }
    m
}

fn worm_maw() -> MeshAsset {
    let mut m = empty();
    const N: usize = 24;
    // Open concentric lips and a recessed dark throat, never a mammal skull.
    for (x, r) in [
        (-0.96, 0.52),
        (-0.55, 0.83),
        (0.18, 0.97),
        (0.65, 0.87),
        (0.77, 0.66),
        (0.38, 0.44),
    ] {
        for j in 0..N {
            let t = TAU * j as f32 / N as f32;
            m.vertices.push([x, r * t.cos(), r * t.sin()]);
        }
    }
    for band in 0..5 {
        for j in 0..N {
            let a = band * N + j;
            let b = band * N + (j + 1) % N;
            let role = if band > 3 {
                "mouth"
            } else if band == 3 {
                "jaw"
            } else {
                "base"
            };
            m.faces
                .extend([(a, b, a + N, role.into()), (b, b + N, a + N, role.into())]);
        }
    }
    let c = m.vertices.len();
    m.vertices.push([0.33, 0.0, 0.0]);
    for j in 0..N {
        m.faces
            .push((c, 5 * N + j, 5 * N + (j + 1) % N, "mouth".into()));
    }
    for j in 0..12 {
        let t = TAU * j as f32 / 12.0;
        let y = t.cos();
        let z = t.sin();
        sweep(
            &mut m,
            &[
                ([0.73, y * 0.73, z * 0.73], 0.105),
                ([0.96, y * 0.56, z * 0.56], 0.070),
                ([0.93, y * 0.34, z * 0.34], 0.004),
            ],
            "tooth",
        );
    }
    m
}

fn bull_horn(side: f32) -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, -0.16, 0.0], 0.30),
            ([-0.03, 0.12, side * 0.56], 0.25),
            ([0.03, 0.45, side * 1.18], 0.17),
            ([0.13, 0.91, side * 1.38], 0.08),
            ([0.26, 1.20, side * 1.29], 0.004),
        ],
        "horn",
    );
    m
}

fn cobra_hood() -> MeshAsset {
    let mut m = empty();
    // Flattened front-to-back with a broad, shield-shaped lateral flare.
    loft(
        &mut m,
        &[
            [-0.50, 0.72, -0.85, 0.32],
            [-0.20, 0.98, -0.96, 1.27],
            [0.10, 0.96, -0.93, 1.30],
            [0.34, 0.72, -0.81, 0.34],
        ],
        "base",
    );
    m
}

fn rabbit_ear() -> MeshAsset {
    let mut m = empty();
    const N: usize = 12;
    for (y, rx, rz) in [
        (-1.0, 0.10, 0.12),
        (-0.60, 0.22, 0.46),
        (0.0, 0.23, 0.60),
        (0.58, 0.17, 0.45),
        (0.98, 0.025, 0.05),
    ] {
        for j in 0..N {
            let t = TAU * j as f32 / N as f32;
            m.vertices.push([rx * t.cos(), y, rz * t.sin()]);
        }
    }
    for ring in 0..4 {
        for j in 0..N {
            let a = ring * N + j;
            let b = ring * N + (j + 1) % N;
            let role = if !(2..=N - 3).contains(&j) {
                "jaw"
            } else {
                "ear"
            };
            m.faces
                .extend([(a, b, a + N, role.into()), (b, b + N, a + N, role.into())]);
        }
    }
    m
}

fn bear_tail() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-0.36, 0.02, -0.03, 0.03],
            [-0.12, 0.53, -0.39, 0.45],
            [0.20, 0.31, -0.27, 0.28],
        ],
        "base",
    );
    m
}

fn snake_body() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-1.04, 0.16, -0.16, 0.20],
            [-0.75, 0.33, -0.33, 0.40],
            [-0.25, 0.38, -0.38, 0.46],
            [0.35, 0.38, -0.38, 0.46],
            [0.80, 0.31, -0.31, 0.38],
            [1.04, 0.13, -0.13, 0.18],
        ],
        "base",
    );
    m
}

fn snake_segment() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-1.02, 0.13, -0.13, 0.14],
            [-0.65, 0.43, -0.43, 0.44],
            [0.0, 0.56, -0.56, 0.57],
            [0.67, 0.45, -0.45, 0.46],
            [1.02, 0.15, -0.15, 0.17],
        ],
        "base",
    );
    m
}

fn earthworm_head() -> MeshAsset {
    let mut m = empty();
    loft(
        &mut m,
        &[
            [-1.0, 0.17, -0.17, 0.19],
            [-0.63, 0.57, -0.57, 0.60],
            [-0.10, 0.64, -0.64, 0.66],
            [0.45, 0.49, -0.49, 0.50],
            [0.90, 0.18, -0.18, 0.20],
        ],
        "base",
    );
    blob(&mut m, [0.91, 0.0, 0.0], [0.025, 0.14, 0.15], "mouth");
    m
}

fn goat_horn() -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, -0.16, 0.0], 0.24),
            ([-0.08, 0.22, 0.0], 0.20),
            ([-0.30, 0.73, 0.0], 0.14),
            ([-0.62, 1.15, 0.0], 0.07),
            ([-0.96, 1.41, 0.0], 0.004),
        ],
        "horn",
    );
    m
}

fn ram_horn() -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, 0.10, 0.0], 0.30),
            ([-0.43, 0.59, 0.0], 0.29),
            ([-0.87, 0.57, 0.0], 0.25),
            ([-1.03, 0.10, 0.0], 0.20),
            ([-0.75, -0.45, 0.0], 0.15),
            ([-0.28, -0.57, 0.0], 0.095),
            ([0.03, -0.35, 0.0], 0.05),
            ([0.0, -0.12, 0.0], 0.004),
        ],
        "horn",
    );
    m
}

fn lobster_fan() -> MeshAsset {
    let mut m = empty();
    let outline = [
        [0.72, 0.0],
        [0.18, -0.40],
        [-0.30, -0.94],
        [-0.70, -1.0],
        [-0.87, -0.57],
        [-1.03, -0.27],
        [-0.90, 0.0],
        [-1.03, 0.27],
        [-0.87, 0.57],
        [-0.70, 1.0],
        [-0.30, 0.94],
        [0.18, 0.40],
    ];
    let n = outline.len();
    for y in [-0.16, 0.16] {
        m.vertices.push([0.0, y, 0.0]);
        for [x, z] in outline {
            m.vertices.push([x, y, z]);
        }
    }
    for j in 0..n {
        let k = (j + 1) % n;
        m.faces.extend([
            (0, 1 + k, 1 + j, "shell".into()),
            (n + 1, n + 2 + j, n + 2 + k, "shell".into()),
            (1 + j, 1 + k, n + 2 + j, "shell".into()),
            (1 + k, n + 2 + k, n + 2 + j, "shell".into()),
        ]);
    }
    m
}

/// Annulated cylindrical trunk, shared by the worm's native body joints.
/// Nearly constant cross-section keeps the silhouette long and low; shallow
/// collars remain readable without turning the individual joints into beads.
fn worm_tube(segment: bool) -> MeshAsset {
    let mut m = empty();
    let count = if segment { 7 } else { 25 };
    let mut stations = Vec::with_capacity(count);
    for i in 0..count {
        let x = -1.0 + 2.0 * i as f32 / (count - 1) as f32;
        let r = if i == 0 || i == count - 1 {
            0.83
        } else if i % 3 == 1 {
            1.035
        } else {
            0.94
        };
        stations.push([x, r, -r, r]);
    }
    loft(&mut m, &stations, "base");
    m
}

/// Thick leaf-shaped dorsal armor, rather than repurposed wing needles.
fn stego_plate() -> MeshAsset {
    let mut m = empty();
    let outline = [
        [-0.68, -0.65],
        [-1.12, -0.05],
        [-0.88, 0.56],
        [-0.22, 1.03],
        [0.30, 0.93],
        [1.02, 0.29],
        [0.66, -0.65],
    ];
    let n = outline.len();
    for z in [-0.22, 0.22] {
        m.vertices.push([0.0, 0.06, z * 1.65]);
        for [x, y] in outline {
            m.vertices.push([x, y, z]);
        }
    }
    for j in 0..n {
        let k = (j + 1) % n;
        m.faces.extend([
            (0, 1 + k, 1 + j, "horn".into()),
            (n + 1, n + 2 + j, n + 2 + k, "horn".into()),
            (1 + j, 1 + k, n + 2 + j, "horn".into()),
            (1 + k, n + 2 + k, n + 2 + j, "horn".into()),
        ]);
    }
    m
}

/// A shallow bowed, scalloped shield behind the skull. Its oval rim has real
/// thickness and lateral breadth, so it is neither a triangle nor a flat card.
fn triceratops_frill() -> MeshAsset {
    let mut m = empty();
    const N: usize = 24;
    for back in [false, true] {
        m.vertices
            .push([if back { -0.20 } else { 0.12 }, -0.12, 0.0]);
        for j in 0..N {
            let t = TAU * j as f32 / N as f32;
            let scallop = if j % 2 == 0 { 1.0 } else { 0.95 };
            let y = t.cos() * 1.02 * scallop;
            let z = t.sin() * 1.26 * scallop;
            m.vertices
                .push([-0.35 - 0.19 * y + if back { -0.13 } else { 0.13 }, y, z]);
        }
    }
    for j in 0..N {
        let k = (j + 1) % N;
        m.faces.extend([
            (0, 1 + j, 1 + k, "base".into()),
            (N + 1, N + 2 + k, N + 2 + j, "base".into()),
            (1 + j, N + 2 + j, 1 + k, "horn".into()),
            (1 + k, N + 2 + j, N + 2 + k, "horn".into()),
        ]);
    }
    m
}

fn triceratops_horn() -> MeshAsset {
    let mut m = empty();
    sweep(
        &mut m,
        &[
            ([0.0, -0.15, 0.0], 0.51),
            ([0.18, 0.20, 0.0], 0.43),
            ([0.74, 0.57, 0.0], 0.31),
            ([1.41, 0.83, 0.0], 0.16),
            ([2.17, 0.96, 0.0], 0.004),
        ],
        "horn",
    );
    m
}

fn meshes() -> &'static [MeshAsset] {
    static ASSETS: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    ASSETS.get_or_init(|| {
        vec![
            canine(false),
            feline(true, false),
            feline(false, false),
            feline(false, true),
            bear(),
            ungulate(true, false),
            ungulate(false, false),
            ungulate(false, true),
            boar(),
            rabbit(),
            rodent(),
            reptile("crocodile"),
            reptile("dragon"),
            reptile("tyrannosaur"),
            reptile("raptor"),
            herbivore_dino(),
            bird("raptor"),
            bird("songbird"),
            bird("heron"),
            bird("pelican"),
            bird("owl"),
            elephant(false),
            elephant(true),
            rhino(),
            canine(true),
            body("canine"),
            body("feline"),
            body("bear"),
            body("ungulate"),
            body("boar"),
            body("rabbit"),
            body("bird"),
            body("reptile"),
            tail("canine"),
            tail("feline"),
            tail("reptile"),
            tail("tuft"),
            tail("feather"),
            tail("thin"),
            paw(),
            hoof(),
            trunk(),
            tusk(false),
            tusk(true),
            nose_horn(),
            shell(false),
            shell(true),
            crustacean_head(),
            pincer(),
            snake(),
            worm_maw(),
            bull_horn(-1.0),
            bull_horn(1.0),
            cobra_hood(),
            rabbit_ear(),
            bear_tail(),
            snake_body(),
            snake_segment(),
            earthworm_head(),
            goat_horn(),
            ram_horn(),
            lobster_fan(),
            worm_tube(false),
            worm_tube(true),
            stego_plate(),
            triceratops_frill(),
            triceratops_horn(),
        ]
    })
}

/// Call only behind PROFILE_V2 and after humanoid/skeletal overrides. Affinity
/// must already be resolved to the local mount/rider/hybrid node owner.
pub(super) fn mesh(affinity: &str, kind: &str, node_id: &str) -> Option<&'static MeshAsset> {
    if let Some(mesh) = dinosaurs::mesh(affinity, kind, node_id) {
        return Some(mesh);
    }
    let canine = matches!(
        affinity,
        "WOLF"
            | "DOG"
            | "JACKAL"
            | "CERBERUS"
            | "FENRIR"
            | "CHUPACABRA"
            | "XOLOTL"
            | "HYENA"
            | "FOX"
    );
    let feline = matches!(
        affinity,
        "LION"
            | "CAT"
            | "TIGER"
            | "LEOPARD"
            | "CHEETAH"
            | "PANTHER"
            | "SABERTOOTH_CAT"
            | "MANTICORE"
    );
    let ungulate = matches!(
        affinity,
        "HORSE"
            | "ZEBRA"
            | "BULL"
            | "BUFFALO"
            | "GOAT"
            | "RAM"
            | "MOOSE"
            | "GIRAFFE"
            | "ANTELOPE"
            | "GAZELLE"
            | "DEER"
    );
    let bird = matches!(
        affinity,
        "FALCON"
            | "IBIS"
            | "VULTURE"
            | "RAVEN"
            | "HAWK"
            | "EAGLE"
            | "CROW"
            | "CONDOR"
            | "HERON"
            | "PELICAN"
            | "ALBATROSS"
            | "HUMMINGBIRD"
            | "BENNU"
            | "STRIX"
            | "BIRD"
            | "PIGEON"
            | "SPARROW"
            | "OWL"
            | "PHOENIX"
    );
    let reptile = matches!(
        affinity,
        "CROCODILE"
            | "DRAGON"
            | "WYVERN"
            | "HYDRA"
            | "COBRA"
            | "SERPENT"
            | "SNAKE"
            | "SEA_DRAGON"
            | "SEA_SERPENT"
            | "TREX"
            | "RAPTOR"
            | "LONGNECK_DINO"
            | "STEGO"
            | "TRICERATOPS"
            | "ANKYLOSAUR"
            | "QUETZALCOATL"
            | "APOPHIS"
            | "JORMUNGANDR"
            | "CIPACTLI"
    );
    let index = match kind {
        "HEAD" => match affinity {
            "HYENA" => 24,
            _ if canine => 0,
            "LION" => 1,
            "CAT" => 3,
            _ if feline => 2,
            "BEAR" => 4,
            "HORSE" | "ZEBRA" | "MOOSE" | "GIRAFFE" | "ANTELOPE" | "GAZELLE" | "DEER" => 5,
            "BULL" | "BUFFALO" | "MINOTAUR" => 6,
            "GOAT" | "RAM" => 7,
            "BOAR" | "PIG" => 8,
            "RABBIT" | "KILLER_RABBIT" => 9,
            "RAT" | "MOUSE" | "RODENT" => 10,
            "CROCODILE" | "CIPACTLI" => 11,
            "DRAGON" | "WYVERN" | "HYDRA" | "SEA_DRAGON" | "QUETZALCOATL" => 12,
            "TREX" => 13,
            "SNAKE" | "SERPENT" | "COBRA" | "APOPHIS" | "JORMUNGANDR" => 49,
            "SAND_WORM" => 50,
            "WORM" => 58,
            "LONGNECK_DINO" | "STEGO" | "TRICERATOPS" | "ANKYLOSAUR" => 15,
            _ if reptile => 14,
            "HERON" | "IBIS" | "HUMMINGBIRD" => 18,
            "PELICAN" => 19,
            "OWL" | "STRIX" => 20,
            "BIRD" | "PIGEON" | "SPARROW" => 17,
            _ if bird => 16,
            "ELEPHANT" | "GROOTSLANG" => 21,
            "MAMMOTH" => 22,
            "RHINO" => 23,
            "CRAB" | "LOBSTER" => 47,
            _ => return None,
        },
        "TORSO" => match affinity {
            "WORM" | "SAND_WORM" => 62,
            _ if canine => 25,
            _ if feline => 26,
            "BEAR" => 27,
            _ if ungulate => 28,
            "ELEPHANT" | "MAMMOTH" | "RHINO" | "HIPPO" => 28,
            "BOAR" | "PIG" => 29,
            "RABBIT" | "KILLER_RABBIT" | "RAT" | "MOUSE" => 30,
            _ if bird => 31,
            "SNAKE" | "SERPENT" | "COBRA" | "APOPHIS" | "JORMUNGANDR" => 56,
            _ if reptile => 32,
            "CRAB" => 45,
            "LOBSTER" => 46,
            _ => return None,
        },
        "TAIL" => match affinity {
            "WORM" | "SAND_WORM" => 63,
            _ if node_id.contains("tail_tuft") => 36,
            _ if node_id.contains("cotton_tail") => return None,
            _ if canine => 33,
            _ if feline => 34,
            "SNAKE" | "SERPENT" | "COBRA" | "APOPHIS" | "JORMUNGANDR" => 57,
            _ if reptile => 35,
            _ if bird => 37,
            "RAT" | "MOUSE" | "BOAR" | "PIG" => 38,
            "BEAR" => 55,
            _ => return None,
        },
        "LOBE" if node_id.contains("gorgon_snake_tip") => 49,
        "LOBE" if node_id.contains("tail_tuft") => 36,
        "EAR" if matches!(affinity, "RABBIT" | "KILLER_RABBIT") => 54,
        "FOOT" if canine || feline || affinity == "BEAR" => 39,
        "FOOT" if ungulate || affinity == "BOAR" => 40,
        "TENTACLE" | "TRUNK"
            if node_id.contains("trunk")
                && matches!(affinity, "ELEPHANT" | "MAMMOTH" | "GROOTSLANG") =>
        {
            41
        }
        "TUSK" if affinity == "MAMMOTH" => 43,
        "TUSK" if matches!(affinity, "ELEPHANT" | "GROOTSLANG") => 42,
        "HORN" if affinity == "RHINO" => 44,
        "HORN" if affinity == "TRICERATOPS" && node_id.contains("triceratops_horn_") => 66,
        "HORN" if affinity == "GOAT" => 59,
        "HORN" if affinity == "RAM" => 60,
        "FLUKE" if affinity == "LOBSTER" => 61,
        "HORN"
            if matches!(affinity, "BULL" | "BUFFALO" | "MINOTAUR")
                && node_id.contains("bull_horn") =>
        {
            if node_id.ends_with("_0") {
                51
            } else {
                52
            }
        }
        "NECK" if affinity == "COBRA" => 53,
        "NECK" if matches!(affinity, "WORM" | "SAND_WORM") => 63,
        "FIN" if affinity == "STEGO" && node_id.contains("dorsal_plate_") => 64,
        "SHELL" if affinity == "TRICERATOPS" && node_id.contains("head_frill") => 65,
        "SHELL" if affinity == "CRAB" => 45,
        "SHELL" if affinity == "LOBSTER" => 46,
        "CLAW" | "HAND" if matches!(affinity, "CRAB" | "LOBSTER") => 48,
        _ => return None,
    };
    Some(&meshes()[index])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animal_meshes_have_finite_vertices_and_valid_faces() {
        for m in meshes() {
            assert!(!m.vertices.is_empty() && !m.faces.is_empty());
            assert!(m.vertices.iter().flatten().all(|v| v.is_finite()));
            for (a, b, c, _) in &m.faces {
                assert!(*a < m.vertices.len() && *b < m.vertices.len() && *c < m.vertices.len());
            }
        }
    }
    #[test]
    fn worms_and_dinosaur_landmarks_are_volumetric_species_surfaces() {
        let tube = mesh("WORM", "TORSO", "torso").unwrap();
        let ring_radii: Vec<f32> = tube
            .vertices
            .chunks(16)
            .take(25)
            .map(|ring| ring[0][1])
            .collect();
        assert!(ring_radii.iter().all(|r| (0.83..=1.04).contains(r)));
        assert!(ring_radii.windows(2).any(|r| (r[0] - r[1]).abs() > 0.08));
        let plate = mesh("STEGO", "FIN", "dorsal_plate_2").unwrap();
        assert!(plate.vertices.iter().any(|p| p[0] > 1.0));
        assert!(plate.vertices.iter().any(|p| p[0] < -1.0));
        assert!(plate.vertices.iter().any(|p| p[2] > 0.3));
        let frill = mesh("TRICERATOPS", "SHELL", "head_frill").unwrap();
        assert!(frill.vertices.iter().any(|p| p[2] > 1.2));
        assert!(frill.vertices.iter().any(|p| p[2] < -1.2));
        let horn = mesh("TRICERATOPS", "HORN", "triceratops_horn_0").unwrap();
        assert!(horn.vertices.iter().any(|p| p[0] > 2.1));
    }
    #[test]
    fn wolf_lion_and_cat_keep_distinct_silhouette_landmarks() {
        let wolf = mesh("WOLF", "HEAD", "head_0").unwrap();
        let lion = mesh("LION", "HEAD", "head_0").unwrap();
        let cat = mesh("CAT", "HEAD", "head_0").unwrap();
        let extent = |m: &MeshAsset, axis: usize| {
            m.vertices
                .iter()
                .map(|v| v[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        assert!(
            extent(wolf, 0) > extent(lion, 0) + 0.30,
            "canid muzzle must project"
        );
        assert!(
            extent(wolf, 1) > extent(cat, 1) + 0.15,
            "wolf pinnae are upright"
        );
        assert!(
            extent(lion, 2) > extent(cat, 2) + 0.25,
            "mane widens lion silhouette"
        );
        assert!(lion.faces.iter().any(|f| f.3 == "mane"));
        assert!(!wolf.faces.iter().any(|f| f.3 == "mane"));
    }
    #[test]
    fn body_profiles_have_a_tuck_and_species_specific_withers() {
        let wolf = mesh("WOLF", "TORSO", "torso").unwrap();
        let bear = mesh("BEAR", "TORSO", "torso").unwrap();
        let belly = |x: f32| {
            wolf.vertices
                .iter()
                .filter(|v| (v[0] - x).abs() < 0.01)
                .map(|v| v[1])
                .fold(f32::INFINITY, f32::min)
        };
        assert!(belly(-0.45) > belly(0.34) + 0.35);
        assert!(bear.vertices.iter().any(|v| v[1] > 1.15));
    }
    #[test]
    fn composite_heads_share_species_surface_without_changing_rig() {
        assert!(std::ptr::eq(
            mesh("WOLF", "HEAD", "head_0").unwrap(),
            mesh("CERBERUS", "HEAD", "head_2").unwrap()
        ));
        assert!(mesh("MINOTAUR", "HEAD", "head_0").is_some());
        assert!(mesh("HYDRA", "HEAD", "head_4").is_some());
        assert!(mesh("HUMAN", "HEAD", "head_0").is_none());
    }
    #[test]
    fn snake_worm_and_sandworm_have_distinct_heads() {
        let snake = mesh("SNAKE", "HEAD", "head_0").unwrap();
        let worm = mesh("WORM", "HEAD", "head_0").unwrap();
        let sand = mesh("SAND_WORM", "HEAD", "head_0").unwrap();
        assert!(snake.faces.iter().any(|f| f.3 == "eye"));
        assert!(!worm.faces.iter().any(|f| f.3 == "eye" || f.3 == "tooth"));
        assert!(sand.faces.iter().any(|f| f.3 == "tooth"));
        assert!(!sand.faces.iter().any(|f| f.3 == "eye"));
        assert!(std::ptr::eq(
            snake,
            mesh("GORGON", "LOBE", "v2_gorgon_snake_tip_0_2").unwrap()
        ));
    }

    #[test]
    fn feature_nodes_own_tusks_trunks_and_rhino_horns() {
        let elephant = mesh("ELEPHANT", "HEAD", "head_0").unwrap();
        assert!(!elephant
            .faces
            .iter()
            .any(|f| f.3 == "tooth" || f.3 == "horn"));
        let trunk = mesh("ELEPHANT", "TENTACLE", "trunk").unwrap();
        assert!(trunk.vertices.iter().any(|v| v[1] < -0.8));
        assert!(mesh("MAMMOTH", "TUSK", "tusk_0")
            .unwrap()
            .vertices
            .iter()
            .any(|v| v[0] > 2.2));
        assert!(mesh("RHINO", "HORN", "horn_0")
            .unwrap()
            .vertices
            .iter()
            .any(|v| v[1] > 1.3));
    }
}
