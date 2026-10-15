//! V2 fantasy accessory and deliberately mismatched machine geometry.
use super::{v2_append_blob, MeshAsset, V3};
use std::sync::OnceLock;
fn empty() -> MeshAsset {
    MeshAsset {
        vertices: vec![],
        faces: vec![],
    }
}
fn blob(m: &mut MeshAsset, p: [f32; 3], s: [f32; 3], r: &str) {
    v2_append_blob(m, V3::new(p[0], p[1], p[2]), V3::new(s[0], s[1], s[2]), r);
}
fn cube(m: &mut MeshAsset, p: [f32; 3], s: [f32; 3], role: &str) {
    let a = m.vertices.len();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [-1.0, 1.0] {
                m.vertices
                    .push([p[0] + x * s[0], p[1] + y * s[1], p[2] + z * s[2]]);
            }
        }
    }
    for (i, j, k) in [
        (0, 2, 3),
        (0, 3, 1),
        (4, 5, 7),
        (4, 7, 6),
        (0, 1, 5),
        (0, 5, 4),
        (2, 6, 7),
        (2, 7, 3),
        (0, 4, 6),
        (0, 6, 2),
        (1, 3, 7),
        (1, 7, 5),
    ] {
        m.faces.push((a + i, a + j, a + k, role.into()));
    }
}
fn face(m: &mut MeshAsset) {
    for z in [-0.38, 0.38] {
        blob(m, [0.89, 0.22, z], [0.16, 0.22, 0.2], "eye_white");
        blob(m, [1.02, 0.20, z], [0.06, 0.13, 0.10], "eye_dark");
    }
    blob(m, [1.02, -0.12, 0.0], [0.25, 0.19, 0.25], "jaw");
    blob(m, [0.95, -0.43, 0.0], [0.12, 0.10, 0.4], "mouth");
}
fn build() -> Vec<MeshAsset> {
    let mut robot = empty();
    cube(
        &mut robot,
        [0.0, 0.15, 0.0],
        [0.88, 0.70, 0.64],
        "robot_metal",
    );
    cube(
        &mut robot,
        [0.7, 0.15, 0.0],
        [0.25, 0.42, 0.5],
        "helmet_black",
    );
    for z in [-0.3, 0.3] {
        cube(&mut robot, [0.98, 0.18, z], [0.025, 0.14, 0.1], "robot_eye");
    }
    cube(&mut robot, [-0.08, -0.8, 0.0], [0.5, 0.2, 0.45], "metal");
    let mut limb = empty();
    cube(
        &mut limb,
        [0.0, 0.0, 0.0],
        [0.72, 0.82, 0.67],
        "robot_metal",
    );
    blob(&mut limb, [0.0, 0.8, 0.0], [0.8, 0.2, 0.8], "metal");
    cube(&mut limb, [0.7, 0.0, 0.0], [0.16, 0.5, 0.3], "helmet_black");
    let mut potato = empty();
    blob(&mut potato, [0.0, 0.0, 0.0], [0.92, 1.0, 0.86], "base");
    face(&mut potato);
    for (y, z) in [(0.65, 0.3), (-0.6, 0.4), (0.1, -0.72)] {
        blob(&mut potato, [0.65, y, z], [0.045, 0.06, 0.06], "mouth");
    }
    let mut round = empty();
    blob(&mut round, [0.0, 0.0, 0.0], [1.0, 1.0, 1.0], "base");
    face(&mut round);
    let mut hood = empty();
    blob(&mut hood, [-0.18, 0.0, 0.0], [0.96, 1.12, 1.05], "hair");
    blob(
        &mut hood,
        [0.78, -0.1, 0.0],
        [0.20, 0.8, 0.67],
        "helmet_black",
    );
    for z in [-0.3, 0.3] {
        blob(&mut hood, [0.96, 0.15, z], [0.05, 0.075, 0.12], "eye_white");
    }
    let mut mask = empty();
    cube(&mut mask, [0.0, 0.0, 0.0], [0.18, 0.7, 1.0], "helmet_black");
    let mut flame = empty();
    blob(&mut flame, [0.0, -0.2, 0.0], [0.5, 0.65, 0.5], "red");
    blob(&mut flame, [0.0, 0.25, 0.0], [0.28, 0.75, 0.28], "tooth");
    let mut hand = empty();
    blob(&mut hand, [0.0, 0.0, 0.0], [0.75, 0.65, 0.55], "base");
    for i in 0..4 {
        blob(
            &mut hand,
            [0.15, -0.6, (i as f32 - 1.5) * 0.3],
            [0.32, 0.52, 0.14],
            "base",
        );
    }
    blob(&mut hand, [0.6, 0.0, 0.5], [0.35, 0.45, 0.18], "base");
    let mut echo = empty();
    blob(&mut echo, [0.0, 0.0, 0.0], [0.75, 0.65, 0.55], "base");
    for i in 0..6 {
        blob(
            &mut echo,
            [0.15, -0.6, (i as f32 - 2.5) * 0.25],
            [0.32, 0.52, 0.13],
            "base",
        );
    }
    blob(&mut echo, [0.6, 0.0, 0.5], [0.35, 0.45, 0.18], "base");
    let mut shades = empty();
    for side in [-1.0, 1.0] {
        cube(
            &mut shades,
            [0.0, 0.0, side * 0.52],
            [0.12, 0.3, 0.43],
            "helmet_black",
        );
    }
    cube(&mut shades, [0.0, 0.04, 0.0], [0.13, 0.06, 0.18], "metal");
    let mut bling = empty();
    for i in 0..24 {
        let a = i as f32 * std::f32::consts::TAU / 24.0;
        blob(
            &mut bling,
            [a.sin() * 0.55, a.cos() * 0.45, 0.0],
            [0.095, 0.095, 0.1],
            "gold",
        );
    }
    let dollar = [
        [-0.19, -0.56],
        [0.0, -0.49],
        [0.21, -0.56],
        [0.22, -0.69],
        [0.0, -0.8],
        [-0.21, -0.91],
        [-0.20, -1.04],
        [0.0, -1.1],
        [0.20, -1.04],
    ];
    for pair in dollar.windows(2) {
        for i in 0..5 {
            let t = i as f32 / 4.0;
            blob(
                &mut bling,
                [
                    pair[0][0] * (1.0 - t) + pair[1][0] * t,
                    pair[0][1] * (1.0 - t) + pair[1][1] * t,
                    0.08,
                ],
                [0.065, 0.065, 0.09],
                "gold",
            );
        }
    }
    cube(&mut bling, [0.0, -0.8, 0.08], [0.045, 0.4, 0.1], "gold");
    // Chain and dollar pendant face forwards (+X), rather than edge-on.
    for v in &mut bling.vertices {
        v.swap(0, 2);
    }
    vec![
        robot, limb, potato, round, hood, mask, flame, hand, echo, shades, bling,
    ]
}
pub(super) fn mesh(affinity: &str, kind: &str) -> Option<&'static MeshAsset> {
    let i = match (affinity, kind) {
        ("ROBOT", "TORSO") => 0,
        ("ROBOT", "ARM" | "LIMB" | "SHIN" | "HAND" | "FOOT" | "SHOULDER" | "HIP") => 1,
        ("POTATO", "HEAD") => 2,
        ("ROUNDLING", "HEAD") => 3,
        ("REAPER", "HEAD") => 4,
        (_, "MASK") => 5,
        (_, "FLAME") => 6,
        ("ORC" | "TROLL" | "GRENDEL" | "CYCLOPEAN" | "MINOTAUR", "HAND") => 7,
        (_, "AI_HAND_ECHO") => 8,
        (_, "SUNGLASSES") => 9,
        (_, "BLING") => 10,
        _ => return None,
    };
    static M: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    Some(&M.get_or_init(build)[i])
}
