//! V2-only, shallow-front humanoid faces. Legacy mesh recipes never use these.
use super::{v2_append_blob, MeshAsset, OnceLock, V3};

fn blob(mesh: &mut MeshAsset, p: [f32; 3], s: [f32; 3], role: &str) {
    v2_append_blob(
        mesh,
        V3::new(p[0], p[1], p[2]),
        V3::new(s[0], s[1], s[2]),
        role,
    );
}

fn oriented_blob(mesh: &mut MeshAsset, p: [f32; 3], s: [f32; 3], yaw: f32, role: &str) {
    let start = mesh.vertices.len();
    blob(mesh, [0.0, 0.0, 0.0], s, role);
    let (sn, cs) = yaw.sin_cos();
    for v in &mut mesh.vertices[start..] {
        let x = v[0];
        let z = v[2];
        *v = [p[0] + cs * x - sn * z, p[1] + v[1], p[2] + sn * x + cs * z];
    }
}

fn ribbon(mesh: &mut MeshAsset, points: &[[f32; 3]], width: f32, role: &str) {
    let start = mesh.vertices.len();
    for p in points {
        mesh.vertices
            .extend([[p[0], p[1] - width, p[2]], [p[0], p[1] + width, p[2]]]);
    }
    for i in 0..points.len() - 1 {
        let a = start + i * 2;
        mesh.faces.extend([
            (a, a + 1, a + 2, role.into()),
            (a + 1, a + 3, a + 2, role.into()),
        ]);
    }
}

fn hair_lock(mesh: &mut MeshAsset, path: &[(V3, f32)]) {
    let start = mesh.vertices.len();
    for (ring, (center, radius)) in path.iter().enumerate() {
        let before = path[ring.saturating_sub(1)].0;
        let after = path[(ring + 1).min(path.len() - 1)].0;
        let tangent = (after - before).normalized();
        let reference = if tangent.z.abs() < 0.85 {
            V3::new(0.0, 0.0, 1.0)
        } else {
            V3::new(1.0, 0.0, 0.0)
        };
        let side = tangent.cross(reference).normalized();
        let up = tangent.cross(side).normalized();
        for i in 0..8 {
            let angle = std::f32::consts::TAU * i as f32 / 8.0;
            let v = *center + (side * angle.cos() + up * angle.sin()) * *radius;
            mesh.vertices.push([v.x, v.y, v.z]);
        }
    }
    for ring in 0..path.len() - 1 {
        for i in 0..8 {
            let a = start + ring * 8 + i;
            let b = start + ring * 8 + (i + 1) % 8;
            let role = if i == 2 { "hair_ridge" } else { "hair" };
            mesh.faces
                .extend([(a, b, a + 8, role.into()), (b, b + 8, a + 8, role.into())]);
        }
    }
}

fn flowing_hair(mesh: &mut MeshAsset) {
    // Unequal, curved clumps with swept tips break up the cap and rigid curtains.
    for lock in 0..7 {
        let phase = lock as f32 * 0.73;
        let theta = std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * lock as f32 / 6.0;
        let root_x = theta.cos() * 0.43 - 0.10;
        let root_z = theta.sin() * 0.48;
        let side = if root_z >= 0.0 { 1.0 } else { -1.0 };
        let path: Vec<_> = (0..=10)
            .map(|ring| {
                let t = ring as f32 / 10.0;
                let x = root_x - 0.34 * (std::f32::consts::PI * t).sin()
                    + 0.22 * (std::f32::consts::TAU * t + phase).sin() * t;
                let z = root_z * (1.0 + 0.24 * (std::f32::consts::PI * t).sin())
                    + side * (0.30 * (std::f32::consts::TAU * t + phase).sin() * t + 0.22 * t);
                let y = 0.76 - t * (2.18 + 0.25 * phase.sin());
                (V3::new(x, y, z), 0.205 * (1.0 - t).sqrt())
            })
            .collect();
        hair_lock(mesh, &path);
    }
    let fringe = [
        (V3::new(-0.15, 0.88, -0.40), 0.12),
        (V3::new(0.12, 0.83, -0.40), 0.18),
        (V3::new(0.36, 0.70, -0.20), 0.16),
        (V3::new(0.38, 0.62, 0.08), 0.12),
        (V3::new(0.22, 0.60, 0.36), 0.0),
    ];
    hair_lock(mesh, &fringe);
}

// Each section is (height, front, back, half-width). A shallow continuous
// forehead-to-chin plane replaces the old projecting muzzle and separate jaw.
fn skull(sections: &[(f32, f32, f32, f32)], skin: &str, hair: bool) -> MeshAsset {
    let mut mesh = MeshAsset {
        vertices: Vec::new(),
        faces: Vec::new(),
    };
    const SIDES: usize = 20;
    for &(y, front, back, width) in sections {
        for i in 0..SIDES {
            let theta = std::f32::consts::TAU * i as f32 / SIDES as f32;
            let c = theta.cos();
            mesh.vertices.push([
                if c >= 0.0 { front * c } else { -back * c },
                y,
                width * theta.sin(),
            ]);
        }
    }
    for (ring, section) in sections.iter().enumerate().take(sections.len() - 1) {
        for i in 0..SIDES {
            let a = ring * SIDES + i;
            let b = ring * SIDES + (i + 1) % SIDES;
            let role = if hair && (section.0 >= 0.65 || (i > 6 && i < 14 && section.0 > -0.5)) {
                "hair"
            } else {
                skin
            };
            mesh.faces.push((a, b, a + SIDES, role.into()));
            mesh.faces.push((b, b + SIDES, a + SIDES, role.into()));
        }
    }
    mesh
}

