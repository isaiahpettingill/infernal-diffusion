//! Baked projectile art and a bounded mapper from attack semantics to sprite recipes.
use crate::{
    parser::MonsterSpec,
    proto::{Attack, Projectile},
    render,
};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};

const SIZE: u32 = 24;
const FRAMES: u32 = 4;

pub fn recipe_kind(spec: &MonsterSpec, attack: &Attack) -> &'static str {
    match spec.attack.concept.as_str() {
        "BOW" => return "ARROW",
        "SHOTGUN" | "RAILGUN" | "CANNON" => return "BULLET",
        "ROCKET" | "MISSILE" => return "ROCKET",
        "LASER" => return "BEAM",
        "SPORES" => return "SPORE_CLOUD",
        _ => {}
    }
    match attack.element.as_str() {
        "FIRE" => "FIREBALL",
        "POISON" | "VENOM" | "ACID" => "SLUDGE",
        "ICE" => "ICE_SHARD",
        "LIGHTNING" | "PLASMA" => "SPARK",
        "SHADOW" => "SHADOW_ORB",
        _ if spec.is_blob() || spec.is_gastropod() => "SLUDGE",
        _ => "STONE_SHARD",
    }
}

fn paint(image: &mut RgbaImage, x: i32, y: i32, color: [u8; 4]) {
    if x >= 0 && y >= 0 && x < SIZE as i32 && y < SIZE as i32 {
        image.put_pixel(x as u32, y as u32, Rgba(color));
    }
}

fn disc(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
    for y in cy - radius..=cy + radius {
        for x in cx - radius..=cx + radius {
            if (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2) {
                paint(image, x, y, color);
            }
        }
    }
}

