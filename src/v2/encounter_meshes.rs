//! Whole-character meshes for opt-in encounter recipes. These bypass the generic
//! creature renderer: a disguised mimic must never acquire a default face, and a
//! boulder pusher's separately simulated rock must never become sprite geometry.
use super::{
    add_asset, add_connector, node_pose, sphere, visual_v2, AssetOptions, Body, MonsterSpec, Pose,
    TextureKind, Triangle, V3,
};
use std::f32::consts::{PI, TAU};

const SLIME: [u8; 4] = [71, 187, 65, 255];
const SLIME_LIGHT: [u8; 4] = [177, 236, 104, 255];
const DARK: [u8; 4] = [29, 27, 31, 255];
const TOOTH: [u8; 4] = [245, 224, 172, 255];
const BRONZE: [u8; 4] = [163, 103, 44, 255];
const GOLD: [u8; 4] = [225, 173, 79, 255];
const CORE: [u8; 4] = [253, 126, 51, 255];
const HIDE: [u8; 4] = [133, 124, 144, 255];

fn has(spec: &MonsterSpec, feature: &str) -> bool {
    spec.features.iter().any(|f| f == feature)
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
fn index(body: &Body, id: &str) -> Option<usize> {
    body.nodes.iter().position(|n| n.id == id)
}
fn anchor(body: &Body, pose: &Pose, id: &str) -> Option<(V3, V3)> {
    let i = index(body, id)?;
    let n = &body.nodes[i];
    Some((
        node_pose(body, i, pose).0,
        V3::new(n.rx, n.ry, n.rz.unwrap_or(n.rx)),
    ))
}
fn blob(out: &mut Vec<Triangle>, c: V3, r: V3, color: [u8; 4], seed: u32) {
    if r.x.min(r.y).min(r.z) <= 0.001 {
        return;
    }
    let start = out.len();
    add_asset(
        out,
        sphere(),
        c,
        r,
        0.0,
        "FLESH",
        AssetOptions {
            flip_z: false,
            seed,
            irregularity: 0.0,
        },
    );
    for t in &mut out[start..] {
        t.color = color;
        t.texture = TextureKind::None;
    }
}
fn tri(out: &mut Vec<Triangle>, a: V3, b: V3, c: V3, color: [u8; 4]) {
    out.push(Triangle {
        a,
        b,
        c,
        color,
        local: [a, b, c],
        texture: TextureKind::None,
        seed: 0,
    });
}
// A solid tapered tooth, rather than a painted white spot on the disguise.
fn tooth(out: &mut Vec<Triangle>, base: V3, tip: V3, radius: f32) {
    let a = base + V3::new(-radius, 0.0, -radius);
    let b = base + V3::new(radius, 0.0, -radius);
    let c = base + V3::new(radius, 0.0, radius);
    let d = base + V3::new(-radius, 0.0, radius);
    for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
        tri(out, p, q, tip, TOOTH);
    }
    tri(out, a, c, b, TOOTH);
    tri(out, a, d, c, TOOTH);
}
// Ring in the YZ plane, facing forwards (+X).
fn ring(out: &mut Vec<Triangle>, c: V3, ry: f32, rz: f32, thickness: f32, color: [u8; 4]) {
    let count = 24;
    for i in 0..count {
        let a = i as f32 * TAU / count as f32;
        let b = (i + 1) as f32 * TAU / count as f32;
        add_connector(
            out,
            c + V3::new(0.0, a.cos() * ry, a.sin() * rz),
            c + V3::new(0.0, b.cos() * ry, b.sin() * rz),
            thickness,
            color,
        );
    }
}
fn tube(out: &mut Vec<Triangle>, points: &[(V3, f32)], width: f32, color: [u8; 4]) {
    let sides = 12;
    for i in 0..points.len().saturating_sub(1) {
        let (a, ar) = points[i];
        let (b, br) = points[i + 1];
        let direction = (b - a).normalized();
        let side = V3::new(1.0, 0.0, 0.0);
        let up = direction.cross(side).normalized();
        for j in 0..sides {
            let t = j as f32 * TAU / sides as f32;
            let u = (j + 1) as f32 * TAU / sides as f32;
            let p = a + side * (t.cos() * ar * width) + up * (t.sin() * ar);
            let q = a + side * (u.cos() * ar * width) + up * (u.sin() * ar);
            let r = b + side * (t.cos() * br * width) + up * (t.sin() * br);
            let s = b + side * (u.cos() * br * width) + up * (u.sin() * br);
            tri(out, p, q, r, color);
            tri(out, q, s, r, color);
        }
    }
}