fn eyes(mesh: &mut MeshAsset, wide: f32, y: f32, squint: f32) {
    let (front, radius) = if wide > 0.5 {
        (0.50_f32, 0.84_f32)
    } else {
        (0.53_f32, 0.66_f32)
    };
    let x = front * (1.0 - (wide / radius).powi(2)).sqrt();
    let yaw = (wide / (radius * radius)).atan2(x / (front * front));
    for side in [-1.0, 1.0] {
        let normal = [yaw.cos(), side * yaw.sin()];
        oriented_blob(
            mesh,
            [x + normal[0] * 0.015, y, side * wide + normal[1] * 0.015],
            [0.025, 0.12 * squint, 0.16],
            side * yaw,
            "eye_white",
        );
        oriented_blob(
            mesh,
            [x + normal[0] * 0.042, y, side * wide + normal[1] * 0.042],
            [0.006, 0.082 * squint, 0.07],
            side * yaw,
            "eye_dark",
        );
        oriented_blob(
            mesh,
            [
                x + normal[0] * 0.050,
                y + 0.045,
                side * wide + normal[1] * 0.050,
            ],
            [0.003, 0.022, 0.019],
            side * yaw,
            "eye_white",
        );
    }
}

fn human(mermaid: bool, cyclops: bool) -> MeshAsset {
    let skin = if mermaid { "skin_soft" } else { "face_skin" };
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.84, 0.28, -0.32, 0.32),
            (-0.58, 0.49, -0.49, 0.48),
            (-0.25, 0.53, -0.61, 0.61),
            (0.15, 0.53, -0.65, 0.66),
            (0.48, 0.49, -0.65, 0.62),
            (0.7, 0.42, -0.57, 0.53),
            (0.93, 0.2, -0.36, 0.29),
            (1.0, 0.0, 0.0, 0.0),
        ],
        skin,
        true,
    );
    for side in [-1.0, 1.0] {
        blob(
            &mut mesh,
            [-0.06, -0.07, side * 0.64],
            [0.09, 0.18, 0.06],
            skin,
        );
    }
    // Nose bridge is a small face feature, never a cone or a muzzle.
    blob(&mut mesh, [0.52, -0.04, 0.0], [0.085, 0.18, 0.09], skin);
    if cyclops {
        blob(
            &mut mesh,
            [0.54, 0.23, 0.0],
            [0.09, 0.23, 0.30],
            "eye_white",
        );
        blob(
            &mut mesh,
            [0.62, 0.23, 0.0],
            [0.035, 0.15, 0.13],
            "eye_dark",
        );
    } else {
        eyes(&mut mesh, 0.43, 0.17, 1.0);
    }
    let mouth: Vec<_> = (-4..=4)
        .map(|i| {
            let z = i as f32 * 0.06;
            [
                0.51 * (1.0 - (z / 0.55).powi(2)).sqrt() + 0.022,
                -0.48 + z * z * 1.6,
                z,
            ]
        })
        .collect();
    ribbon(&mut mesh, &mouth, 0.025, "mouth");
    if mermaid {
        flowing_hair(&mut mesh);
    }
    mesh
}

fn ape_base() -> MeshAsset {
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.83, 0.46, -0.36, 0.62),
            (-0.52, 0.60, -0.54, 0.77),
            (-0.15, 0.56, -0.62, 0.84),
            (0.2, 0.50, -0.68, 0.84),
            (0.55, 0.43, -0.66, 0.73),
            (0.85, 0.26, -0.45, 0.49),
            (1.0, 0.0, 0.0, 0.0),
        ],
        "face_skin",
        false,
    );
    eyes(&mut mesh, 0.61, 0.15, 0.72);
    for side in [-1.0, 1.0] {
        blob(
            &mut mesh,
            [0.36, 0.32, side * 0.60],
            [0.21, 0.12, 0.24],
            "face_skin",
        );
        blob(
            &mut mesh,
            [-0.08, 0.16, side * 0.88],
            [0.18, 0.28, 0.24],
            "face_skin",
        );
        // Curved upward lower tusks, rooted in the broad jaw rather than a beak.
        let offset = mesh.vertices.len();
        for (y, radius, x) in [
            (-0.57, 0.10, 0.61),
            (-0.37, 0.12, 0.68),
            (-0.12, 0.07, 0.70),
            (0.10, 0.0, 0.64),
        ] {
            for i in 0..8 {
                let t = std::f32::consts::TAU * i as f32 / 8.0;
                mesh.vertices
                    .push([x + radius * t.cos(), y, side * 0.45 + radius * t.sin()]);
            }
        }
        for ring in 0..3 {
            for i in 0..8 {
                let a = offset + ring * 8 + i;
                let b = offset + ring * 8 + (i + 1) % 8;
                mesh.faces.push((a, b, a + 8, "face_tusk".into()));
                mesh.faces.push((b, b + 8, a + 8, "face_tusk".into()));
            }
        }
    }
    blob(
        &mut mesh,
        [0.53, -0.07, 0.0],
        [0.14, 0.17, 0.22],
        "face_skin",
    );
    blob(&mut mesh, [0.59, -0.46, 0.0], [0.045, 0.065, 0.38], "mouth");
    mesh
}

