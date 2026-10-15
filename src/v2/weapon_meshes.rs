//! V2-only authored equipment. Local +X is the firing/blade direction and +Y is up.
//! Gun muzzles remain at x=1.3; grip anchors match v2::grip exactly.
use super::{MeshAsset, OnceLock};

fn empty() -> MeshAsset {
    MeshAsset {
        vertices: vec![],
        faces: vec![],
    }
}

fn prism(m: &mut MeshAsset, outline: &[[f32; 2]], half_depth: f32, role: &str) {
    let start = m.vertices.len();
    let n = outline.len();
    for z in [-half_depth, half_depth] {
        m.vertices.extend(outline.iter().map(|p| [p[0], p[1], z]));
    }
    for i in 1..n - 1 {
        m.faces.extend([
            (start, start + i, start + i + 1, role.into()),
            (start + n, start + n + i + 1, start + n + i, role.into()),
        ]);
    }
    for i in 0..n {
        let j = (i + 1) % n;
        m.faces.extend([
            (start + i, start + j, start + n + i, role.into()),
            (start + j, start + n + j, start + n + i, role.into()),
        ]);
    }
}

fn block(m: &mut MeshAsset, x: [f32; 2], y: [f32; 2], z: f32, role: &str) {
    prism(
        m,
        &[[x[0], y[0]], [x[1], y[0]], [x[1], y[1]], [x[0], y[1]]],
        z,
        role,
    );
}

fn tube(m: &mut MeshAsset, sections: &[(f32, f32, f32)], y: f32, role: &str, cap: &str) {
    let start = m.vertices.len();
    const SIDES: usize = 12;
    for &(x, ry, rz) in sections {
        for i in 0..SIDES {
            let t = std::f32::consts::TAU * i as f32 / SIDES as f32;
            m.vertices.push([x, y + ry * t.cos(), rz * t.sin()]);
        }
    }
    for ring in 0..sections.len() - 1 {
        for i in 0..SIDES {
            let a = start + ring * SIDES + i;
            let b = start + ring * SIDES + (i + 1) % SIDES;
            m.faces.extend([
                (a, b, a + SIDES, role.into()),
                (b, b + SIDES, a + SIDES, role.into()),
            ]);
        }
    }
    for ring in [0, sections.len() - 1] {
        let c = m.vertices.len();
        m.vertices.push([sections[ring].0, y, 0.0]);
        for i in 0..SIDES {
            m.faces.push((
                c,
                start + ring * SIDES + i,
                start + ring * SIDES + (i + 1) % SIDES,
                cap.into(),
            ));
        }
    }
}