fn slime(body: &Body, pose: &Pose, seed: u32) -> Vec<Triangle> {
    let Some((mut c, mut r)) = anchor(body, pose, "torso") else {
        return vec![];
    };
    let mut out = vec![];
    let ground = c.y - r.y;
    let emerge = if pose.clip_id == "v2_emerge" {
        smooth(pose.phase)
    } else {
        1.0
    };
    let split = if pose.clip_id == "v2_split_burst" {
        smooth(pose.phase)
    } else {
        0.0
    };
    let pulse = if split == 0.0 {
        (pose.phase * TAU).sin() * 0.07
    } else {
        0.0
    };
    r.x *= (1.0 - pulse) * (1.0 + (1.0 - emerge) * 0.2);
    r.z *= 1.0 - pulse;
    r.y *= (1.0 + pulse) * (0.08 + 0.92 * emerge);
    c.y = ground + r.y;
    if split < 1.0 {
        let remaining = 1.0 - split;
        let sr = V3::new(
            r.x * remaining.sqrt(),
            r.y * remaining,
            r.z * remaining.sqrt(),
        );
        c.y = ground + sr.y;
        blob(&mut out, c, sr, SLIME, seed);
        for side in [-1.0, 1.0] {
            blob(
                &mut out,
                c + V3::new(sr.x * 0.72, sr.y * 0.28, side * sr.z * 0.36),
                V3::new(sr.x * 0.16, sr.y * 0.29, sr.z * 0.18),
                [235, 245, 187, 255],
                seed,
            );
            blob(
                &mut out,
                c + V3::new(
                    r.x * 0.84 * remaining.sqrt(),
                    sr.y * 0.27,
                    side * sr.z * 0.36,
                ),
                V3::new(sr.x * 0.06, sr.y * 0.18, sr.z * 0.08),
                DARK,
                seed,
            );
        }
        blob(
            &mut out,
            c + V3::new(-sr.x * 0.2, sr.y * 0.72, -sr.z * 0.35),
            V3::new(sr.x * 0.31, sr.y * 0.13, sr.z * 0.18),
            SLIME_LIGHT,
            seed,
        );
        // A scalloped, pooled lower edge makes the jelly silhouette unmistakable.
        for i in 0..6 {
            let a = i as f32 * TAU / 6.0;
            blob(
                &mut out,
                V3::new(
                    c.x + a.cos() * sr.x * 0.7,
                    ground + sr.y * 0.16,
                    c.z + a.sin() * sr.z * 0.7,
                ),
                V3::new(sr.x * 0.34, sr.y * 0.2, sr.z * 0.34),
                SLIME,
                seed,
            );
        }
    }
    if split > 0.0 && split < 1.0 {
        // Decorative droplets only. Child entities belong to the engine contract.
        for i in 0..9 {
            let a = i as f32 * TAU / 9.0 + seed as f32 * 0.017;
            let size = (1.0 - split) * (1.6 + (i % 3) as f32 * 0.7);
            let distance = split * r.x * (1.3 + (i % 2) as f32 * 0.25);
            blob(
                &mut out,
                V3::new(
                    c.x + a.cos() * distance,
                    ground + (PI * split).sin() * r.y * 1.8 + size,
                    c.z + a.sin() * distance,
                ),
                V3::new(size, size * 1.35, size),
                SLIME_LIGHT,
                seed,
            );
        }
    }
    out
}