// A sculpted humanoid cranium has flatter facial planes and an angular mandible.
// Keep this separate from the softly rounded human/mermaid and legacy ape heads.
fn heavy_skull(sections: &[(f32, f32, f32, f32)]) -> MeshAsset {
    let mut mesh = skull(sections, "face_skin", false);
    const SIDES: usize = 20;
    for (ring, &(_, front, _, width)) in sections.iter().enumerate() {
        for i in 0..SIDES {
            let theta = std::f32::consts::TAU * i as f32 / SIDES as f32;
            let c = theta.cos();
            if c >= 0.0 {
                // Broad front, distinct side planes; no spherical muzzle.
                mesh.vertices[ring * SIDES + i][0] = front * c.powf(0.64);
                mesh.vertices[ring * SIDES + i][2] =
                    width * theta.sin().signum() * theta.sin().abs().powf(0.88);
            }
        }
    }
    mesh
}

// A closed shallow, faceted relief. Polygon vertices lie on the visible surface;
// a recessed copy joins it to the skull instead of leaving a floating plane.
fn relief(mesh: &mut MeshAsset, points: &[[f32; 3]], depth: f32, role: &str) {
    let start = mesh.vertices.len();
    let n = points.len();
    mesh.vertices.extend_from_slice(points);
    mesh.vertices
        .extend(points.iter().map(|p| [p[0] - depth, p[1], p[2]]));
    for i in 1..n - 1 {
        mesh.faces.extend([
            (start, start + i, start + i + 1, role.into()),
            (start + n, start + n + i + 1, start + n + i, role.into()),
        ]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        mesh.faces.extend([
            (start + i, start + j, start + n + i, role.into()),
            (start + j, start + n + j, start + n + i, role.into()),
        ]);
    }
}

fn heavy_eyes(mesh: &mut MeshAsset, troll: bool) {
    let (height, width, spread, front) = if troll {
        (0.066, 0.145, 0.36, 0.48)
    } else {
        (0.080, 0.170, 0.39, 0.52)
    };
    for side in [-1.0, 1.0] {
        // Socket, almond-shaped sclera and a restrained pupil are all surface
        // geometry. The eyes stay recessed under the brow, never stuck-on balls.
        let y = if troll { 0.12 } else { 0.14 };
        let yaw = side * 0.48;
        oriented_blob(
            mesh,
            [front - 0.024, y, side * spread],
            [0.027, height * 1.6, width * 1.23],
            yaw,
            "socket",
        );
        let mut almond = Vec::new();
        for i in 0..12 {
            let t = std::f32::consts::TAU * i as f32 / 12.0;
            let dz = width * t.cos();
            almond.push([
                front + 0.022 - dz * 0.48,
                y + height * t.sin() * t.sin().abs().sqrt() + dz * 0.11,
                side * (spread + dz),
            ]);
        }
        relief(mesh, &almond, 0.012, "eye_white");
        oriented_blob(
            mesh,
            [front + 0.039, y + 0.004, side * spread],
            [0.014, height * 0.87, width * 0.36],
            yaw,
            "eye_dark",
        );

        let brow = if troll {
            vec![
                [0.60, 0.23, side * 0.12],
                [0.61, 0.35, side * 0.17],
                [0.53, 0.41, side * 0.38],
                [0.36, 0.32, side * 0.62],
                [0.40, 0.19, side * 0.57],
                [0.52, 0.20, side * 0.34],
            ]
        } else {
            vec![
                [0.62, 0.23, side * 0.13],
                [0.60, 0.36, side * 0.16],
                [0.52, 0.41, side * 0.42],
                [0.37, 0.33, side * 0.67],
                [0.40, 0.20, side * 0.61],
                [0.53, 0.19, side * 0.35],
            ]
        };
        relief(mesh, &brow, 0.14, "brow");
        // Cheekbone planes support the orbital rim and meet the jaw naturally.
        let cheek = [
            [front - 0.04, 0.005, side * 0.27],
            [front + 0.01, -0.03, side * 0.48],
            [0.32, 0.02, side * if troll { 0.65 } else { 0.76 }],
            [0.35, -0.24, side * if troll { 0.58 } else { 0.69 }],
            [front - 0.01, -0.25, side * 0.34],
        ];
        relief(mesh, &cheek, 0.10, "face_skin");
    }
}

fn pointed_ears(mesh: &mut MeshAsset, troll: bool) {
    for side in [-1.0, 1.0] {
        let (root, tip, rise) = if troll {
            (0.65, 1.16, 0.38)
        } else {
            (0.72, 1.07, 0.30)
        };
        // A thick root, folded upper helix and tapered tip replace round disks.
        let outer = [
            [0.05, 0.29, side * root],
            [-0.06, rise, side * tip],
            [0.01, 0.09, side * (tip - 0.09)],
            [0.06, -0.16, side * (root + 0.10)],
            [0.15, -0.12, side * root],
        ];
        relief(mesh, &outer, 0.15, "face_skin");
        let fold = [
            [0.09, 0.21, side * (root + 0.03)],
            [-0.01, rise - 0.09, side * (tip - 0.09)],
            [0.06, 0.04, side * (root + 0.18)],
            [0.13, -0.07, side * (root + 0.04)],
        ];
        relief(mesh, &fold, 0.025, "brow");
    }
}

// The nose is a continuous set of broad triangular sections, with distinct
// bridge, tip and alar planes. Trolls get the long hooked profile, orcs a blunt
// broad bridge; neither is a muzzle or a cone.
fn heavy_nose(mesh: &mut MeshAsset, troll: bool) {
    let sections: &[(f32, f32, f32, f32)] = if troll {
        &[
            (0.39, 0.52, 0.44, 0.07),
            (0.23, 0.65, 0.46, 0.10),
            (-0.03, 0.76, 0.47, 0.13),
            (-0.24, 0.85, 0.48, 0.19),
            (-0.37, 0.82, 0.48, 0.22),
            (-0.45, 0.64, 0.47, 0.12),
        ]
    } else {
        &[
            (0.37, 0.53, 0.46, 0.08),
            (0.19, 0.61, 0.49, 0.11),
            (-0.06, 0.70, 0.51, 0.15),
            (-0.18, 0.72, 0.51, 0.23),
            (-0.27, 0.61, 0.50, 0.19),
        ]
    };
    let start = mesh.vertices.len();
    for &(y, tip, base, width) in sections {
        mesh.vertices.extend([
            [base, y, -width],
            [tip - 0.035, y, -width * 0.48],
            [tip, y, 0.0],
            [tip - 0.035, y, width * 0.48],
            [base, y, width],
        ]);
    }
    for ring in 0..sections.len() - 1 {
        for side in 0..4 {
            let a = start + ring * 5 + side;
            mesh.faces.extend([
                (a, a + 1, a + 5, "face_skin".into()),
                (a + 1, a + 6, a + 5, "face_skin".into()),
            ]);
        }
    }
    for side in [-1.0, 1.0] {
        oriented_blob(
            mesh,
            [
                if troll { 0.76 } else { 0.66 },
                if troll { -0.36 } else { -0.20 },
                side * if troll { 0.15 } else { 0.17 },
            ],
            [0.012, 0.032, 0.045],
            side * 0.55,
            "nose",
        );
    }
}

fn lower_tusks(mesh: &mut MeshAsset, troll: bool) {
    for side in [-1.0, 1.0] {
        let path: &[(f32, f32, f32, f32)] = if troll {
            &[
                (-0.64, 0.055, 0.57, 0.30),
                (-0.48, 0.066, 0.67, 0.33),
                (-0.30, 0.035, 0.70, 0.35),
                (-0.16, 0.0, 0.66, 0.36),
            ]
        } else {
            &[
                (-0.64, 0.075, 0.58, 0.39),
                (-0.44, 0.090, 0.67, 0.42),
                (-0.22, 0.051, 0.71, 0.44),
                (-0.06, 0.0, 0.65, 0.43),
            ]
        };
        let start = mesh.vertices.len();
        for &(y, radius, x, z) in path {
            for i in 0..8 {
                let theta = std::f32::consts::TAU * i as f32 / 8.0;
                mesh.vertices
                    .push([x + radius * theta.cos(), y, side * z + radius * theta.sin()]);
            }
        }
        for ring in 0..path.len() - 1 {
            for i in 0..8 {
                let a = start + ring * 8 + i;
                let b = start + ring * 8 + (i + 1) % 8;
                mesh.faces.extend([
                    (a, b, a + 8, "face_tusk".into()),
                    (b, b + 8, a + 8, "face_tusk".into()),
                ]);
            }
        }
    }
}

fn heavy_humanoid(troll: bool) -> MeshAsset {
    let sections: &[(f32, f32, f32, f32)] = if troll {
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.88, 0.36, -0.24, 0.33),
            (-0.72, 0.57, -0.42, 0.49),
            (-0.48, 0.54, -0.53, 0.62),
            (-0.14, 0.48, -0.64, 0.69),
            (0.17, 0.46, -0.68, 0.70),
            (0.49, 0.42, -0.68, 0.64),
            (0.77, 0.38, -0.59, 0.58),
            (0.96, 0.25, -0.44, 0.43),
            (1.03, 0.10, -0.27, 0.24),
            (1.05, 0.0, -0.14, 0.0),
        ]
    } else {
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.86, 0.40, -0.28, 0.48),
            (-0.72, 0.62, -0.41, 0.64),
            (-0.47, 0.59, -0.55, 0.77),
            (-0.12, 0.51, -0.63, 0.80),
            (0.19, 0.50, -0.68, 0.78),
            (0.49, 0.43, -0.64, 0.69),
            (0.76, 0.37, -0.57, 0.61),
            (0.91, 0.24, -0.42, 0.45),
            (0.98, 0.10, -0.27, 0.26),
            (1.00, 0.0, -0.14, 0.0),
        ]
    };
    let mut mesh = heavy_skull(sections);
    pointed_ears(&mut mesh, troll);
    heavy_eyes(&mut mesh, troll);
    heavy_nose(&mut mesh, troll);
    let mouth: Vec<_> = (-6..=6)
        .map(|i| {
            let z = i as f32 * if troll { 0.047 } else { 0.061 };
            [
                if troll { 0.572 } else { 0.611 } - 0.30 * z * z,
                if troll { -0.555 } else { -0.435 } - 0.10 * z.abs(),
                z,
            ]
        })
        .collect();
    ribbon(
        &mut mesh,
        &mouth,
        if troll { 0.025 } else { 0.028 },
        "mouth",
    );
    // The lower lip joins a single powerful jaw/chin rather than a separate ball.
    let lip: Vec<_> = mouth
        .iter()
        .map(|p| [p[0] + 0.008, p[1] - 0.065, p[2] * 0.91])
        .collect();
    ribbon(&mut mesh, &lip, 0.035, "face_skin");
    lower_tusks(&mut mesh, troll);
    mesh
}

