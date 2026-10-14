//! Whole-surface shade unwinding. Every source vertex follows the same continuous
//! three-branch field; branch boundaries meet at the shared root rather than tear.
use super::{Body, Pose, Triangle, V3};

pub(super) struct Field {
    chains: [[V3; 6]; 3],
    root: V3,
    extent: f32,
    amount: f32,
}
impl Field {
    pub(super) fn new(body: &Body, pose: &Pose, positions: &[(V3, f32)]) -> Option<Self> {
        if pose.state != "SHADE_UNWIND" || pose.phase <= 0.0 || positions.is_empty() {
            return None;
        }
        let mut chains = [[V3::default(); 6]; 3];
        for (chain, points) in chains.iter_mut().enumerate() {
            points[0] = positions[0].0;
            for (segment, point) in points.iter_mut().enumerate().skip(1) {
                let id = format!("v2_shade_tendril_{chain}_{}", segment - 1);
                let index = body.nodes.iter().position(|n| n.id == id)?;
                *point = positions[index].0;
            }
        }
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
        let phase = pose.phase.clamp(0.0, 1.0);
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
        let u = (distance.sqrt() * fan * 5.0).clamp(0.0, 4.999_999);
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
    fn warp(field: &Field, source: Triangle, depth: u8, out: &mut Vec<Triangle>) {
        let [a, b, c] = [source.a, source.b, source.c];
        let [ab, bc, ca] = [(a + b) * 0.5, (b + c) * 0.5, (c + a) * 0.5];
        let [ma, mb, mc] = [field.map(a), field.map(b), field.map(c)];
        let error = [
            (field.map(ab) - (ma + mb) * 0.5),
            (field.map(bc) - (mb + mc) * 0.5),
            (field.map(ca) - (mc + ma) * 0.5),
        ]
        .iter()
        .any(|q| q.x * q.x + q.y * q.y + q.z * q.z > 0.12);
        if depth < 5 && error {
            for [a, b, c] in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
                warp(field, Triangle { a, b, c, ..source }, depth + 1, out);
            }
        } else {
            out.push(Triangle {
                a: ma,
                b: mb,
                c: mc,
                ..source
            });
        }
    }
    let original = std::mem::take(triangles);
    triangles.reserve(original.len());
    for triangle in original {
        warp(&field, triangle, 0, triangles);
    }
}