fn bush(c: V3, r: V3, seed: u32) -> Vec<Triangle> {
    let mut out = vec![];
    add_connector(
        &mut out,
        c - V3::new(0.0, r.y * 0.9, 0.0),
        c,
        r.x * 0.17,
        [91, 63, 34, 255],
    );
    blob(
        &mut out,
        c,
        V3::new(r.x * 0.87, r.y * 0.85, r.z * 0.9),
        [48, 110, 49, 255],
        seed,
    );
    for i in 0..9 {
        let a = i as f32 * TAU / 9.0;
        let y = if i % 2 == 0 { 0.27 } else { -0.2 };
        blob(
            &mut out,
            c + V3::new(a.cos() * r.x * 0.61, y * r.y, a.sin() * r.z * 0.65),
            V3::new(r.x * 0.49, r.y * 0.57, r.z * 0.48),
            if i % 3 == 0 {
                [87, 145, 56, 255]
            } else {
                [57, 124, 48, 255]
            },
            seed,
        );
    }
    // A few visibly botanical pointed leaves, no facial marks while disguised.
    for i in 0..8 {
        let a = i as f32 * TAU / 8.0 + 0.2;
        let p = c + V3::new(a.cos() * r.x * 0.7, r.y * 0.55, a.sin() * r.z * 0.7);
        let tip = c + V3::new(a.cos() * r.x * 1.15, r.y * 0.92, a.sin() * r.z * 1.15);
        let side = V3::new(-a.sin() * r.x * 0.18, 0.0, a.cos() * r.z * 0.18);
        tri(&mut out, p - side, tip, p + side, [112, 160, 65, 255]);
    }
    out
}
fn banana(c: V3, r: V3, seed: u32) -> Vec<Triangle> {
    let mut out = vec![];
    let mut curve = vec![];
    for i in 0..15 {
        let u = i as f32 / 14.0 * 2.0 - 1.0;
        let center = c + V3::new(0.0, r.y * (-0.24 + 1.1 * u * u), r.z * u * 1.35);
        let radius = r.y * (0.13 + 0.24 * (1.0 - u * u));
        curve.push((center, radius));
    }
    tube(&mut out, &curve, r.x / r.y * 1.1, [237, 193, 51, 255]);
    for (i, (tip, radius)) in [curve[0], curve[curve.len() - 1]].into_iter().enumerate() {
        blob(
            &mut out,
            tip,
            V3::new(radius * 0.8, radius * 0.85, radius * 0.8),
            [91, 66, 32, 255],
            seed,
        );
        if i == 1 {
            add_connector(
                &mut out,
                tip,
                tip + V3::new(0.0, r.y * 0.21, r.z * 0.06),
                radius * 0.53,
                [100, 82, 34, 255],
            );
        }
    }
    // A narrow peel ridge follows the crescent, preserving its organic curvature.
    for pair in curve.windows(2).skip(2).take(10) {
        add_connector(
            &mut out,
            pair[0].0 + V3::new(pair[0].1 * r.x / r.y * 1.1, 0.0, 0.0),
            pair[1].0 + V3::new(pair[1].1 * r.x / r.y * 1.1, 0.0, 0.0),
            r.y * 0.025,
            [253, 220, 92, 255],
        );
    }
    // Lay the crescent diagonally across the facing/lateral axes. A pure Z/Y
    // curve reads edge-on as a pear in game side views (directions 0 and 2).
    // Rotating the completed sweep avoids changing its stable tube frame, and
    // retains a broad curved banana silhouette in all four baked directions.
    let (sin, cos) = std::f32::consts::FRAC_PI_4.sin_cos();
    for triangle in &mut out {
        for p in [&mut triangle.a, &mut triangle.b, &mut triangle.c] {
            let q = *p - c;
            *p = c + V3::new(q.x * cos - q.z * sin, q.y, q.x * sin + q.z * cos);
        }
    }
    out
}

fn vase(c: V3, r: V3, seed: u32) -> Vec<Triangle> {
    let mut out = vec![];
    let profile = [
        (-1.0, 0.38),
        (-0.9, 0.5),
        (-0.72, 0.54),
        (-0.37, 0.9),
        (0.08, 1.0),
        (0.46, 0.68),
        (0.66, 0.38),
        (0.91, 0.4),
        (1.0, 0.59),
    ];
    for pair in profile.windows(2) {
        for i in 0..24 {
            let a = i as f32 * TAU / 24.0;
            let b = (i + 1) as f32 * TAU / 24.0;
            let point = |(y, width): (f32, f32), t: f32| {
                c + V3::new(t.cos() * width * r.x, y * r.y, t.sin() * width * r.z)
            };
            let color = if pair[0].0 < -0.72 || pair[0].0 > 0.8 {
                [109, 57, 34, 255]
            } else {
                [183, 101, 58, 255]
            };
            tri(
                &mut out,
                point(pair[0], a),
                point(pair[0], b),
                point(pair[1], a),
                color,
            );
            tri(
                &mut out,
                point(pair[0], b),
                point(pair[1], b),
                point(pair[1], a),
                color,
            );
        }
    }
    blob(
        &mut out,
        c + V3::new(0.0, r.y * 0.97, 0.0),
        V3::new(r.x * 0.48, r.y * 0.025, r.z * 0.48),
        DARK,
        seed,
    );
    // Amphora handles remain hollow loops, unlike box/chest silhouettes.
    for side in [-1.0, 1.0] {
        let mut points = vec![];
        for i in 0..17 {
            let a = i as f32 * TAU / 16.0;
            points.push((
                c + V3::new(
                    0.0,
                    r.y * (0.37 + a.cos() * 0.32),
                    side * r.z * (0.73 + a.sin() * 0.37),
                ),
                r.x * 0.09,
            ));
        }
        tube(&mut out, &points, 1.0, [155, 79, 43, 255]);
    }
    // Painted horizontal bands wrap the pot rather than creating facial features.
    for y in [-0.47, -0.36, 0.21, 0.29] {
        let width = if y < 0.0 { 0.84 } else { 0.9 };
        for i in 0..24 {
            let a = i as f32 * TAU / 24.0;
            let b = (i + 1) as f32 * TAU / 24.0;
            add_connector(
                &mut out,
                c + V3::new(a.cos() * width * r.x, y * r.y, a.sin() * width * r.z),
                c + V3::new(b.cos() * width * r.x, y * r.y, b.sin() * width * r.z),
                r.y * 0.025,
                [224, 173, 105, 255],
            );
        }
    }
    out
}