fn orc() -> MeshAsset {
    heavy_humanoid(false)
}
fn troll() -> MeshAsset {
    heavy_humanoid(true)
}

fn cyclops() -> MeshAsset {
    let mut mesh = heavy_skull(&[
        (-1.0, 0.0, 0.0, 0.0),
        (-0.85, 0.37, -0.31, 0.40),
        (-0.62, 0.57, -0.49, 0.59),
        (-0.28, 0.53, -0.63, 0.68),
        (0.15, 0.49, -0.68, 0.70),
        (0.51, 0.45, -0.66, 0.64),
        (0.81, 0.30, -0.50, 0.46),
        (1.0, 0.0, -0.13, 0.0),
    ]);
    // One central orbit is carved across the brow. Its eye is a shallow almond,
    // contained within upper and lower lids, rather than a sphere on the face.
    blob(&mut mesh, [0.50, 0.22, 0.0], [0.043, 0.20, 0.38], "socket");
    let almond: Vec<_> = (0..16)
        .map(|i| {
            let t = std::f32::consts::TAU * i as f32 / 16.0;
            let z = 0.33 * t.cos();
            [0.552 - 0.19 * z * z, 0.22 + 0.137 * t.sin(), z]
        })
        .collect();
    relief(&mut mesh, &almond, 0.026, "eye_white");
    blob(
        &mut mesh,
        [0.568, 0.22, 0.0],
        [0.016, 0.125, 0.125],
        "eye_dark",
    );
    blob(
        &mut mesh,
        [0.585, 0.265, -0.04],
        [0.004, 0.027, 0.021],
        "eye_white",
    );
    for side in [-1.0, 1.0] {
        relief(
            &mut mesh,
            &[
                [0.60, 0.39, 0.0],
                [0.55, 0.51, side * 0.15],
                [0.42, 0.49, side * 0.42],
                [0.31, 0.31, side * 0.58],
                [0.39, 0.25, side * 0.46],
                [0.53, 0.35, side * 0.22],
            ],
            0.12,
            "brow",
        );
        blob(
            &mut mesh,
            [-0.04, 0.02, side * 0.69],
            [0.10, 0.20, 0.08],
            "face_skin",
        );
    }
    relief(
        &mut mesh,
        &[
            [0.51, 0.02, -0.09],
            [0.62, -0.18, -0.14],
            [0.69, -0.27, 0.0],
            [0.62, -0.18, 0.14],
            [0.51, 0.02, 0.09],
        ],
        0.12,
        "face_skin",
    );
    let mouth: Vec<_> = (-5..=5)
        .map(|i| {
            let z = i as f32 * 0.060;
            [0.566 - z * z * 0.25, -0.51 - z.abs() * 0.08, z]
        })
        .collect();
    ribbon(&mut mesh, &mouth, 0.026, "mouth");
    mesh
}