fn gun(kind: &str) -> MeshAsset {
    let mut m = empty();
    let energy = matches!(kind, "LASER" | "RAILGUN");
    let heavy = matches!(kind, "CANNON" | "ROCKET" | "MISSILE");
    prism(
        &mut m,
        &[
            [-1.25, 0.22],
            [-0.74, 0.17],
            [-0.61, -0.12],
            [-0.78, -0.30],
            [-1.25, -0.37],
        ],
        0.19,
        "helmet_black",
    );
    prism(
        &mut m,
        &[
            [-0.76, 0.22],
            [-0.57, 0.34],
            [0.27, 0.30],
            [0.42, 0.15],
            [0.35, -0.20],
            [-0.70, -0.21],
        ],
        0.25,
        "metal",
    );
    // Slanted pistol grip; the existing primary hand anchor is inside this grip.
    prism(
        &mut m,
        &[
            [-0.65, -0.13],
            [-0.44, -0.15],
            [-0.46, -0.66],
            [-0.65, -0.66],
        ],
        0.13,
        "helmet_black",
    );
    block(&mut m, [-0.15, 0.60], [-0.27, -0.12], 0.20, "helmet_black");
    let radius = if heavy {
        0.32
    } else if energy {
        0.24
    } else {
        0.15
    };
    tube(
        &mut m,
        &[
            (0.28, radius, radius),
            (1.22, radius * 0.86, radius * 0.86),
            (1.30, radius * 0.97, radius * 0.97),
        ],
        0.08,
        "metal",
        "helmet_black",
    );
    if energy {
        for x in [0.28, 0.49, 0.70, 0.91] {
            tube(
                &mut m,
                &[(x, 0.30, 0.28), (x + 0.07, 0.30, 0.28)],
                0.08,
                "helmet_black",
                "metal",
            );
        }
        // Exposed blue capacitor strip and a bright emitter recessed in the muzzle.
        block(&mut m, [-0.39, 0.20], [0.31, 0.38], 0.19, "robot_eye");
        tube(
            &mut m,
            &[(1.295, 0.13, 0.13), (1.300, 0.13, 0.13)],
            0.08,
            "robot_eye",
            "robot_eye",
        );
        block(&mut m, [-0.41, -0.15], [-0.34, -0.23], 0.18, "robot_eye");
    } else if kind == "SHOTGUN" {
        tube(
            &mut m,
            &[(0.26, 0.10, 0.10), (1.18, 0.10, 0.10)],
            -0.16,
            "helmet_black",
            "helmet_black",
        );
        for x in [0.15, 0.25, 0.35, 0.45, 0.55] {
            block(&mut m, [x, x + 0.035], [-0.26, -0.12], 0.23, "metal");
        }
    } else if heavy {
        tube(
            &mut m,
            &[(1.08, 0.38, 0.36), (1.30, 0.38, 0.36)],
            0.08,
            "helmet_black",
            "helmet_black",
        );
        tube(
            &mut m,
            &[(1.298, 0.21, 0.20), (1.300, 0.21, 0.20)],
            0.08,
            "metal",
            "mouth",
        );
        block(&mut m, [-0.45, -0.05], [0.33, 0.42], 0.17, "helmet_white");
    }
    block(&mut m, [-0.40, -0.29], [0.33, 0.48], 0.07, "helmet_black");
    block(&mut m, [0.99, 1.07], [0.28, 0.40], 0.06, "helmet_black");
    if energy || heavy {
        // A low-resolution cannon needs a substantial receiver and cooling
        // silhouette. Both unchanged hand anchors remain inside their grips.
        for vertex in &mut m.vertices {
            vertex[1] *= 1.55;
            vertex[2] *= 1.35;
        }
    }
    m
}

fn wrapped_handle(m: &mut MeshAsset, from: f32, to: f32, radius: f32) {
    tube(
        m,
        &[(from, radius, radius), (to, radius, radius)],
        0.0,
        "wood",
        "metal",
    );
    for i in 0..6 {
        let x = from + (to - from) * i as f32 / 6.0;
        tube(
            m,
            &[
                (x, radius * 1.06, radius * 1.06),
                (x + 0.035, radius * 1.06, radius * 1.06),
            ],
            0.0,
            "helmet_black",
            "helmet_black",
        );
    }
}

fn sword() -> MeshAsset {
    let mut m = empty();
    wrapped_handle(&mut m, -1.05, -0.29, 0.10);
    tube(
        &mut m,
        &[(-1.19, 0.14, 0.14), (-1.05, 0.15, 0.15)],
        0.0,
        "metal",
        "metal",
    );
    prism(
        &mut m,
        &[
            [-0.30, -0.44],
            [-0.19, -0.46],
            [-0.14, -0.12],
            [-0.14, 0.12],
            [-0.19, 0.46],
            [-0.30, 0.44],
        ],
        0.13,
        "metal",
    );
    prism(
        &mut m,
        &[
            [-0.17, -0.16],
            [0.99, -0.13],
            [1.45, 0.0],
            [0.99, 0.13],
            [-0.17, 0.16],
        ],
        0.035,
        "helmet_white",
    );
    prism(
        &mut m,
        &[
            [-0.14, -0.07],
            [0.99, -0.055],
            [1.31, 0.0],
            [0.99, 0.055],
            [-0.14, 0.07],
        ],
        0.065,
        "metal",
    );
    m
}