// Clip the closed disguise at the mouth plane before separating the jaws. This
// prevents any crossing surface triangles from stretching across and sealing it.
fn separate_jaws(source: Vec<Triangle>, plane: f32, upper: V3, lower: V3) -> Vec<Triangle> {
    let mut out = Vec::with_capacity(source.len() + 300);
    for t in source {
        for (sign, delta) in [(1.0, upper), (-1.0, lower)] {
            let mut polygon = vec![];
            let vertices = [t.a, t.b, t.c];
            for i in 0..3 {
                let a = vertices[i];
                let b = vertices[(i + 1) % 3];
                let inside_a = (a.y - plane) * sign >= 0.0;
                let inside_b = (b.y - plane) * sign >= 0.0;
                if inside_a {
                    polygon.push(a);
                }
                if inside_a != inside_b {
                    let u = (plane - a.y) / (b.y - a.y);
                    polygon.push(a + (b - a) * u);
                }
            }
            for i in 1..polygon.len().saturating_sub(1) {
                let [a, b, c] = [
                    polygon[0] + delta,
                    polygon[i] + delta,
                    polygon[i + 1] + delta,
                ];
                out.push(Triangle {
                    a,
                    b,
                    c,
                    local: [a, b, c],
                    ..t
                });
            }
        }
    }
    out
}
fn jaw_offset(body: &Body, pose: &Pose, id: &str, torso: V3) -> V3 {
    let (Some(i), Some(root)) = (index(body, id), index(body, "torso")) else {
        return V3::default();
    };
    let n = &body.nodes[i];
    let r = &body.nodes[root];
    let rest = V3::new(
        n.x - r.x,
        -(n.y - r.y),
        n.z.unwrap_or(0.0) - r.z.unwrap_or(0.0),
    );
    node_pose(body, i, pose).0 - torso - rest
}
fn mimic(spec: &MonsterSpec, body: &Body, pose: &Pose, seed: u32) -> Vec<Triangle> {
    let Some((c, r)) = anchor(body, pose, "torso") else {
        return vec![];
    };
    let source = if has(spec, "MIMIC_BANANA") {
        banana(c, r, seed)
    } else if has(spec, "MIMIC_VASE") {
        vase(c, r, seed)
    } else {
        bush(c, r, seed)
    };
    let openness = if pose.clip_id == "v2_reveal" {
        smooth(pose.phase)
    } else if pose.state == "LURE" || pose.clip_id == "v2_lure" {
        0.0
    } else {
        0.72
    };
    if openness <= 0.001 {
        return source;
    }
    let mut upper = jaw_offset(body, pose, "v2_mimic_upper", c);
    let mut lower = jaw_offset(body, pose, "v2_mimic_lower", c);
    // Respect the anatomical jaw pose. Fallback keeps hand-authored preview poses
    // useful when they intentionally omit physical_positions.
    if pose.physical_positions.is_empty() {
        upper.y = r.y * 0.57 * openness;
        lower.y = -r.y * 0.22 * openness;
    }
    let mut out = separate_jaws(source, c.y, upper, lower);
    let gap = (upper.y - lower.y).max(0.0);
    let mid = c + (upper + lower) * 0.5;
    blob(
        &mut out,
        mid,
        V3::new(r.x * 0.88, gap * 0.5, r.z * 0.85),
        [58, 24, 34, 255],
        seed,
    );
    blob(
        &mut out,
        mid + V3::new(r.x * 0.83, -gap * 0.23, 0.0),
        V3::new(r.x * 0.18, gap * 0.13, r.z * 0.47),
        [167, 57, 71, 255],
        seed,
    );
    for i in 0..7 {
        let z = (i as f32 - 3.0) * r.z * 0.23;
        let x = c.x + r.x * (0.85 - 0.08 * ((i as f32 - 3.0) / 3.0).powi(2));
        for (delta, direction) in [(upper, -1.0), (lower, 1.0)] {
            let base = V3::new(x, c.y, c.z + z) + delta;
            tooth(
                &mut out,
                base,
                base + V3::new(r.x * 0.045, direction * gap * 0.34, 0.0),
                r.z * 0.075,
            );
        }
    }
    out
}

