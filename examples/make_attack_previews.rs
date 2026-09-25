//! Extract alternate frames of one baked attack for quick visual inspection.
//! cargo run --example make_attack_previews -- examples/showcase/combat_dragon swoop

use image::{imageops, Rgba, RgbaImage};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let directory = PathBuf::from(args.next().ok_or("package directory required")?);
    let attack_id = args.next().ok_or("attack ID required")?;
    let monster = infernal_diffusion::load_package(&directory)?;
    let attack = monster
        .attacks
        .iter()
        .find(|attack| attack.id == attack_id)
        .ok_or("attack not found")?;
    let clip = monster
        .animations
        .iter()
        .find(|clip| clip.id == attack.animation_id)
        .ok_or("attack clip not found")?;
    let atlas = image::open(directory.join("sprites.png"))?.into_rgba8();
    let sprites = monster.sprites.ok_or("missing sprite layout")?;
    let frames: Vec<_> = clip.frames.iter().step_by(2).collect();
    let width = sprites.frame_width * 2;
    let height = sprites.frame_height * 2;
    let mut strip =
        RgbaImage::from_pixel(width * frames.len() as u32, height, Rgba([8, 9, 12, 255]));
    for (index, frame) in frames.into_iter().enumerate() {
        let x = frame.sprite_frame_id % sprites.columns * sprites.frame_width;
        let y = frame.sprite_frame_id / sprites.columns * sprites.frame_height;
        let cutout = imageops::crop_imm(&atlas, x, y, sprites.frame_width, sprites.frame_height);
        let enlarged = imageops::resize(
            &cutout.to_image(),
            width,
            height,
            imageops::FilterType::Nearest,
        );
        imageops::overlay(&mut strip, &enlarged, index as i64 * width as i64, 0);
    }
    let output = directory.join("attack_preview.png");
    strip.save(&output)?;
    println!("{}", output.display());
    Ok(())
}
