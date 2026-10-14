//! Anatomical, open-work V2 skeleton parts. These assets are never used by V1.
//!
//! Coordinates follow the renderer: +X is forward, +Y is up, and Z is lateral.
//! In particular, the ribs and pelvis are disconnected bony rails with genuine
//! empty space between them, rather than a bone-colored solid body.

use super::{v2_append_blob, MeshAsset, V3};
use std::{f32::consts::TAU, sync::OnceLock};

const BONE: &str = "face_tusk";

fn empty() -> MeshAsset {
    MeshAsset {
        vertices: Vec::new(),
        faces: Vec::new(),
    }
}

fn point(p: [f32; 3]) -> V3 {
    V3::new(p[0], p[1], p[2])
}

fn blob(mesh: &mut MeshAsset, p: [f32; 3], s: [f32; 3], role: &str) {
    v2_append_blob(mesh, point(p), point(s), role);
}

/// Sweep an eight-sided cortical tube along a path, with a true narrow shaft
/// or an open, curved rib. No solid surfaces are added between separate paths.
fn sweep(mesh: &mut MeshAsset, path: &[([f32; 3], f32)], role: &str, closed: bool) {
    const SIDES: usize = 8;
    assert!(path.len() >= 2);
    let start = mesh.vertices.len();
    for (i, &(p, radius)) in path.iter().enumerate() {
        let prev = if i == 0 {
            if closed {
                path.len() - 1
            } else {
                0
            }
        } else {
            i - 1
        };
        let next = if i + 1 == path.len() {
            if closed {
                0
            } else {
                i
            }
        } else {
            i + 1
        };
        let tangent = (point(path[next].0) - point(path[prev].0)).normalized();
        let reference = if tangent.y.abs() > 0.85 {
            V3::new(0.0, 0.0, 1.0)
        } else {
            V3::new(0.0, 1.0, 0.0)
        };
        let u = tangent.cross(reference).normalized();
        let v = tangent.cross(u).normalized();
        for side in 0..SIDES {
            let angle = TAU * side as f32 / SIDES as f32;
            let p = point(p) + u * (radius * angle.cos()) + v * (radius * angle.sin());
            mesh.vertices.push([p.x, p.y, p.z]);
        }
    }
    let bands = if closed { path.len() } else { path.len() - 1 };
    for ring in 0..bands {
        let next = (ring + 1) % path.len();
        for side in 0..SIDES {
            let a = start + ring * SIDES + side;
            let b = start + ring * SIDES + (side + 1) % SIDES;
            let c = start + next * SIDES + side;
            let d = start + next * SIDES + (side + 1) % SIDES;
            mesh.faces
                .extend([(a, b, c, role.into()), (b, d, c, role.into())]);
        }
    }
    if !closed {
        for (ring, reverse) in [(0, true), (path.len() - 1, false)] {
            let center = mesh.vertices.len();
            mesh.vertices.push(path[ring].0);
            for side in 0..SIDES {
                let a = start + ring * SIDES + side;
                let b = start + ring * SIDES + (side + 1) % SIDES;
                mesh.faces.push(if reverse {
                    (center, b, a, role.into())
                } else {
                    (center, a, b, role.into())
                });
            }
        }
    }
}

fn tube(mesh: &mut MeshAsset, points: &[[f32; 3]], radius: f32, role: &str) {
    let path: Vec<_> = points.iter().map(|&p| (p, radius)).collect();
    sweep(mesh, &path, role, false);
}

/// A thick, curved bony plate, used only for the small iliac wings. The central
/// pelvic opening remains unfilled. The outline may be slightly non-planar.
fn plate(mesh: &mut MeshAsset, outline: &[[f32; 3]], thickness: f32) {
    let n = outline.len();
    let start = mesh.vertices.len();
    for offset in [-thickness, thickness] {
        for p in outline {
            mesh.vertices.push([p[0] + offset, p[1], p[2]]);
        }
    }
    for i in 1..n - 1 {
        mesh.faces.extend([
            (start, start + i + 1, start + i, BONE.into()),
            (start + n, start + n + i, start + n + i + 1, BONE.into()),
        ]);
    }
    for i in 0..n {
        let a = start + i;
        let b = start + (i + 1) % n;
        mesh.faces
            .extend([(a, b, a + n, BONE.into()), (b, b + n, a + n, BONE.into())]);
    }
}