fn axe() -> MeshAsset {
    let mut m = empty();
    tube(
        &mut m,
        &[(-1.20, 0.075, 0.075), (0.95, 0.075, 0.075)],
        0.0,
        "wood",
        "wood",
    );
    wrapped_handle(&mut m, -1.12, -0.37, 0.085);
    tube(
        &mut m,
        &[(0.62, 0.19, 0.15), (0.88, 0.19, 0.15)],
        0.0,
        "helmet_black",
        "metal",
    );
    prism(
        &mut m,
        &[
            [0.62, -0.17],
            [0.32, -0.54],
            [0.31, -0.89],
            [0.74, -1.03],
            [1.05, -0.87],
            [0.92, -0.34],
            [0.87, 0.17],
        ],
        0.10,
        "metal",
    );
    prism(
        &mut m,
        &[
            [0.31, -0.89],
            [0.74, -1.03],
            [1.05, -0.87],
            [1.02, -0.70],
            [0.72, -0.84],
            [0.33, -0.72],
        ],
        0.12,
        "helmet_white",
    );
    prism(
        &mut m,
        &[[0.63, 0.13], [0.77, 0.63], [0.94, 0.16]],
        0.085,
        "metal",
    );
    m
}

fn spear() -> MeshAsset {
    let mut m = empty();
    tube(
        &mut m,
        &[(-1.40, 0.055, 0.055), (0.87, 0.055, 0.055)],
        0.0,
        "wood",
        "metal",
    );
    wrapped_handle(&mut m, -0.77, -0.24, 0.068);
    tube(
        &mut m,
        &[(0.69, 0.085, 0.085), (0.89, 0.11, 0.09)],
        0.0,
        "metal",
        "metal",
    );
    prism(
        &mut m,
        &[[0.84, 0.0], [1.03, -0.18], [1.48, 0.0], [1.03, 0.18]],
        0.025,
        "helmet_white",
    );
    prism(
        &mut m,
        &[[0.84, 0.0], [1.03, -0.08], [1.40, 0.0], [1.03, 0.08]],
        0.058,
        "metal",
    );
    tube(
        &mut m,
        &[(-1.48, 0.0, 0.0), (-1.30, 0.072, 0.072)],
        0.0,
        "metal",
        "metal",
    );
    m
}

fn chainsaw() -> MeshAsset {
    let mut m = empty();
    prism(
        &mut m,
        &[
            [-0.91, -0.32],
            [-0.99, 0.11],
            [-0.77, 0.39],
            [-0.18, 0.39],
            [0.03, 0.18],
            [-0.02, -0.30],
        ],
        0.28,
        "red",
    );
    prism(
        &mut m,
        &[
            [-0.09, -0.18],
            [1.12, -0.18],
            [1.38, -0.09],
            [1.43, 0.0],
            [1.38, 0.09],
            [1.12, 0.18],
            [-0.09, 0.18],
        ],
        0.085,
        "metal",
    );
    block(&mut m, [0.04, 1.22], [-0.065, 0.065], 0.10, "helmet_black");
    for i in 0..11 {
        let x = 0.02 + i as f32 * 0.12;
        for side in [-1.0, 1.0] {
            prism(
                &mut m,
                &[
                    [x, side * 0.17],
                    [x + 0.07, side * 0.26],
                    [x + 0.13, side * 0.17],
                ],
                0.10,
                "helmet_white",
            );
        }
    }
    block(&mut m, [-1.28, -1.12], [-0.36, 0.27], 0.13, "helmet_black");
    block(&mut m, [-1.18, -0.82], [0.19, 0.30], 0.13, "helmet_black");
    block(&mut m, [-1.18, -0.82], [-0.39, -0.26], 0.13, "helmet_black");
    block(&mut m, [-0.64, -0.48], [0.37, 0.62], 0.13, "helmet_black");
    block(&mut m, [-0.59, -0.13], [0.52, 0.64], 0.13, "helmet_black");
    for y in [-0.13, 0.0, 0.13] {
        block(&mut m, [-0.80, -0.47], [y, y + 0.04], 0.29, "helmet_black");
    }
    for vertex in &mut m.vertices {
        vertex[1] *= 1.65;
    }
    m
}