fn anime_adult() -> MeshAsset {
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.86, 0.29, -0.25, 0.23),
            (-0.64, 0.45, -0.40, 0.40),
            (-0.32, 0.50, -0.53, 0.53),
            (0.12, 0.51, -0.59, 0.61),
            (0.48, 0.47, -0.61, 0.59),
            (0.77, 0.33, -0.53, 0.46),
            (0.98, 0.0, -0.12, 0.0),
        ],
        "skin_soft",
        true,
    );
    // An adult jaw and long midface balance the expressive manga eyes.
    for side in [-1.0, 1.0] {
        let eye: Vec<_> = (0..12)
            .map(|i| {
                let t = std::f32::consts::TAU * i as f32 / 12.0;
                let dz = 0.174 * t.cos();
                [
                    0.455 - dz * 0.62,
                    0.18 + 0.115 * t.sin() + dz * 0.17,
                    side * (0.33 + dz),
                ]
            })
            .collect();
        relief(&mut mesh, &eye, 0.020, "eye_white");
        oriented_blob(
            &mut mesh,
            [0.475, 0.18, side * 0.33],
            [0.016, 0.100, 0.070],
            side * 0.55,
            "eye_dark",
        );
        oriented_blob(
            &mut mesh,
            [0.492, 0.224, side * 0.315],
            [0.004, 0.031, 0.021],
            side * 0.55,
            "eye_white",
        );
        let lash: Vec<_> = (0..=7)
            .map(|i| {
                let t = std::f32::consts::PI * i as f32 / 7.0;
                let dz = 0.18 * t.cos();
                [
                    0.465 - dz * 0.62,
                    0.18 + 0.119 * t.sin() + dz * 0.17,
                    side * (0.33 + dz),
                ]
            })
            .collect();
        ribbon(&mut mesh, &lash, 0.023, "hair");
        let eyebrow = [
            [0.50, 0.38, side * 0.17],
            [0.44, 0.42, side * 0.34],
            [0.33, 0.39, side * 0.49],
        ];
        ribbon(&mut mesh, &eyebrow, 0.018, "hair");
        blob(
            &mut mesh,
            [-0.01, -0.02, side * 0.59],
            [0.08, 0.17, 0.05],
            "skin_soft",
        );
        // Shoulder-length layers, with swept tips and visible separate clumps.
        for lock in 0..3 {
            let l = lock as f32;
            hair_lock(
                &mut mesh,
                &[
                    (
                        V3::new(-0.10 - l * 0.12, 0.77 - l * 0.06, side * (0.44 - l * 0.05)),
                        0.17,
                    ),
                    (
                        V3::new(-0.12 - l * 0.10, 0.20, side * (0.64 + l * 0.04)),
                        0.19,
                    ),
                    (
                        V3::new(-0.16 - l * 0.10, -0.60, side * (0.63 + l * 0.07)),
                        0.14,
                    ),
                    (
                        V3::new(-0.28 - l * 0.12, -1.20 - l * 0.06, side * (0.53 + l * 0.11)),
                        0.0,
                    ),
                ],
            );
        }
    }
    // Asymmetric broad fringe exposes the eyes and prevents a toy-like cap.
    for i in 0..3 {
        let z = i as f32 * 0.18;
        hair_lock(
            &mut mesh,
            &[
                (V3::new(0.02, 0.89, -0.36 + z), 0.15),
                (V3::new(0.32, 0.78, -0.28 + z), 0.16),
                (V3::new(0.49, 0.60, -0.08 + z), 0.12),
                (V3::new(0.38, 0.33, 0.30 + z * 0.60), 0.0),
            ],
        );
    }
    relief(
        &mut mesh,
        &[
            [0.51, 0.02, -0.055],
            [0.58, -0.18, -0.075],
            [0.61, -0.24, 0.0],
            [0.56, -0.22, 0.07],
            [0.51, 0.02, 0.055],
        ],
        0.06,
        "skin_soft",
    );
    ribbon(
        &mut mesh,
        &[
            [0.489, -0.48, -0.14],
            [0.51, -0.50, 0.0],
            [0.489, -0.48, 0.14],
        ],
        0.023,
        "mouth",
    );
    mesh
}