fn skull() -> MeshAsset {
    let mut mesh = empty();
    // Vault plus occiput. The anterior lower bands deliberately omit their
    // central faces: the eye/nasal holes below are not painted over a sphere.
    let rings = [
        (-0.42, 0.24, -0.40, 0.42),
        (-0.12, 0.38, -0.61, 0.65),
        (0.25, 0.48, -0.66, 0.73),
        (0.49, 0.52, -0.61, 0.69),
        (0.73, 0.44, -0.51, 0.55),
        (0.92, 0.24, -0.34, 0.34),
    ];
    const SIDES: usize = 24;
    for &(y, front, back, width) in &rings {
        for side in 0..SIDES {
            let angle = TAU * side as f32 / SIDES as f32;
            let c = angle.cos();
            mesh.vertices.push([
                if c > 0.0 { front * c } else { -back * c },
                y,
                width * angle.sin(),
            ]);
        }
    }
    for band in 0..rings.len() - 1 {
        for side in 0..SIDES {
            let angle = TAU * (side as f32 + 0.5) / SIDES as f32;
            if band < 3 && angle.cos() > 0.32 {
                continue;
            }
            let a = band * SIDES + side;
            let b = band * SIDES + (side + 1) % SIDES;
            mesh.faces.extend([
                (a, b, a + SIDES, BONE.into()),
                (b, b + SIDES, a + SIDES, BONE.into()),
            ]);
        }
    }
    let top = mesh.vertices.len();
    mesh.vertices.push([-0.06, 1.0, 0.0]);
    for side in 0..SIDES {
        mesh.faces.push((
            top,
            (rings.len() - 1) * SIDES + side,
            (rings.len() - 1) * SIDES + (side + 1) % SIDES,
            BONE.into(),
        ));
    }

    for side in [-1.0, 1.0] {
        // Recessed orbital bowl with an independent bony rim. Front-facing
        // rays encounter dark walls behind the hole, never a white eyeball.
        let center_z = side * 0.345;
        let mut rim = Vec::new();
        let bowl_start = mesh.vertices.len();
        for i in 0..16 {
            let angle = TAU * i as f32 / 16.0;
            let z = center_z + 0.255 * angle.cos();
            let y = 0.11 + 0.235 * angle.sin();
            let x = 0.565 - (z.abs() - 0.20).max(0.0) * 0.28;
            rim.push(([x + 0.012, y, z], 0.057));
            mesh.vertices.push([x - 0.016, y, z]);
        }
        let bowl_back = mesh.vertices.len();
        mesh.vertices.push([0.235, 0.09, center_z]);
        for i in 0..16 {
            mesh.faces.push((
                bowl_back,
                bowl_start + i,
                bowl_start + (i + 1) % 16,
                "socket".into(),
            ));
        }
        sweep(&mut mesh, &rim, BONE, true);
        // Heavy brow and a thin cheek arch create the human skull silhouette.
        tube(
            &mut mesh,
            &[
                [0.49, 0.39, side * 0.08],
                [0.56, 0.40, side * 0.30],
                [0.48, 0.35, side * 0.62],
            ],
            0.076,
            BONE,
        );
        tube(
            &mut mesh,
            &[
                [-0.18, -0.12, side * 0.62],
                [0.14, -0.22, side * 0.72],
                [0.44, -0.24, side * 0.60],
                [0.57, -0.35, side * 0.34],
            ],
            0.075,
            BONE,
        );
        tube(
            &mut mesh,
            &[
                [0.56, -0.15, side * 0.13],
                [0.60, -0.37, side * 0.17],
                [0.57, -0.48, side * 0.30],
            ],
            0.047,
            BONE,
        );
    }
    // Nasal aperture: a real triangular gap with a recessed black interior,
    // flanked by the maxilla. No projecting soft-tissue nose or muzzle.
    let n = mesh.vertices.len();
    mesh.vertices.extend([
        [0.56, 0.09, 0.0],
        [0.59, -0.30, -0.135],
        [0.59, -0.30, 0.135],
        [0.32, -0.18, 0.0],
    ]);
    mesh.faces.extend([
        (n, n + 1, n + 3, "socket".into()),
        (n + 1, n + 2, n + 3, "socket".into()),
        (n + 2, n, n + 3, "socket".into()),
    ]);
    tube(
        &mut mesh,
        &[[0.52, 0.34, 0.0], [0.59, 0.10, 0.0]],
        0.043,
        BONE,
    );

    // Separate horseshoe maxilla, mandible, and discrete teeth. The mouth is
    // an open slit; a recessed inner panel keeps it dark from frontal angles.
    let mut upper = Vec::new();
    let mut lower = Vec::new();
    for i in 0..=12 {
        let angle = std::f32::consts::PI * i as f32 / 12.0;
        let x = 0.14 + 0.47 * angle.sin();
        let z = 0.405 * angle.cos();
        upper.push(([x, -0.455, z], 0.065));
        lower.push((
            [x - 0.02, -0.85 + 0.06 * angle.cos().abs(), z * 1.08],
            0.085,
        ));
        if i > 0 && i < 12 {
            blob(
                &mut mesh,
                [x + 0.025, -0.55, z],
                [0.065, 0.08, 0.052],
                "tooth",
            );
            blob(
                &mut mesh,
                [x + 0.005, -0.755, z * 1.03],
                [0.062, 0.055, 0.049],
                "tooth",
            );
        }
    }
    sweep(&mut mesh, &upper, BONE, false);
    sweep(&mut mesh, &lower, BONE, false);
    for side in [-1.0, 1.0] {
        tube(
            &mut mesh,
            &[
                [-0.14, -0.26, side * 0.57],
                [-0.08, -0.54, side * 0.55],
                [0.02, -0.75, side * 0.48],
                [0.14, -0.80, side * 0.44],
            ],
            0.083,
            BONE,
        );
    }
    blob(&mut mesh, [0.37, -0.655, 0.0], [0.08, 0.075, 0.35], "mouth");
    mesh
}

