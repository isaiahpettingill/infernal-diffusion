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

fn orc() -> MeshAsset {
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
    let mut mesh = orc();
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
            human(false, true),
            robot(),
            alien(),
            ape(),
            fish(),
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
        _ => 0,
    }]
}
