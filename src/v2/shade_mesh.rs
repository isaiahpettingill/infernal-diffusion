//! Whole-surface shade unwinding. Every source vertex follows the same continuous
//! three-branch field; branch boundaries meet at the shared root rather than tear.
use super::{Body, Pose, TextureKind, Triangle, V3};

fn chain_indices(body: &Body) -> Option<[[usize; 5]; 3]> {
    let mut indices = [[usize::MAX; 5]; 3];
    for (index, node) in body.nodes.iter().enumerate() {
        if let Some(id) = node.id.strip_prefix("v2_shade_tendril_") {
            let (chain, segment) = id.split_once('_')?;
            *indices
                .get_mut(chain.parse::<usize>().ok()?)?
                .get_mut(segment.parse::<usize>().ok()?)? = index;
        }
    }
    indices
        .iter()
        .flatten()
        .all(|&index| index != usize::MAX)
        .then_some(indices)
}

/// Keep every contact joint and the pointed tip on the rig. The intervening
/// Catmull-Rom midpoints round each bend. End tangents follow the end links.
fn centerline(indices: [usize; 5], positions: &[(V3, f32)]) -> [V3; 11] {
    let mut knots = [positions[0].0; 6];
    for (k, i) in indices.into_iter().enumerate() {
        knots[k + 1] = positions[i].0;
    }
    std::array::from_fn(|i| {
        let k = i / 2;
        if i % 2 == 0 {
            return knots[k];
        }
        let (a, b) = (knots[k], knots[k + 1]);
        let prev = if k == 0 { a + (a - b) } else { knots[k - 1] };
        let next = if k + 2 == knots.len() {
            b + (b - a)
        } else {
            knots[k + 2]
        };
        (a + b) * 0.5 + (a + b - prev - next) * 0.0625
    })
}

pub(super) fn strands(out: &mut Vec<Triangle>, body: &Body, positions: &[(V3, f32)]) {
    let Some(chains) = chain_indices(body) else {
        return;
    };
    const SIDES: usize = 8;
    let circle: [(f32, f32); SIDES] =
        std::array::from_fn(|i| (std::f32::consts::TAU * i as f32 / SIDES as f32).sin_cos());
    let mut face = |a, b, c| {
        out.push(Triangle {
            a,
            b,
            c,
            color: [0, 0, 0, 255],
            local: [V3::default(); 3],
            texture: TextureKind::None,
            seed: 0,
        })
    };
    for indices in chains {
        let path = centerline(indices, positions);
        let radii = [
            body.nodes[indices[0]].rx * 1.6,
            body.nodes[indices[0]].rx,
            body.nodes[indices[1]].rx,
            body.nodes[indices[2]].rx,
            body.nodes[indices[3]].rx,
            0.0,
        ];
        let mut last = [V3::default(); SIDES];
        let mut side = V3::new(0.0, 0.0, 1.0);
        for i in 0..10 {
            let tangent = (path[i + 1] - path[i.saturating_sub(1)]).normalized();
            // Transport one ring axis along the spine to avoid a twisting seam.
            side = (side - tangent * side.dot(tangent)).normalized();
            let up = tangent.cross(side);
            let radius = if i % 2 == 0 {
                radii[i / 2]
            } else {
                (radii[i / 2] + radii[i / 2 + 1]) * 0.5
            };
            let ring = circle.map(|(s, c)| path[i] + (side * c + up * s) * radius);
            for j in 0..SIDES {
                let k = (j + 1) % SIDES;
                if i == 0 {
                    face(path[0], ring[k], ring[j]);
                } else {
                    face(last[j], last[k], ring[j]);
                    face(last[k], ring[k], ring[j]);
                }
                if i == 9 {
                    face(ring[j], ring[k], path[10]);
                }
            }
            last = ring;
        }
    }
}