fn frame(
    kind: &str,
    phase: u32,
    rng: &mut ChaCha8Rng,
    element: &str,
    affinity: &str,
    spectral: bool,
) -> RgbaImage {
    let mut image = RgbaImage::new(SIZE, SIZE);
    let flicker = phase as i32 % 3;
    match kind {
        "ARROW" => {
            for x in 3..17 {
                for y in 11..13 {
                    paint(&mut image, x, y, [131, 93, 52, 255]);
                }
            }
            for x in 15..22 {
                for y in 8i32..16 {
                    if (y - 12).abs() <= (21 - x) / 2 {
                        paint(&mut image, x, y, [214, 221, 224, 255]);
                    }
                }
            }
            for x in 3..8 {
                paint(&mut image, x, 9, [198, 203, 220, 255]);
                paint(&mut image, x, 14, [198, 203, 220, 255]);
            }
        }
        "BULLET" => {
            for y in 9i32..15 {
                for x in 5..19 {
                    if x < 16 || (y - 12).abs() < 2 {
                        paint(
                            &mut image,
                            x,
                            y,
                            if x < 8 {
                                [111, 79, 41, 255]
                            } else {
                                [220, 220, 198, 255]
                            },
                        );
                    }
                }
            }
            for x in 9..17 {
                paint(&mut image, x, 9, [255, 249, 210, 255]);
            }
        }
        "ROCKET" => {
            for x in 6..18 {
                for y in 9i32..15 {
                    paint(&mut image, x, y, [111, 128, 132, 255]);
                }
            }
            for x in 17..22 {
                for y in 9i32..15 {
                    if (y - 12).abs() <= (21 - x) / 2 {
                        paint(&mut image, x, y, [221, 222, 203, 255]);
                    }
                }
            }
            disc(&mut image, 5 - flicker, 12, 2, [255, 122, 29, 240]);
            paint(&mut image, 2 - flicker, 12, [255, 223, 91, 255]);
        }
        "BEAM" => {
            for x in 2..22 {
                for y in 9..15 {
                    paint(&mut image, x, y, [77, 149, 240, 155]);
                }
            }
            for x in 3..22 {
                for y in 11..13 {
                    paint(&mut image, x, y, [238, 249, 255, 255]);
                }
            }
        }
        "FIREBALL" => {
            let (outer, middle, core) = if spectral {
                (
                    [28, 93, 155, 220],
                    [83, 191, 220, 255],
                    [210, 253, 255, 255],
                )
            } else if affinity == "DRAGON" {
                ([157, 34, 20, 235], [255, 99, 24, 255], [255, 225, 83, 255])
            } else {
                (
                    [188, 53, 21, 230],
                    [250, 138, 28, 255],
                    [255, 244, 129, 255],
                )
            };
            for i in 0..5 {
                disc(
                    &mut image,
                    5 + i * 2 - flicker,
                    11 + rng.random_range(-2..=2),
                    1 + (i / 3),
                    outer,
                );
            }
            disc(&mut image, 14, 12, 6 + (phase % 2) as i32, outer);
            disc(&mut image, 15, 11, 4, middle);
            disc(&mut image, 16, 11, 2, core);
        }
        "SLUDGE" => {
            let (dark, middle, light) = match element {
                "ACID" => ([73, 100, 20, 225], [162, 202, 41, 235], [232, 249, 91, 255]),
                "POISON" => ([70, 42, 91, 225], [131, 72, 159, 235], [196, 151, 223, 255]),
                "VENOM" => ([24, 84, 75, 225], [55, 165, 132, 235], [142, 238, 191, 255]),
                _ => ([30, 91, 63, 225], [70, 163, 91, 235], [160, 232, 112, 255]),
            };
            disc(&mut image, 12, 13, 6, dark);
            for _ in 0..5 {
                let x = 7 + rng.random_range(0..11);
                let y = 7 + rng.random_range(0..11);
                disc(&mut image, x, y, rng.random_range(2..4), middle);
            }
            disc(&mut image, 13, 10, 2, light);
            paint(&mut image, 4 - flicker, 15, middle);
        }
        "SPORE_CLOUD" => {
            for i in 0..11 {
                let x = 4 + rng.random_range(0..16);
                let y = 4 + rng.random_range(0..16);
                let radius = if i % 4 == 0 { 2 } else { 1 };
                disc(
                    &mut image,
                    x + flicker - 1,
                    y,
                    radius + 1,
                    [96, 108, 57, 105],
                );
                disc(&mut image, x + flicker - 1, y, radius, [211, 218, 132, 225]);
            }
            disc(&mut image, 12, 12, 2, [247, 231, 166, 245]);
        }
        "ICE_SHARD" | "STONE_SHARD" => {
            let (edge, core) = if kind == "ICE_SHARD" {
                ([55, 130, 175, 255], [196, 241, 252, 255])
            } else {
                ([71, 68, 64, 255], [175, 164, 131, 255])
            };
            for x in 5..21 {
                for y in 6i32..19 {
                    if (y - 12).abs() <= (x - 5).min(21 - x) / 2 + 1 {
                        paint(&mut image, x, y, edge);
                    }
                }
            }
            for x in 9..19 {
                paint(&mut image, x, 11, core);
            }
        }
        "SPARK" => {
            disc(&mut image, 12, 12, 5, [75, 137, 241, 170]);
            disc(&mut image, 12, 12, 3, [227, 247, 255, 255]);
            for offset in [-8, 8] {
                paint(
                    &mut image,
                    12 + offset,
                    12 + flicker - 1,
                    [153, 203, 255, 220],
                );
                paint(&mut image, 12, 12 + offset, [153, 203, 255, 220]);
            }
        }
        _ => {
            disc(&mut image, 12, 12, 6, [59, 45, 90, 160]);
            disc(&mut image, 12, 12, 4, [127, 73, 164, 220]);
            disc(&mut image, 12, 12, 2, [204, 144, 239, 255]);
        }
    }
    image
}