fn ribs() -> MeshAsset {
    let mut mesh = empty();
    // Six independently swept pairs. Their fall toward the front, variable
    // width, and smaller free lower ends read as ribs in every quarter turn.
    for (row, &(height, width, depth)) in [
        (0.74, 0.72, 0.50),
        (0.46, 0.91, 0.62),
        (0.18, 1.00, 0.68),
        (-0.10, 0.97, 0.67),
        (-0.38, 0.86, 0.59),
        (-0.65, 0.70, 0.47),
    ]
    .iter()
    .enumerate()
    {
        for side in [-1.0, 1.0] {
            let mut path = Vec::new();
            let extent = if row >= 4 { 0.84 } else { 1.0 };
            for i in 0..=14 {
                let t = extent * i as f32 / 14.0;
                let angle = std::f32::consts::PI * t;
                path.push((
                    [
                        -depth * angle.cos(),
                        height - 0.15 * angle.sin() - 0.065 * t,
                        side * (0.10 + width * angle.sin()),
                    ],
                    0.058 + 0.006 * (1.0 - t),
                ));
            }
            sweep(&mut mesh, &path, BONE, false);
        }
    }
    // The vertebral column stays at the back, so the intercostal openings are
    // transparent. Individual processes break up an otherwise smooth pole.
    tube(
        &mut mesh,
        &[
            [-0.53, -1.0, 0.0],
            [-0.60, -0.3, 0.0],
            [-0.56, 0.45, 0.0],
            [-0.48, 1.0, 0.0],
        ],
        0.095,
        BONE,
    );
    for i in 0..9 {
        let y = -0.94 + i as f32 * 0.235;
        blob(&mut mesh, [-0.56, y, 0.0], [0.16, 0.083, 0.17], BONE);
        tube(
            &mut mesh,
            &[[-0.59, y, 0.0], [-0.78, y - 0.06, 0.0]],
            0.047,
            BONE,
        );
    }
    tube(
        &mut mesh,
        &[[0.49, 0.86, 0.0], [0.67, 0.35, 0.0], [0.64, -0.40, 0.0]],
        0.075,
        BONE,
    );
    // Clavicles leave the thoracic inlet open rather than capping the cage.
    for side in [-1.0, 1.0] {
        tube(
            &mut mesh,
            &[
                [0.47, 0.87, 0.0],
                [0.43, 0.96, side * 0.36],
                [0.16, 0.98, side * 0.75],
            ],
            0.066,
            BONE,
        );
        tube(
            &mut mesh,
            &[
                [-0.48, 0.75, side * 0.22],
                [-0.43, 0.56, side * 0.66],
                [-0.48, 0.14, side * 0.46],
            ],
            0.073,
            BONE,
        );
    }
    mesh
}

fn long_bone() -> MeshAsset {
    let mut mesh = empty();
    // Diaphysis is less than half the joint diameter, with a gentle anatomical
    // bow. Split condyles at each end remain distinct from the shaft.
    sweep(
        &mut mesh,
        &[
            ([0.00, -0.82, 0.0], 0.225),
            ([0.02, -0.60, 0.0], 0.175),
            ([0.05, -0.23, 0.025], 0.135),
            ([0.045, 0.28, 0.018], 0.140),
            ([0.005, 0.64, 0.0], 0.185),
            ([-0.025, 0.81, 0.0], 0.230),
        ],
        BONE,
        false,
    );
    for side in [-1.0, 1.0] {
        blob(
            &mut mesh,
            [0.005, -0.82, side * 0.17],
            [0.28, 0.18, 0.235],
            BONE,
        );
        blob(
            &mut mesh,
            [-0.02, 0.81, side * 0.145],
            [0.255, 0.19, 0.22],
            BONE,
        );
    }
    mesh
}