fn limbs(out: &mut Vec<Triangle>, body: &Body, pose: &Pose, seed: u32, sentinel: bool) {
    for (i, node) in body.nodes.iter().enumerate() {
        if !matches!(
            node.kind.as_str(),
            "SHOULDER" | "ARM" | "HAND" | "HIP" | "LIMB" | "SHIN" | "FOOT"
        ) {
            continue;
        }
        let c = node_pose(body, i, pose).0;
        let r = V3::new(node.rx, node.ry, node.rz.unwrap_or(node.rx));
        let color = if sentinel { BRONZE } else { HIDE };
        if let Some(parent) = node.parent {
            add_connector(
                out,
                node_pose(body, parent, pose).0,
                c,
                node.rx.min(node.ry) * 0.67,
                color,
            );
        }
        blob(out, c, r, color, seed);
        if node.kind == "HAND" && !sentinel {
            // Open, empty palms and separated forward-pointing fingers. There is
            // deliberately no procedural boulder or weapon in this renderer.
            for finger in 0..4 {
                let z = (finger as f32 - 1.5) * r.z * 0.42;
                let start = c + V3::new(r.x * 0.5, -r.y * 0.12, z);
                let end = start
                    + V3::new(
                        r.x * (0.6 + (1.5 - (finger as f32 - 1.5).abs()) * 0.12),
                        0.0,
                        0.0,
                    );
                add_connector(out, start, end, r.z * 0.17, [158, 143, 161, 255]);
                blob(
                    out,
                    end,
                    V3::new(r.z * 0.17, r.z * 0.17, r.z * 0.17),
                    [158, 143, 161, 255],
                    seed,
                );
            }
        }
    }
}
fn sentinel(body: &Body, pose: &Pose, seed: u32) -> Vec<Triangle> {
    let Some((c, r)) = anchor(body, pose, "torso") else {
        return vec![];
    };
    let mut out = vec![];
    limbs(&mut out, body, pose, seed, true);
    blob(&mut out, c, r, [96, 65, 43, 255], seed);
    for side in [-1.0, 1.0] {
        blob(
            &mut out,
            c + V3::new(0.0, r.y * 0.75, side * r.z * 0.87),
            V3::new(r.x * 0.82, r.y * 0.29, r.z * 0.45),
            BRONZE,
            seed,
        );
    }
    if let Some((core, cr)) = anchor(body, pose, "v2_core") {
        blob(
            &mut out,
            core,
            V3::new(cr.x, cr.y * 1.3, cr.z * 1.3),
            DARK,
            seed,
        );
        blob(
            &mut out,
            core + V3::new(cr.x * 0.42, 0.0, 0.0),
            cr,
            CORE,
            seed,
        );
        ring(
            &mut out,
            core + V3::new(cr.x * 0.8, 0.0, 0.0),
            cr.y * 1.1,
            cr.z * 1.1,
            cr.y * 0.17,
            GOLD,
        );
    }
    if let Some((h, hr)) = anchor(body, pose, "head_0") {
        blob(&mut out, h, hr, BRONZE, seed);
        blob(
            &mut out,
            h + V3::new(hr.x * 0.83, 0.0, 0.0),
            V3::new(hr.x * 0.25, hr.y * 0.17, hr.z * 0.78),
            DARK,
            seed,
        );
        blob(
            &mut out,
            h + V3::new(hr.x * 1.0, -hr.y * 0.2, 0.0),
            V3::new(hr.x * 0.14, hr.y * 0.48, hr.z * 0.15),
            GOLD,
            seed,
        );
        // Tall longitudinal helmet crest reads from all four baked directions.
        for i in 0..6 {
            let x = (i as f32 - 2.5) * hr.x * 0.29;
            let y = hr.y * (0.85 + 0.34 * (1.0 - (x / hr.x).powi(2)));
            blob(
                &mut out,
                h + V3::new(x, y, 0.0),
                V3::new(hr.x * 0.23, hr.y * 0.48, hr.z * 0.19),
                [137, 44, 34, 255],
                seed,
            );
        }
    }
    if let Some((s, sr)) = anchor(body, pose, "v2_shield") {
        let shield_start = out.len();
        blob(&mut out, s, sr, BRONZE, seed);
        blob(
            &mut out,
            s + V3::new(sr.x * 0.6, 0.0, 0.0),
            V3::new(sr.x * 0.6, sr.y * 0.89, sr.z * 0.89),
            [183, 121, 49, 255],
            seed,
        );
        ring(
            &mut out,
            s + V3::new(sr.x * 0.5, 0.0, 0.0),
            sr.y * 0.95,
            sr.z * 0.95,
            sr.x * 0.44,
            GOLD,
        );
        ring(
            &mut out,
            s + V3::new(sr.x * 1.13, 0.0, 0.0),
            sr.y * 0.59,
            sr.z * 0.59,
            sr.x * 0.18,
            GOLD,
        );
        blob(
            &mut out,
            s + V3::new(sr.x * 1.25, 0.0, 0.0),
            V3::new(sr.x * 1.3, sr.y * 0.21, sr.z * 0.21),
            GOLD,
            seed,
        );
        for i in 0..8 {
            let a = i as f32 * TAU / 8.0;
            blob(
                &mut out,
                s + V3::new(sr.x * 0.85, a.cos() * sr.y * 0.8, a.sin() * sr.z * 0.8),
                V3::new(sr.x * 0.32, sr.y * 0.035, sr.z * 0.035),
                [246, 195, 105, 255],
                seed,
            );
        }
        // The shield is a rigid prop: its posed tilt rotates the rim, boss and
        // decorations together, instead of only moving their shared center.
        let angle = -node_pose(body, index(body, "v2_shield").unwrap(), pose).1;
        let (sin, cos) = angle.sin_cos();
        for triangle in &mut out[shield_start..] {
            for p in [&mut triangle.a, &mut triangle.b, &mut triangle.c] {
                let q = *p - s;
                *p = s + V3::new(q.x * cos - q.y * sin, q.x * sin + q.y * cos, q.z);
            }
        }
    }
    out
}
fn pusher(body: &Body, pose: &Pose, seed: u32) -> Vec<Triangle> {
    let Some((c, r)) = anchor(body, pose, "torso") else {
        return vec![];
    };
    let mut out = vec![];
    limbs(&mut out, body, pose, seed, false);
    blob(&mut out, c, r, HIDE, seed);
    blob(
        &mut out,
        c + V3::new(-r.x * 0.44, r.y * 0.4, 0.0),
        V3::new(r.x * 0.79, r.y * 0.72, r.z * 0.93),
        [118, 110, 129, 255],
        seed,
    );
    // Leather waist wrap and belt visually distinguish organic body from stone.
    blob(
        &mut out,
        c - V3::new(0.0, r.y * 0.76, 0.0),
        V3::new(r.x * 0.74, r.y * 0.32, r.z * 0.93),
        [82, 56, 41, 255],
        seed,
    );
    if let Some((h, hr)) = anchor(body, pose, "head_0") {
        blob(&mut out, h, hr, HIDE, seed);
        blob(
            &mut out,
            h + V3::new(hr.x * 0.73, -hr.y * 0.2, 0.0),
            V3::new(hr.x * 0.48, hr.y * 0.44, hr.z * 0.7),
            [158, 142, 158, 255],
            seed,
        );
        for side in [-1.0, 1.0] {
            blob(
                &mut out,
                h + V3::new(hr.x * 0.87, hr.y * 0.22, side * hr.z * 0.39),
                V3::new(hr.x * 0.15, hr.y * 0.12, hr.z * 0.17),
                DARK,
                seed,
            );
            blob(
                &mut out,
                h + V3::new(hr.x * 0.95, hr.y * 0.21, side * hr.z * 0.39),
                V3::new(hr.x * 0.065, hr.y * 0.05, hr.z * 0.08),
                [224, 191, 90, 255],
                seed,
            );
            blob(
                &mut out,
                h + V3::new(hr.x * 0.69, hr.y * 0.4, side * hr.z * 0.4),
                V3::new(hr.x * 0.36, hr.y * 0.19, hr.z * 0.34),
                [103, 94, 114, 255],
                seed,
            );
        }
        blob(
            &mut out,
            h + V3::new(hr.x * 1.06, -hr.y * 0.46, 0.0),
            V3::new(hr.x * 0.07, hr.y * 0.05, hr.z * 0.44),
            DARK,
            seed,
        );
    }
    out
}