fn historical() -> MeshAsset {
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.83, 0.30, -0.29, 0.32),
            (-0.60, 0.50, -0.45, 0.49),
            (-0.26, 0.54, -0.58, 0.62),
            (0.15, 0.53, -0.64, 0.66),
            (0.49, 0.48, -0.64, 0.61),
            (0.77, 0.36, -0.53, 0.48),
            (0.98, 0.0, -0.12, 0.0),
        ],
        "face_skin",
        true,
    );
    eyes(&mut mesh, 0.43, 0.17, 0.67);
    for side in [-1.0, 1.0] {
        blob(
            &mut mesh,
            [-0.06, -0.03, side * 0.65],
            [0.09, 0.20, 0.06],
            "face_skin",
        );
        ribbon(
            &mut mesh,
            &[
                [0.49, 0.34, side * 0.23],
                [0.45, 0.36, side * 0.40],
                [0.34, 0.31, side * 0.54],
            ],
            0.026,
            "hair",
        );
        // Small nasolabial folds suggest an older, leaner face.
        ribbon(
            &mut mesh,
            &[[0.54, -0.21, side * 0.15], [0.52, -0.40, side * 0.23]],
            0.015,
            "jaw",
        );
    }
    relief(
        &mut mesh,
        &[
            [0.53, 0.21, -0.075],
            [0.64, -0.07, -0.10],
            [0.68, -0.20, 0.0],
            [0.62, -0.23, 0.12],
            [0.53, 0.21, 0.075],
        ],
        0.12,
        "face_skin",
    );
    relief(
        &mut mesh,
        &[
            [0.558, -0.275, -0.12],
            [0.565, -0.275, 0.12],
            [0.566, -0.385, 0.11],
            [0.561, -0.385, -0.11],
        ],
        0.020,
        "moustache",
    );
    ribbon(
        &mut mesh,
        &[
            [0.52, -0.46, -0.21],
            [0.555, -0.475, 0.0],
            [0.52, -0.46, 0.21],
        ],
        0.021,
        "mouth",
    );
    // A compact side-parted sweep, not a helmet, is the defining hair silhouette.
    for i in 0..4 {
        let z = i as f32 * 0.11;
        hair_lock(
            &mut mesh,
            &[
                (V3::new(0.02, 0.87, 0.36 - z), 0.10),
                (V3::new(0.29, 0.79, 0.15 - z), 0.13),
                (V3::new(0.43, 0.66, -0.17 - z), 0.11),
                (V3::new(0.29, 0.43, -0.53), 0.0),
            ],
        );
    }
    mesh
}