pub fn bake(
    spec: &MonsterSpec,
    attacks: &mut [Attack],
    seed: u64,
) -> (Vec<Projectile>, Option<RgbaImage>) {
    let ranged: Vec<usize> = attacks
        .iter()
        .enumerate()
        .filter(|(_, attack)| attack.delivery == "PROJECTILE")
        .map(|(index, _)| index)
        .collect();
    if ranged.is_empty() {
        return (Vec::new(), None);
    }
    let mut atlas = RgbaImage::new(SIZE * FRAMES, SIZE * ranged.len() as u32);
    let mut projectiles = Vec::new();
    for (row, index) in ranged.into_iter().enumerate() {
        let attack = &mut attacks[index];
        let kind = recipe_kind(spec, attack);
        let id = format!("projectile_{}", attack.id);
        let mut digest = Sha256::new();
        digest.update(seed.to_le_bytes());
        digest.update(spec.affinity.as_bytes());
        digest.update(attack.id.as_bytes());
        digest.update(kind.as_bytes());
        let seed_bytes: [u8; 32] = digest.finalize().into();
        for phase in 0..FRAMES {
            let mut rng = ChaCha8Rng::from_seed(seed_bytes);
            for _ in 0..phase * 3 {
                let _: u32 = rng.random();
            }
            let sprite = frame(
                kind,
                phase,
                &mut rng,
                &attack.element,
                &spec.affinity,
                spec.materials.iter().any(|material| material == "SPECTRAL"),
            );
            for y in 0..SIZE {
                for x in 0..SIZE {
                    atlas.put_pixel(
                        phase * SIZE + x,
                        row as u32 * SIZE + y,
                        *sprite.get_pixel(x, y),
                    );
                }
            }
        }
        attack.projectile_id = id.clone();
        projectiles.push(Projectile {
            id,
            kind: kind.into(),
            image_file: "projectiles.png".into(),
            frame_width: SIZE,
            frame_height: SIZE,
            columns: FRAMES,
            first_frame: row as u32 * FRAMES,
            frame_count: FRAMES,
            frame_duration_ms: 80,
            collision_radius: if matches!(kind, "ARROW" | "BULLET" | "BEAM") {
                2.5
            } else {
                5.0
            },
            orientation_mode: if matches!(
                kind,
                "ARROW" | "BULLET" | "ROCKET" | "ICE_SHARD" | "STONE_SHARD" | "BEAM"
            ) {
                "ALIGN_VELOCITY"
            } else {
                "BILLBOARD"
            }
            .into(),
        });
    }
    (projectiles, Some(atlas))
}

pub fn valid_atlas(projectiles: &[Projectile], atlas: &RgbaImage) -> bool {
    if projectiles.is_empty() || render::color_count(atlas) >= 256 {
        return false;
    }
    let columns = projectiles[0].columns;
    let width = projectiles[0].frame_width;
    let height = projectiles[0].frame_height;
    if columns == 0
        || width == 0
        || height == 0
        || projectiles
            .iter()
            .any(|p| p.columns != columns || p.frame_width != width || p.frame_height != height)
    {
        return false;
    }
    let Some(total) = projectiles
        .iter()
        .map(|p| p.first_frame.checked_add(p.frame_count))
        .collect::<Option<Vec<_>>>()
        .and_then(|ends| ends.into_iter().max())
    else {
        return false;
    };
    let (Some(atlas_width), Some(atlas_height)) = (
        columns.checked_mul(width),
        total.div_ceil(columns).checked_mul(height),
    ) else {
        return false;
    };
    if atlas.width() != atlas_width || atlas.height() != atlas_height {
        return false;
    }
    projectiles.iter().all(|p| {
        (p.first_frame..p.first_frame + p.frame_count).all(|frame_id| {
            let ox = frame_id % columns * width;
            let oy = frame_id / columns * height;
            (oy..oy + height).any(|y| (ox..ox + width).any(|x| atlas.get_pixel(x, y)[3] > 0))
        })
    })
}