fn pelvis() -> MeshAsset {
    let mut mesh = empty();
    // Sacrum, with separate broad iliac blades and an open pelvic bowl.
    plate(
        &mut mesh,
        &[
            [-0.51, 0.64, -0.27],
            [-0.51, 0.64, 0.27],
            [-0.40, -0.48, 0.0],
        ],
        0.075,
    );
    for side in [-1.0, 1.0] {
        plate(
            &mut mesh,
            &[
                [-0.44, 0.45, side * 0.21],
                [-0.50, 0.73, side * 0.48],
                [-0.33, 0.82, side * 0.94],
                [0.03, 0.63, side * 1.05],
                [0.16, 0.10, side * 0.75],
                [-0.06, -0.13, side * 0.54],
            ],
            0.055,
        );
        tube(
            &mut mesh,
            &[
                [-0.44, 0.51, side * 0.27],
                [-0.38, 0.77, side * 0.68],
                [-0.07, 0.75, side * 1.02],
            ],
            0.080,
            BONE,
        );
        // Ischium and pubic rami surround a genuine obturator foramen.
        let arch = [
            ([0.10, 0.02, side * 0.73], 0.105),
            ([-0.03, -0.41, side * 0.73], 0.095),
            ([0.16, -0.78, side * 0.51], 0.085),
            ([0.48, -0.74, side * 0.13], 0.085),
            ([0.50, -0.28, side * 0.15], 0.075),
            ([0.29, -0.12, side * 0.50], 0.085),
        ];
        sweep(&mut mesh, &arch, BONE, true);
        blob(
            &mut mesh,
            [0.02, -0.10, side * 0.80],
            [0.20, 0.22, 0.12],
            BONE,
        );
    }
    tube(
        &mut mesh,
        &[[0.48, -0.61, -0.16], [0.52, -0.61, 0.16]],
        0.091,
        BONE,
    );
    mesh
}

fn hand() -> MeshAsset {
    let mut mesh = empty();
    // Wrist, four separated metacarpals and jointed fingers, plus a thumb.
    blob(&mut mesh, [0.0, 0.73, 0.0], [0.19, 0.18, 0.34], BONE);
    for finger in 0..4 {
        let z = (finger as f32 - 1.5) * 0.22;
        let length = if finger == 0 || finger == 3 {
            0.78
        } else {
            1.0
        };
        tube(
            &mut mesh,
            &[[0.0, 0.65, z * 0.6], [0.01, 0.10, z]],
            0.071,
            BONE,
        );
        tube(
            &mut mesh,
            &[
                [0.01, 0.09, z],
                [0.12, -0.38 * length, z * 1.1],
                [0.23, -0.80 * length, z * 1.1],
            ],
            0.055,
            BONE,
        );
        blob(&mut mesh, [0.01, 0.08, z], [0.095, 0.10, 0.092], BONE);
        blob(
            &mut mesh,
            [0.12, -0.38 * length, z * 1.1],
            [0.078, 0.08, 0.074],
            BONE,
        );
    }
    tube(
        &mut mesh,
        &[
            [0.02, 0.48, -0.24],
            [0.10, 0.15, -0.55],
            [0.24, -0.22, -0.63],
        ],
        0.073,
        BONE,
    );
    mesh
}

fn foot() -> MeshAsset {
    let mut mesh = empty();
    // Heel/ankle behind five free metatarsals, facing +X like the head.
    blob(&mut mesh, [-0.48, 0.03, 0.0], [0.30, 0.29, 0.32], BONE);
    blob(&mut mesh, [-0.28, 0.28, 0.0], [0.23, 0.25, 0.29], BONE);
    for toe in 0..5 {
        let z = (toe as f32 - 2.0) * 0.18;
        let end = 1.0 - (toe as f32 - 1.0).abs() * 0.10;
        tube(
            &mut mesh,
            &[
                [-0.25, 0.02, z * 0.6],
                [0.41, -0.10, z],
                [end, -0.16, z * 1.1],
            ],
            if toe == 0 { 0.083 } else { 0.057 },
            BONE,
        );
        blob(&mut mesh, [0.44, -0.10, z], [0.10, 0.08, 0.078], BONE);
    }
    mesh
}