fn helmet() -> MeshAsset {
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.85, 0.47, -0.38, 0.53),
            (-0.55, 0.62, -0.55, 0.73),
            (-0.15, 0.59, -0.65, 0.77),
            (0.3, 0.57, -0.70, 0.78),
            (0.65, 0.46, -0.64, 0.68),
            (0.9, 0.26, -0.45, 0.43),
            (1.0, 0.0, 0.0, 0.0),
        ],
        "helmet_white",
        false,
    );
    // Black brow and wraparound eye lenses, legible from front and profile.
    let brow: Vec<_> = (-6..=6)
        .map(|i| {
            let z = i as f32 * 0.1;
            [0.57 * (1.0 - (z / 0.78).powi(2)).sqrt() + 0.025, 0.31, z]
        })
        .collect();
    ribbon(&mut mesh, &brow, 0.035, "helmet_black");
    for side in [-1.0, 1.0] {
        let first = mesh.vertices.len();
        for i in 0..=6 {
            let t = i as f32 / 6.0;
            let z = 0.14 + 0.47 * t;
            let x = 0.60 * (1.0 - (z / 0.78).powi(2)).sqrt() + 0.025;
            mesh.vertices.extend([
                [x, 0.23 + 0.02 * t, side * z],
                [x, 0.03 + 0.05 * t, side * z],
            ]);
        }
        for i in 0..6 {
            let a = first + i * 2;
            mesh.faces.extend([
                (a, a + 1, a + 2, "helmet_black".into()),
                (a + 1, a + 3, a + 2, "helmet_black".into()),
            ]);
        }
        blob(
            &mut mesh,
            [0.27, -0.55, side * 0.63],
            [0.36, 0.24, 0.22],
            "helmet_white",
        );
        for i in 0..3 {
            blob(
                &mut mesh,
                [0.22 + i as f32 * 0.115, -0.54, side * 0.82],
                [0.028, 0.09, 0.025],
                "helmet_black",
            );
        }
        blob(
            &mut mesh,
            [0.56, -0.61, side * 0.35],
            [0.13, 0.11, 0.11],
            "helmet_black",
        );
        blob(
            &mut mesh,
            [0.63, -0.60, side * 0.35],
            [0.045, 0.06, 0.065],
            "helmet_white",
        );
    }
    blob(
        &mut mesh,
        [0.57, -0.04, 0.0],
        [0.08, 0.27, 0.14],
        "helmet_white",
    );
    let frown: Vec<_> = (-4..=4)
        .map(|i| {
            let z = i as f32 * 0.065;
            [
                0.62 * (1.0 - (z / 0.75).powi(2)).sqrt() + 0.025,
                -0.32 - z.abs() * 0.65,
                z,
            ]
        })
        .collect();
    ribbon(&mut mesh, &frown, 0.035, "helmet_black");
    mesh
}

fn robot() -> MeshAsset {
    let mut mesh = skull(
        &[
            (-1.0, 0.0, 0.0, 0.0),
            (-0.8, 0.48, -0.42, 0.53),
            (-0.4, 0.55, -0.52, 0.65),
            (0.3, 0.55, -0.52, 0.65),
            (0.8, 0.46, -0.46, 0.57),
            (1.0, 0.0, 0.0, 0.0),
        ],
        "robot_metal",
        false,
    );
    for side in [-1.0, 1.0] {
        oriented_blob(
            &mut mesh,
            [0.43, 0.2, side * 0.45],
            [0.07, 0.14, 0.20],
            side * 0.7,
            "helmet_black",
        );
        oriented_blob(
            &mut mesh,
            [0.48, 0.2, side * 0.49],
            [0.025, 0.07, 0.13],
            side * 0.7,
            "robot_eye",
        );
        blob(
            &mut mesh,
            [-0.1, 0.0, side * 0.66],
            [0.13, 0.26, 0.10],
            "robot_metal",
        );
    }
    for y in [-0.30, -0.42, -0.54] {
        let points: Vec<_> = (-3..=3)
            .map(|i| {
                let z = i as f32 * 0.07;
                [0.55 * (1.0 - (z / 0.65).powi(2)).sqrt() + 0.02, y, z]
            })
            .collect();
        ribbon(&mut mesh, &points, 0.025, "helmet_black");
    }
    mesh
}

fn alien() -> MeshAsset {
    let mut mesh = human(false, false);
    for v in &mut mesh.vertices {
        if v[1] > 0.3 {
            v[1] = 0.3 + (v[1] - 0.3) * 1.24;
            v[2] *= 1.12;
        }
        if v[1] < 0.0 {
            v[2] *= 0.8;
        }
    }
    for (_, _, _, role) in &mut mesh.faces {
        if role == "hair" {
            *role = "face_skin".into();
        }
        if role == "eye_white" {
            *role = "eye_dark".into();
        }
    }
    mesh
}

fn ape() -> MeshAsset {
    let mut mesh = ape_base();
    mesh.faces.retain(|(_, _, _, role)| role != "face_tusk");
    mesh
}

