use crate::{
    anatomy::{Body, Node},
    animation::{node_offset, Pose},
};
use image::{Rgba, RgbaImage};
use std::collections::BTreeSet;

pub(crate) const MAX_BASE_COLORS: usize = 48;

pub(crate) fn add_palette_color(palette: &mut Vec<[u8; 4]>, color: [u8; 4]) {
    if palette.len() < MAX_BASE_COLORS && !palette.contains(&color) {
        palette.push(color);
    }
}

pub(crate) fn color_count(image: &RgbaImage) -> usize {
    image
        .pixels()
        .map(|pixel| pixel.0)
        .collect::<BTreeSet<_>>()
        .len()
}

pub(crate) fn quantized_alpha(alpha: u8) -> u8 {
    match alpha {
        0..=31 => 0,
        32..=95 => 64,
        96..=159 => 128,
        160..=223 => 192,
        _ => 255,
    }
}

pub(crate) fn colors(material: &str) -> ([u8; 4], [u8; 4], [u8; 4]) {
    match material {
        "FLESH" => ([75, 51, 44, 255], [155, 107, 82, 255], [223, 170, 125, 255]),
        "FLESH_ASH" => (
            [58, 63, 60, 255],
            [119, 131, 121, 255],
            [186, 194, 164, 255],
        ),
        "FLESH_OCHRE" => ([82, 56, 30, 255], [174, 119, 55, 255], [231, 183, 97, 255]),
        "FLESH_RUST" => ([82, 36, 35, 255], [163, 72, 58, 255], [223, 134, 91, 255]),
        "SKIN_PALE" => (
            [101, 71, 62, 255],
            [199, 153, 128, 255],
            [244, 210, 176, 255],
        ),
        "EYE_DARK" => ([20, 23, 34, 255], [34, 42, 56, 255], [72, 87, 103, 255]),
        "HAIR_DARK" => ([20, 24, 34, 255], [43, 49, 65, 255], [86, 91, 104, 255]),
        "CLOTH_IVORY" => (
            [67, 72, 75, 255],
            [158, 166, 158, 255],
            [226, 221, 192, 255],
        ),
        "CLOTH_NAVY" => ([22, 34, 58, 255], [49, 70, 103, 255], [110, 133, 155, 255]),
        "STOCKING_DARK" => ([21, 26, 37, 255], [45, 51, 66, 255], [83, 89, 105, 255]),
        "LEATHER_BLACK" => ([23, 22, 23, 255], [53, 46, 45, 255], [101, 85, 69, 255]),
        "UNIFORM_BROWN" => ([47, 42, 37, 255], [97, 82, 61, 255], [157, 133, 92, 255]),
        "BONE" => (
            [91, 79, 70, 255],
            [188, 178, 144, 255],
            [239, 230, 188, 255],
        ),
        "CHITIN" => ([45, 31, 54, 255], [114, 67, 99, 255], [186, 133, 157, 255]),
        "CHITIN_UMBER" => ([43, 32, 25, 255], [112, 74, 44, 255], [195, 140, 76, 255]),
        "CHITIN_JADE" => ([22, 53, 43, 255], [52, 120, 88, 255], [142, 190, 124, 255]),
        "CHITIN_BLUE" => ([29, 40, 65, 255], [58, 91, 133, 255], [139, 165, 190, 255]),
        "SCALES" => ([29, 66, 49, 255], [65, 134, 83, 255], [149, 190, 106, 255]),
        "SCALES_RUST" => ([74, 34, 31, 255], [158, 69, 50, 255], [223, 135, 71, 255]),
        "SCALES_BLUE" => ([28, 48, 71, 255], [58, 112, 139, 255], [135, 179, 188, 255]),
        "SCALES_OCHRE" => ([70, 55, 27, 255], [149, 121, 52, 255], [212, 188, 96, 255]),
        "SAND_HIDE" => ([82, 62, 48, 255], [166, 121, 73, 255], [219, 183, 112, 255]),
        "SHARK_HIDE" => ([27, 49, 67, 255], [68, 104, 120, 255], [158, 184, 181, 255]),
        "ORCA_HIDE" => ([16, 21, 27, 255], [47, 60, 68, 255], [212, 223, 213, 255]),
        "ORCA_WHITE" => (
            [88, 109, 118, 255],
            [183, 202, 205, 255],
            [237, 240, 228, 255],
        ),
        "STORM" => ([39, 54, 68, 220], [85, 111, 126, 230], [177, 197, 190, 235]),
        "FUR" => ([69, 42, 36, 255], [145, 92, 61, 255], [207, 158, 91, 255]),
        "FUR_GOLD" => ([91, 52, 31, 255], [192, 127, 57, 255], [244, 194, 96, 255]),
        "FUR_WHITE" => (
            [76, 91, 99, 255],
            [182, 201, 203, 255],
            [244, 246, 232, 255],
        ),
        "FUR_DARK" => ([31, 36, 43, 255], [72, 81, 85, 255], [137, 148, 143, 255]),
        "FUR_BROWN" => ([55, 38, 31, 255], [117, 78, 53, 255], [189, 142, 94, 255]),
        "ROTTEN" => ([54, 61, 43, 255], [104, 125, 69, 255], [177, 170, 105, 255]),
        "FUNGUS" => (
            [68, 47, 79, 255],
            [149, 102, 144, 255],
            [228, 181, 190, 255],
        ),
        "ALIEN_SKIN" => ([40, 70, 78, 255], [79, 150, 142, 255], [156, 223, 176, 255]),
        "MAKEUP" => (
            [87, 87, 102, 255],
            [211, 204, 194, 255],
            [250, 242, 221, 255],
        ),
        "RED_CLOTH" => ([77, 28, 52, 255], [178, 54, 71, 255], [239, 125, 111, 255]),
        "FEATHER" => (
            [47, 49, 65, 255],
            [106, 109, 141, 255],
            [180, 180, 196, 255],
        ),
        "FEATHER_BLACK" => ([21, 29, 38, 255], [51, 66, 81, 255], [113, 132, 147, 255]),
        "FEATHER_BROWN" => ([62, 45, 36, 255], [126, 92, 61, 255], [203, 165, 109, 255]),
        "FEATHER_WHITE" => (
            [68, 79, 83, 255],
            [177, 190, 184, 255],
            [245, 241, 218, 255],
        ),
        "FEATHER_GOLD" => ([46, 80, 59, 255], [87, 173, 101, 255], [222, 211, 80, 255]),
        "LEAF" => ([21, 51, 33, 255], [58, 109, 54, 255], [135, 157, 71, 255]),
        "GUNGAN_SKIN" => ([88, 70, 55, 255], [159, 129, 95, 255], [212, 181, 132, 255]),
        "DEEP_SEA" => ([17, 43, 57, 255], [35, 105, 111, 255], [94, 175, 168, 255]),
        "METAL" => (
            [43, 53, 65, 255],
            [116, 135, 149, 255],
            [215, 231, 234, 255],
        ),
        "STONE" => (
            [52, 56, 61, 255],
            [112, 116, 107, 255],
            [181, 181, 157, 255],
        ),
        "WRAPPING" => (
            [66, 57, 48, 255],
            [157, 139, 105, 255],
            [217, 200, 154, 255],
        ),
        "SPECTRAL" => (
            [35, 87, 110, 100],
            [73, 177, 190, 155],
            [165, 239, 228, 190],
        ),
        "MEMBRANE" => ([78, 46, 85, 130], [151, 85, 153, 165], [219, 150, 183, 175]),
        "SLIME" => ([32, 80, 60, 210], [65, 156, 99, 220], [142, 228, 144, 230]),
        "SLIME_AMBER" => ([90, 57, 23, 210], [178, 113, 41, 220], [237, 198, 91, 230]),
        "SLIME_BLUE" => ([28, 63, 93, 210], [58, 139, 173, 220], [139, 220, 228, 230]),
        "TUMOR" => ([79, 36, 62, 255], [153, 68, 94, 255], [214, 135, 136, 255]),
        "HORN" => ([65, 56, 49, 255], [142, 120, 89, 255], [220, 194, 139, 255]),
        "ENERGY" => (
            [55, 109, 175, 255],
            [112, 199, 247, 255],
            [242, 249, 255, 255],
        ),
        "FIRE" => (
            [139, 39, 25, 220],
            [240, 108, 31, 245],
            [255, 230, 103, 255],
        ),
        "SMOKE" => ([61, 61, 76, 70], [112, 109, 124, 110], [175, 170, 183, 145]),
        "WOOD" => ([58, 43, 35, 255], [124, 88, 55, 255], [192, 148, 90, 255]),
        _ => ([63, 35, 47, 255], [146, 72, 82, 255], [217, 139, 119, 255]),
    }
}
fn palette(body: &Body) -> Vec<[u8; 4]> {
    let mut palette_colors = Vec::new();
    for material in body.nodes.iter().map(|n| n.material.as_str()) {
        let (a, b, c) = colors(material);
        add_palette_color(&mut palette_colors, a);
        add_palette_color(&mut palette_colors, b);
        add_palette_color(&mut palette_colors, c);
    }
    palette_colors
}
pub(crate) fn quantize(img: &mut RgbaImage, palette: &[[u8; 4]]) {
    for pixel in img.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        let best = palette
            .iter()
            .min_by_key(|candidate| {
                let dr = pixel[0] as i32 - candidate[0] as i32;
                let dg = pixel[1] as i32 - candidate[1] as i32;
                let db = pixel[2] as i32 - candidate[2] as i32;
                2 * dr * dr + 4 * dg * dg + db * db
            })
            .expect("material palette is nonempty");
        for channel in 0..3 {
            pixel[channel] = best[channel];
        }
        // Alpha is quantized separately so spectral tissue survives palette reduction.
        pixel[3] = quantized_alpha(pixel[3]);
    }
}
fn blend(dst: &mut Rgba<u8>, src: [u8; 4]) {
    let a = src[3] as f32 / 255.0;
    let da = dst[3] as f32 / 255.0;
    let out_a = a + da * (1.0 - a);
    if out_a <= 0.0 {
        return;
    }
    for k in 0..3 {
        dst[k] = ((src[k] as f32 * a + dst[k] as f32 * da * (1.0 - a)) / out_a).round() as u8;
    }
    dst[3] = (out_a * 255.0).round() as u8;
}
pub(crate) fn transform(node: &Node, index: usize, pose: &Pose, gravity: f32) -> (f32, f32, f32) {
    if let Some(&(x, y, a)) = pose.physical_positions.get(index) {
        let (dx, dy, da) = if pose.state.contains("ATTACK") {
            node_offset(&node.kind, &node.id, index, pose, gravity)
        } else {
            (0.0, 0.0, 0.0)
        };
        return (x + dx, y + dy, a + da);
    }
    let (dx, dy, da) = node_offset(&node.kind, &node.id, index, pose, gravity);
    let x = node.x + dx;
    let y = node.y + dy;
    let ca = pose.root_angle.cos();
    let sa = pose.root_angle.sin();
    (
        x * ca - y * sa + pose.root_x,
        x * sa + y * ca + pose.root_y,
        node.angle + da + pose.root_angle,
    )
}
fn ellipse(img: &mut RgbaImage, node: &Node, index: usize, pose: &Pose, gravity: f32, factor: f32) {
    let (x, y, a) = transform(node, index, pose, gravity);
    let cx = img.width() as f32 / 2.0 + x * factor;
    let cy = img.height() as f32 * 0.57 + y * factor;
    let rx = node.rx.max(0.4) * factor;
    let ry = node.ry.max(0.4) * factor;
    let ca = a.cos();
    let sa = a.sin();
    let bound = rx.max(ry) + 2.0;
    let xmin = (cx - bound).max(0.0) as u32;
    let xmax = (cx + bound).min(img.width() as f32 - 1.0) as u32;
    let ymin = (cy - bound).max(0.0) as u32;
    let ymax = (cy + bound).min(img.height() as f32 - 1.0) as u32;
    let palette = colors(&node.material);
    for py in ymin..=ymax {
        for px in xmin..=xmax {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let u = (dx * ca + dy * sa) / rx;
            let v = (-dx * sa + dy * ca) / ry;
            let r = u * u + v * v;
            let inside = match node.kind.as_str() {
                "HORN" | "ANTLER" | "CLAW" | "STINGER" => {
                    (-1.0..=1.0).contains(&v) && u.abs() <= (v + 1.0) * 0.5
                }
                "FANG" | "TUSK" => (-1.0..=1.0).contains(&v) && u.abs() <= (1.0 - v) * 0.5,
                "WEAPON" => u.abs() <= 1.0 && v.abs() <= 0.75,
                "WING" => (-1.0..=1.0).contains(&u) && v.abs() <= (1.0 - u) * 0.5,
                _ => r <= 1.0,
            };
            if inside {
                let pattern = match node.material.as_str() {
                    material if material.starts_with("CHITIN") => {
                        ((u * 6.0).floor() as i32).rem_euclid(3) == 0
                    }
                    material if material.starts_with("SCALES") => {
                        (((u * 10.0).floor() as i32 + (v * 8.0).floor() as i32).rem_euclid(4)) == 0
                    }
                    "FUR" | "FEATHER" => ((u * 13.0 + v * 4.0).floor() as i32).rem_euclid(5) == 0,
                    "BONE" => ((u * 11.0 + v * 2.0).floor() as i32).rem_euclid(9) == 0,
                    "SPECTRAL" => {
                        ((u * 12.0).floor() as i32 + (v * 12.0).floor() as i32).rem_euclid(3) == 0
                    }
                    _ => false,
                };
                let shade = if r > 0.82 {
                    palette.0
                } else if u + v < -0.25 || pattern {
                    palette.2
                } else {
                    palette.1
                };
                blend(img.get_pixel_mut(px, py), shade);
            }
        }
    }
}
fn connector(
    img: &mut RgbaImage,
    a: (f32, f32),
    b: (f32, f32),
    color: [u8; 4],
    radius: f32,
    factor: f32,
) {
    let ax = img.width() as f32 / 2.0 + a.0 * factor;
    let ay = img.height() as f32 * 0.57 + a.1 * factor;
    let bx = img.width() as f32 / 2.0 + b.0 * factor;
    let by = img.height() as f32 * 0.57 + b.1 * factor;
    let r = radius * factor;
    let xmin = (ax.min(bx) - r).max(0.0) as u32;
    let xmax = (ax.max(bx) + r).min(img.width() as f32 - 1.0) as u32;
    let ymin = (ay.min(by) - r).max(0.0) as u32;
    let ymax = (ay.max(by) + r).min(img.height() as f32 - 1.0) as u32;
    let vx = bx - ax;
    let vy = by - ay;
    let len = (vx * vx + vy * vy).max(1e-6);
    for py in ymin..=ymax {
        for px in xmin..=xmax {
            let dx = px as f32 - ax;
            let dy = py as f32 - ay;
            let t = ((dx * vx + dy * vy) / len).clamp(0.0, 1.0);
            let dist = ((dx - t * vx).powi(2) + (dy - t * vy).powi(2)).sqrt();
            if dist <= r {
                blend(img.get_pixel_mut(px, py), color);
            }
        }
    }
}
fn frame(body: &Body, pose: &Pose, size: u32) -> RgbaImage {
    let visual_scale = if size >= 96 { 1.25 } else { 1.0 };
    let factor = 4.0 * visual_scale;
    let mut hi = RgbaImage::new(size * 4, size * 4);
    // Connective tissue is drawn beneath parts to unify the silhouette.
    for (i, n) in body.nodes.iter().enumerate() {
        if let Some(parent) = n.parent {
            let p = &body.nodes[parent];
            let (x, y, _) = transform(n, i, pose, body.gravity);
            let (px, py, _) = transform(p, parent, pose, body.gravity);
            connector(
                &mut hi,
                (x, y),
                (px, py),
                colors(&n.material).0,
                n.rx.min(p.rx) * 0.72,
                factor,
            );
        }
    }
    for (i, n) in body.nodes.iter().enumerate().filter(|(_, n)| !n.feature) {
        ellipse(&mut hi, n, i, pose, body.gravity, factor);
    }
    for (i, n) in body.nodes.iter().enumerate().filter(|(_, n)| n.feature) {
        ellipse(&mut hi, n, i, pose, body.gravity, factor);
    }
    let mut low = RgbaImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
            let mut sums = [0u32; 4];
            let mut count = 0u32;
            for yy in 0..4 {
                for xx in 0..4 {
                    let p = hi.get_pixel(x * 4 + xx, y * 4 + yy);
                    if p[3] > 0 {
                        for k in 0..4 {
                            sums[k] += p[k] as u32;
                        }
                        count += 1;
                    }
                }
            }
            if count >= 3 {
                let alpha = (sums[3] / count) as u8;
                low.put_pixel(
                    x,
                    y,
                    Rgba([
                        (sums[0] / count) as u8,
                        (sums[1] / count) as u8,
                        (sums[2] / count) as u8,
                        alpha,
                    ]),
                );
            }
        }
    }
    // A one-pixel feature marker survives reduction for eyes and attack origins.
    for (i, n) in body.nodes.iter().enumerate().filter(|(_, n)| n.feature) {
        let (nx, ny, _) = transform(n, i, pose, body.gravity);
        let x = (size as f32 / 2.0 + nx * visual_scale).round() as i32;
        let y = (size as f32 * 0.57 + ny * visual_scale).round() as i32;
        if x >= 0 && y >= 0 && x < size as i32 && y < size as i32 {
            low.put_pixel(x as u32, y as u32, Rgba(colors(&n.material).2));
        }
    }
    if pose.state == "DEATH" {
        for p in low.pixels_mut() {
            p[3] = quantized_alpha(((p[3] as f32) * (1.0 - pose.phase * 0.22)) as u8);
        }
    }
    low
}
pub fn sheet(body: &Body, poses: &[Pose], size: u32) -> (RgbaImage, u32, u32) {
    let columns = 8;
    let rows = (poses.len() as u32).div_ceil(columns);
    let palette = palette(body);
    let mut frames = Vec::with_capacity(poses.len());
    for pose in poses {
        let mut image = frame(body, pose, size);
        quantize(&mut image, &palette);
        frames.push(image);
    }
    for i in 1..frames.len().saturating_sub(1) {
        if poses[i - 1].clip_id != poses[i].clip_id || poses[i + 1].clip_id != poses[i].clip_id {
            continue;
        }
        let previous = frames[i - 1].clone();
        let next = frames[i + 1].clone();
        for y in 0..size {
            for x in 0..size {
                let a = previous.get_pixel(x, y);
                let b = next.get_pixel(x, y);
                let current = frames[i].get_pixel_mut(x, y);
                if a == b && a[3] > 0 && a[3] <= 128 && current[3] < 128 {
                    *current = *a;
                }
            }
        }
    }
    let mut sheet = RgbaImage::new(size * columns, size * rows);
    for (i, image) in frames.iter().enumerate() {
        let sx = i as u32 % columns * size;
        let sy = i as u32 / columns * size;
        for y in 0..size {
            for x in 0..size {
                sheet.put_pixel(sx + x, sy + y, *image.get_pixel(x, y));
            }
        }
    }
    let color_count = color_count(&sheet) as u32;
    (sheet, columns, color_count)
}

pub fn emission_sheet(sheet: &RgbaImage) -> RgbaImage {
    let emissive = ["ENERGY", "FIRE"]
        .into_iter()
        .flat_map(|m| {
            let (_, mid, high) = colors(m);
            [mid, high]
        })
        .collect::<Vec<_>>();
    let mut emission = RgbaImage::new(sheet.width(), sheet.height());
    for (x, y, p) in sheet.enumerate_pixels() {
        if p[3] == 0 {
            continue;
        }
        if emissive
            .iter()
            .any(|c| p[0] == c[0] && p[1] == c[1] && p[2] == c[2])
        {
            let intensity = p[0].max(p[1]).max(p[2]);
            emission.put_pixel(x, y, Rgba([intensity, intensity, intensity, p[3]]));
        }
    }
    emission
}

#[cfg(test)]
mod palette_tests {
    use super::colors;

    #[test]
    fn common_body_materials_have_distinct_palettes() {
        assert_ne!(colors("FLESH"), colors("unknown"));
        assert_ne!(colors("CHITIN"), colors("CHITIN_UMBER"));
        assert_ne!(colors("SCALES"), colors("SCALES_BLUE"));
        assert_ne!(colors("SHARK_HIDE"), colors("STORM"));
    }
}