pub(super) fn mesh(kind: &str) -> &'static MeshAsset {
    static MESHES: OnceLock<[MeshAsset; 6]> = OnceLock::new();
    let meshes = MESHES.get_or_init(|| [skull(), ribs(), long_bone(), pelvis(), hand(), foot()]);
    &meshes[match kind {
        "skull" => 0,
        "ribs" | "ribcage" => 1,
        "pelvis" => 3,
        "hand" => 4,
        "foot" => 5,
        _ => 2,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    // Orthographic ray from +X. Check the visible geometry itself, not just a
    // role name or the existence of separately generated mesh components.
    fn front_hit(mesh: &MeshAsset, y: f32, z: f32) -> Option<&str> {
        let mut nearest = f32::NEG_INFINITY;
        let mut hit = None;
        for &(a, b, c, ref role) in &mesh.faces {
            let [a, b, c] = [mesh.vertices[a], mesh.vertices[b], mesh.vertices[c]];
            let area = (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]);
            if area.abs() < 1e-7 {
                continue;
            }
            let u = ((b[1] - y) * (c[2] - z) - (b[2] - z) * (c[1] - y)) / area;
            let v = ((c[1] - y) * (a[2] - z) - (c[2] - z) * (a[1] - y)) / area;
            let w = 1.0 - u - v;
            if u >= 0.0 && v >= 0.0 && w >= 0.0 {
                let x = u * a[0] + v * b[0] + w * c[0];
                if x > nearest {
                    nearest = x;
                    hit = Some(role.as_str());
                }
            }
        }
        hit
    }

    #[test]
    fn skeletal_parts_are_finite_bounded_meshes() {
        for kind in ["skull", "ribs", "long_bone", "pelvis", "hand", "foot"] {
            let mesh = mesh(kind);
            assert!(!mesh.vertices.is_empty(), "{kind}");
            assert!(
                mesh.vertices
                    .iter()
                    .flatten()
                    .all(|v| v.is_finite() && v.abs() < 1.25),
                "{kind}"
            );
            assert!(
                mesh.faces
                    .iter()
                    .all(|(a, b, c, _)| *a < mesh.vertices.len()
                        && *b < mesh.vertices.len()
                        && *c < mesh.vertices.len()),
                "{kind}"
            );
            assert!(
                mesh.faces.iter().any(|(_, _, _, role)| role == BONE),
                "{kind}"
            );
        }
    }

    #[test]
    fn skull_has_recessed_sockets_and_separate_teeth() {
        let mesh = mesh("skull");
        assert!(
            mesh.faces
                .iter()
                .filter(|(_, _, _, r)| r == "socket")
                .count()
                >= 32
        );
        assert!(mesh.faces.iter().any(|(_, _, _, r)| r == "tooth"));
        assert!(mesh.faces.iter().any(|(_, _, _, r)| r == "mouth"));
        assert!(!mesh
            .faces
            .iter()
            .any(|(_, _, _, r)| r == "eye" || r == "eye_white"));
        assert_eq!(front_hit(mesh, 0.11, -0.345), Some("socket"));
        assert_eq!(front_hit(mesh, 0.11, 0.345), Some("socket"));
        assert_eq!(front_hit(mesh, -0.20, 0.0), Some("socket"));
        assert_eq!(front_hit(mesh, -0.55, 0.0), Some("tooth"));
        assert_eq!(front_hit(mesh, -0.665, 0.0), Some("mouth"));
    }

    #[test]
    fn ribcage_and_pelvis_have_actual_empty_spaces() {
        assert_eq!(front_hit(mesh("ribs"), 0.45, 0.78), None);
        assert_eq!(front_hit(mesh("ribs"), 0.31, 0.78), Some(BONE));
        assert_eq!(front_hit(mesh("pelvis"), -0.44, 0.40), None);
        assert_eq!(front_hit(mesh("pelvis"), -0.44, -0.40), None);
    }

    #[test]
    fn long_bones_have_thin_shafts_and_enlarged_joint_ends() {
        let mesh = mesh("long_bone");
        let shaft = mesh
            .vertices
            .iter()
            .filter(|v| v[1].abs() < 0.5)
            .map(|v| v[2].abs())
            .fold(0.0_f32, f32::max);
        let ends = mesh
            .vertices
            .iter()
            .filter(|v| v[1].abs() > 0.7)
            .map(|v| v[2].abs())
            .fold(0.0_f32, f32::max);
        assert!(shaft < 0.18);
        assert!(ends > shaft * 2.0);
    }
}