fn fish() -> MeshAsset {
    let mut mesh = MeshAsset {
        vertices: Vec::new(),
        faces: Vec::new(),
    };
    blob(
        &mut mesh,
        [-0.15, 0.0, 0.0],
        [0.85, 0.75, 0.65],
        "face_skin",
    );
    for side in [-1.0, 1.0] {
        blob(
            &mut mesh,
            [0.17, 0.23, side * 0.58],
            [0.18, 0.20, 0.08],
            "eye_white",
        );
        blob(
            &mut mesh,
            [0.20, 0.23, side * 0.65],
            [0.10, 0.13, 0.035],
            "eye_dark",
        );
        let gill: Vec<_> = (0..=8)
            .map(|i| {
                let t = i as f32 / 8.0;
                [
                    -0.43 + 0.10 * (std::f32::consts::PI * t).sin(),
                    0.48 - t * 0.95,
                    side * (0.51 + 0.10 * (std::f32::consts::PI * t).sin()),
                ]
            })
            .collect();
        ribbon(&mut mesh, &gill, 0.035, "mouth");
    }
    blob(&mut mesh, [0.69, -0.14, 0.0], [0.035, 0.21, 0.30], "mouth");
    let first = mesh.vertices.len();
    for i in 0..16 {
        let t = std::f32::consts::TAU * i as f32 / 16.0;
        mesh.vertices.extend([
            [0.70, -0.14 + 0.25 * t.cos(), 0.35 * t.sin()],
            [0.735, -0.14 + 0.18 * t.cos(), 0.26 * t.sin()],
        ]);
    }
    for i in 0..16 {
        let a = first + i * 2;
        let b = first + ((i + 1) % 16) * 2;
        mesh.faces.extend([
            (a, b, a + 1, "face_skin".into()),
            (a + 1, b, b + 1, "face_skin".into()),
        ]);
    }
    mesh
}

pub(super) fn mesh(kind: &str) -> &'static MeshAsset {
    static FACES: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    let faces = FACES.get_or_init(|| {
        vec![
            human(false, false),
            human(true, false),
            orc(),
            helmet(),
            cyclops(),
            robot(),
            alien(),
            ape(),
            fish(),
            troll(),
            anime_adult(),
            historical(),
        ]
    });
    &faces[match kind {
        "mermaid" => 1,
        "orc" => 2,
        "helmet" => 3,
        "cyclops" => 4,
        "robot" => 5,
        "alien" => 6,
        "ape" => 7,
        "fish" => 8,
        "troll" => 9,
        "anime_adult" => 10,
        "historical" => 11,
        _ => 0,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redesigned_faces_are_finite_and_use_valid_indices() {
        for name in ["orc", "troll", "cyclops", "anime_adult", "historical"] {
            let mesh = mesh(name);
            assert!(
                mesh.vertices.iter().flatten().all(|x| x.is_finite()),
                "{name}"
            );
            assert!(
                mesh.faces.iter().all(|(a, b, c, _)| {
                    *a < mesh.vertices.len() && *b < mesh.vertices.len() && *c < mesh.vertices.len()
                }),
                "{name}"
            );
            assert!(
                mesh.vertices.iter().all(|v| v[0] < 0.9),
                "{name}: no projecting muzzle"
            );
            assert!(
                mesh.faces.iter().any(|(_, _, _, role)| role == "eye_dark"),
                "{name}"
            );
        }
    }

    #[test]
    fn heavy_faces_have_distinct_silhouettes_and_shared_readable_features() {
        let orc = mesh("orc");
        let troll = mesh("troll");
        assert_ne!(orc.vertices, troll.vertices);
        let max_front = |m: &MeshAsset| m.vertices.iter().map(|v| v[0]).fold(f32::MIN, f32::max);
        assert!(
            max_front(troll) > max_front(orc) + 0.04,
            "troll's hooked nose must read in profile"
        );
        for m in [orc, troll] {
            for expected in ["brow", "socket", "nose", "mouth", "face_tusk"] {
                assert!(
                    m.faces.iter().any(|(_, _, _, role)| role == expected),
                    "{expected}"
                );
            }
            assert!(
                m.vertices.iter().any(|v| v[2].abs() > 1.0),
                "pointed ears survive silhouette reduction"
            );
        }
    }

    #[test]
    fn character_faces_do_not_alias_generic_humans() {
        for name in ["cyclops", "anime_adult", "historical"] {
            assert_ne!(mesh(name).vertices, mesh("human").vertices, "{name}");
        }
        assert!(mesh("anime_adult").vertices.iter().any(|v| v[1] < -1.1));
        assert!(mesh("historical")
            .faces
            .iter()
            .any(|(_, _, _, role)| role == "moustache"));
        let eye_vertices: Vec<_> = mesh("cyclops")
            .faces
            .iter()
            .filter(|(_, _, _, role)| role == "eye_dark")
            .flat_map(|(a, b, c, _)| [*a, *b, *c])
            .map(|i| mesh("cyclops").vertices[i])
            .collect();
        assert!(
            eye_vertices.iter().all(|v| v[2].abs() < 0.14),
            "one central pupil"
        );
    }
}