fn scythe() -> MeshAsset {
    let mut m = empty();
    tube(
        &mut m,
        &[(-1.45, 0.13, 0.13), (2.62, 0.115, 0.115)],
        0.0,
        "wood",
        "metal",
    );
    wrapped_handle(&mut m, -0.72, -0.31, 0.14);
    tube(
        &mut m,
        &[(2.32, 0.17, 0.17), (2.62, 0.17, 0.17)],
        0.0,
        "metal",
        "metal",
    );
    let blade_start = m.vertices.len();
    prism(
        &mut m,
        &[
            [0.87, -0.04],
            [1.10, 0.12],
            [1.04, 0.55],
            [0.76, 0.94],
            [0.30, 1.25],
            [-0.25, 1.43],
            [-0.79, 1.47],
            [-0.21, 1.22],
            [0.29, 0.94],
            [0.64, 0.59],
            [0.83, 0.21],
        ],
        0.055,
        "metal",
    );
    prism(
        &mut m,
        &[
            [1.04, 0.55],
            [0.76, 0.94],
            [0.30, 1.25],
            [-0.25, 1.43],
            [-0.79, 1.47],
            [-0.20, 1.30],
            [0.33, 1.08],
            [0.77, 0.75],
        ],
        0.065,
        "helmet_white",
    );
    // Weapon nodes have a much smaller Y radius than their shaft X radius.
    // Compensate so the crescent remains a clearly hooked blade at 96 pixels.
    for vertex in &mut m.vertices[blade_start..] {
        vertex[0] += 1.50;
        vertex[1] *= 2.5;
    }
    m
}

pub(super) fn mesh(kind: &str) -> Option<&'static MeshAsset> {
    static MESHES: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    let index = match kind {
        "LASER" => 0,
        "SHOTGUN" => 1,
        "RAILGUN" => 2,
        "CANNON" => 3,
        "ROCKET" | "MISSILE" => 4,
        "SWORD" => 5,
        "AXE" => 6,
        "SPEAR" => 7,
        "CHAINSAW" => 8,
        "SCYTHE" => 9,
        _ => return None,
    };
    Some(
        &MESHES.get_or_init(|| {
            vec![
                gun("LASER"),
                gun("SHOTGUN"),
                gun("RAILGUN"),
                gun("CANNON"),
                gun("ROCKET"),
                sword(),
                axe(),
                spear(),
                chainsaw(),
                scythe(),
            ]
        })[index],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equipment_has_valid_finite_geometry_and_stable_muzzles() {
        for name in [
            "LASER", "SHOTGUN", "RAILGUN", "CANNON", "ROCKET", "SWORD", "AXE", "SPEAR", "CHAINSAW",
            "SCYTHE",
        ] {
            let m = mesh(name).unwrap();
            assert!(m.vertices.iter().flatten().all(|x| x.is_finite()), "{name}");
            assert!(
                m.faces.iter().all(|(a, b, c, _)| *a < m.vertices.len()
                    && *b < m.vertices.len()
                    && *c < m.vertices.len()),
                "{name}"
            );
            if matches!(name, "LASER" | "SHOTGUN" | "RAILGUN" | "CANNON" | "ROCKET") {
                assert!(
                    (m.vertices.iter().map(|v| v[0]).fold(f32::MIN, f32::max) - 1.3).abs() < 0.001,
                    "{name}"
                );
            }
        }
    }
}