pub(super) struct Field {
    chains: [[V3; 11]; 3],
    root: V3,
    extent: f32,
    amount: f32,
}
impl Field {
    pub(super) fn new(body: &Body, pose: &Pose, positions: &[(V3, f32)]) -> Option<Self> {
        if pose.state != "SHADE_UNWIND" || pose.phase <= 0.0 || positions.is_empty() {
            return None;
        }
        let chains = chain_indices(body)?.map(|indices| centerline(indices, positions));
        let root = positions[0].0;
        let extent = body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.id.starts_with("v2_shade_tendril_"))
            .map(|(i, n)| {
                let q = positions[i].0 - root;
                (q.x * q.x + q.y * q.y + q.z * q.z).sqrt()
                    + n.rx.max(n.ry).max(n.rz.unwrap_or(n.rx)) * 2.0
            })
            .fold(1.0_f32, f32::max);
        // Keep the body readable during the coil, then pull it into the strike.
        let phase = pose.phase.clamp(0.0, 1.0).powi(2);
        let amount = phase * phase * (3.0 - 2.0 * phase);
        Some(Self {
            chains,
            root,
            extent,
            amount,
        })
    }
    pub(super) fn map(&self, p: V3) -> V3 {
        let Self {
            chains,
            root,
            extent,
            amount,
        } = self;
        let q = p - *root;
        // A tilted polar chart distributes head, feet and lateral limbs across
        // all three strands. At chart boundaries sin² is zero: both branches
        // map to exactly the common root, maintaining a connected surface.
        let angle = (q.z + q.y * 0.57)
            .atan2(q.x + q.y * 0.31)
            .rem_euclid(std::f32::consts::TAU);
        let sector = angle * 3.0 / std::f32::consts::TAU;
        let chain = (sector.floor() as usize).min(2);
        let fan = (sector.fract() * std::f32::consts::PI).sin().powi(2);
        let distance = (q.x * q.x + q.y * q.y + q.z * q.z).sqrt() / extent;
        let u = (distance.sqrt() * fan * 10.0).clamp(0.0, 9.999_999);
        let segment = u.floor() as usize;
        let t = u.fract();
        let target = chains[chain][segment] * (1.0 - t) + chains[chain][segment + 1] * t;
        p * (1.0 - *amount) + target * *amount
    }
}
pub(super) fn unwind(
    triangles: &mut Vec<Triangle>,
    body: &Body,
    pose: &Pose,
    positions: &[(V3, f32)],
) {
    let Some(field) = Field::new(body, pose, positions) else {
        return;
    };
    // At contact the surface has zero thickness on the centerlines. The
    // swept strands already cover it, including their needle tips.
    if field.amount == 1.0 {
        triangles.clear();
        return;
    }
    fn warp(
        field: &Field,
        source: &Triangle,
        points: [V3; 3],
        mapped: [V3; 3],
        depth: u8,
        out: &mut Vec<Triangle>,
    ) {
        let [a, b, c] = points;
        let [ma, mb, mc] = mapped;
        if depth < 5 {
            let mid = [(a + b) * 0.5, (b + c) * 0.5, (c + a) * 0.5];
            let warped = mid.map(|p| field.map(p));
            let linear = [(ma + mb) * 0.5, (mb + mc) * 0.5, (mc + ma) * 0.5];
            if warped.iter().zip(linear).any(|(&p, q)| {
                let d = p - q;
                d.dot(d) > 0.12
            }) {
                let [ab, bc, ca] = mid;
                let [mab, mbc, mca] = warped;
                for (pts, mapped) in [
                    ([a, ab, ca], [ma, mab, mca]),
                    ([ab, b, bc], [mab, mb, mbc]),
                    ([ca, bc, c], [mca, mbc, mc]),
                    ([ab, bc, ca], [mab, mbc, mca]),
                ] {
                    warp(field, source, pts, mapped, depth + 1, out);
                }
                return;
            }
        }
        out.push(Triangle {
            a: ma,
            b: mb,
            c: mc,
            ..*source
        });
    }
    let original = std::mem::take(triangles);
    triangles.reserve(original.len());
    for triangle in original {
        let points = [triangle.a, triangle.b, triangle.c];
        warp(
            &field,
            &triangle,
            points,
            points.map(|p| field.map(p)),
            0,
            triangles,
        );
    }
}