pub(super) fn scene(
    spec: &MonsterSpec,
    body: &Body,
    pose: &Pose,
    seed: u64,
) -> Option<Vec<Triangle>> {
    if !visual_v2(spec) {
        return None;
    }
    let seed = seed as u32;
    if has(spec, "SPLITTING_SLIME") {
        Some(slime(body, pose, seed))
    } else if has(spec, "OBJECT_MIMIC") {
        Some(mimic(spec, body, pose, seed))
    } else if has(spec, "SHIELD_SENTINEL") {
        Some(sentinel(body, pose, seed))
    } else if has(spec, "BOULDER_PUSHER") {
        Some(pusher(body, pose, seed))
    } else {
        None
    }
}

/// Keep identifying colors during palette reduction. No V1 palette is changed.
pub(super) fn palette(spec: &MonsterSpec) -> Option<Vec<[u8; 4]>> {
    if !visual_v2(spec) {
        return None;
    }
    let bases = if has(spec, "SPLITTING_SLIME") {
        vec![SLIME, SLIME_LIGHT, [235, 245, 187, 255], DARK]
    } else if has(spec, "OBJECT_MIMIC") {
        let mut c = vec![DARK, TOOTH, [58, 24, 34, 255], [167, 57, 71, 255]];
        if has(spec, "MIMIC_BANANA") {
            c.extend([[237, 193, 51, 255], [253, 220, 92, 255], [91, 66, 32, 255]]);
        } else if has(spec, "MIMIC_VASE") {
            c.extend([
                [183, 101, 58, 255],
                [109, 57, 34, 255],
                [224, 173, 105, 255],
            ]);
        } else {
            c.extend([
                [48, 110, 49, 255],
                [87, 145, 56, 255],
                [112, 160, 65, 255],
                [91, 63, 34, 255],
            ]);
        }
        c
    } else if has(spec, "SHIELD_SENTINEL") {
        vec![
            DARK,
            BRONZE,
            GOLD,
            CORE,
            [96, 65, 43, 255],
            [137, 44, 34, 255],
        ]
    } else if has(spec, "BOULDER_PUSHER") {
        vec![
            DARK,
            HIDE,
            [118, 110, 129, 255],
            [158, 142, 158, 255],
            [82, 56, 41, 255],
            [224, 191, 90, 255],
        ]
    } else {
        return None;
    };
    let mut colors = vec![];
    for base in bases {
        for gain in [0.45, 0.67, 0.85, 1.0, 1.2] {
            let mut color = base;
            for channel in &mut color[..3] {
                *channel = (*channel as f32 * gain).min(255.0) as u8;
            }
            if !colors.contains(&color) {
                colors.push(color);
            }
        }
    }
    Some(colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn fixture(feature: &str, object: Option<&str>) -> (MonsterSpec, Body, Vec<Pose>) {
        let mut spec = crate::parser::parse_vocabulary("humanoid");
        spec.features.extend(["PROFILE_V2".into(), feature.into()]);
        if let Some(object) = object {
            spec.features.push(object.into());
        }
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(8));
        assert!(crate::v2::encounter_visuals::refine(&spec, &mut body));
        let (mut clips, mut modes, mut attacks, mut poses) = crate::animation::make(&spec, &body);
        crate::v2::encounter_visuals::animate(
            &spec,
            &body,
            &mut clips,
            &mut modes,
            &mut attacks,
            &mut poses,
        );
        (spec, body, poses)
    }
    fn bounds(mesh: &[Triangle]) -> (V3, V3) {
        let mut min = V3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
        let mut max = V3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in mesh.iter().flat_map(|t| [t.a, t.b, t.c]) {
            assert!(p.x.is_finite() && p.y.is_finite() && p.z.is_finite());
            min.x = min.x.min(p.x);
            min.y = min.y.min(p.y);
            min.z = min.z.min(p.z);
            max.x = max.x.max(p.x);
            max.y = max.y.max(p.y);
            max.z = max.z.max(p.z);
        }
        (min, max)
    }
    #[test]
    fn encounter_renderer_and_palette_require_explicit_v2_gate() {
        let (mut spec, body, poses) = fixture("SPLITTING_SLIME", None);
        assert!(scene(&spec, &body, &poses[0], 8).is_some());
        spec.features.retain(|f| f != "PROFILE_V2");
        assert!(scene(&spec, &body, &poses[0], 8).is_none());
        assert!(palette(&spec).is_none());
    }
    #[test]
    fn disguises_have_no_face_or_teeth_and_reveals_have_physical_teeth() {
        for object in ["MIMIC_BUSH", "MIMIC_BANANA", "MIMIC_VASE"] {
            let (spec, body, poses) = fixture("OBJECT_MIMIC", Some(object));
            let closed_pose = poses.iter().find(|p| p.clip_id == "v2_lure").unwrap();
            let open_pose = poses.iter().rfind(|p| p.clip_id == "v2_reveal").unwrap();
            let closed = scene(&spec, &body, closed_pose, 8).unwrap();
            let open = scene(&spec, &body, open_pose, 8).unwrap();
            assert!(closed
                .iter()
                .all(|t| t.color != TOOTH && t.color != [242, 79, 127, 255]));
            assert!(
                open.iter().filter(|t| t.color == TOOTH).count() >= 84,
                "{object}"
            );
            let (_, closed_max) = bounds(&closed);
            let (_, open_max) = bounds(&open);
            assert!(open_max.y > closed_max.y + 5.0, "{object}");
            assert_ne!(closed.len(), open.len());
        }
    }
    #[test]
    fn banana_crescent_is_broad_in_both_side_and_front_views() {
        let (spec, body, poses) = fixture("OBJECT_MIMIC", Some("MIMIC_BANANA"));
        let closed_pose = poses.iter().find(|p| p.clip_id == "v2_lure").unwrap();
        let mesh = scene(&spec, &body, closed_pose, 8).unwrap();
        let (min, max) = bounds(&mesh);
        let (_, r) = anchor(&body, closed_pose, "torso").unwrap();
        let forward_span = max.x - min.x;
        let lateral_span = max.z - min.z;
        assert!(
            forward_span > r.z * 1.85,
            "side views must show the crescent width"
        );
        assert!(
            lateral_span > r.z * 1.85,
            "front views must retain the crescent width"
        );
        assert!((forward_span - lateral_span).abs() < r.z * 0.1);
    }

    #[test]
    fn slime_emerges_and_split_burst_removes_parent_surface() {
        let (spec, body, poses) = fixture("SPLITTING_SLIME", None);
        let emerge: Vec<_> = poses.iter().filter(|p| p.clip_id == "v2_emerge").collect();
        let initial = scene(&spec, &body, emerge[0], 8).unwrap();
        let full = scene(&spec, &body, emerge[emerge.len() - 1], 8).unwrap();
        let (a, b) = bounds(&initial);
        let (c, d) = bounds(&full);
        assert!(d.y - c.y > (b.y - a.y) * 5.0);
        let split = poses
            .iter()
            .rfind(|p| p.clip_id == "v2_split_burst")
            .unwrap();
        assert!(scene(&spec, &body, split, 8).unwrap().is_empty());
    }
    #[test]
    fn shield_recovers_as_rigid_geometry_and_exposes_core() {
        let (spec, body, poses) = fixture("SHIELD_SENTINEL", None);
        let guard_pose = poses.iter().find(|p| p.clip_id == "v2_guard").unwrap();
        let recover_pose = poses
            .iter()
            .find(|p| p.clip_id == "v2_shield_recovery")
            .unwrap();
        let guard = scene(&spec, &body, guard_pose, 8).unwrap();
        let recovery = scene(&spec, &body, recover_pose, 8).unwrap();
        assert!(guard.iter().any(|t| t.color == CORE));
        let draw = |triangles: &[Triangle], pose: &Pose| {
            // Object-ID colors test actual occlusion rather than confusing the
            // bronze shield's orange highlights with the glowing core.
            let tagged: Vec<_> = triangles
                .iter()
                .map(|t| Triangle {
                    color: if t.color == CORE {
                        [255, 0, 255, 255]
                    } else {
                        [0, 0, 0, 255]
                    },
                    ..*t
                })
                .collect();
            super::super::frame(
                &tagged,
                pose,
                192,
                PI * 0.5,
                &[[255, 0, 255, 255], [0, 0, 0, 255]],
                &mut std::collections::HashMap::new(),
            )
        };
        let core_pixels = |image: image::RgbaImage| {
            image
                .pixels()
                .filter(|p| p[0] > 0 && p[2] > 0 && p[3] > 0)
                .count()
        };
        let hidden = core_pixels(draw(&guard, guard_pose));
        let exposed = core_pixels(draw(&recovery, recover_pose));
        assert!(
            exposed > hidden + 5,
            "core pixels: guard={hidden}, recovery={exposed}"
        );
    }
    #[test]
    fn pusher_is_empty_handed_and_follows_posed_hands() {
        let (spec, body, poses) = fixture("BOULDER_PUSHER", None);
        let resting = poses.iter().find(|p| p.clip_id == "v2_push").unwrap();
        let pushing = poses.iter().rfind(|p| p.clip_id == "v2_push").unwrap();
        let first = scene(&spec, &body, resting, 8).unwrap();
        let last = scene(&spec, &body, pushing, 8).unwrap();
        let (_, first_max) = bounds(&first);
        let (_, last_max) = bounds(&last);
        assert!(last_max.x > first_max.x + 5.0);
        let (hand, r) = anchor(&body, pushing, "hand_0").unwrap();
        assert!(
            last_max.x <= hand.x + r.x * 1.6,
            "a held boulder must not be baked"
        );
        assert!(last.iter().all(|t| t.texture != TextureKind::Stone));
    }
    #[test]
    fn named_anchors_do_not_depend_on_root_node_ordinal() {
        let (spec, mut body, poses) = fixture("SPLITTING_SLIME", None);
        let mut pose = poses[0].clone();
        let before = scene(&spec, &body, &pose, 8).unwrap();
        body.nodes.swap(0, 1);
        pose.physical_positions.swap(0, 1);
        for n in &mut body.nodes {
            n.parent = n.parent.map(|i| {
                if i == 0 {
                    1
                } else if i == 1 {
                    0
                } else {
                    i
                }
            });
        }
        let after = scene(&spec, &body, &pose, 8).unwrap();
        assert_eq!(before.len(), after.len());
        for (a, b) in before.iter().zip(&after) {
            assert_eq!(
                [a.a.x, a.a.y, a.a.z, a.b.x, a.b.y, a.b.z],
                [b.a.x, b.a.y, b.a.z, b.b.x, b.b.y, b.b.z]
            );
        }
    }
}
