//! Opt-in lossless runtime atlas storage. Logical frames keep their original
//! dimensions, coordinates and IDs; only their backing texture is repacked.
use crate::{
    proto::{FrameRegion, SpriteSet},
    Error, GeneratedMonster, GeneratedPackage,
};
use image::RgbaImage;
use std::collections::HashMap;

pub fn valid_regions(s: &SpriteSet, width: u32, height: u32) -> bool {
    s.frame_regions.len() == s.frame_count as usize
        && s.frame_regions.iter().all(|r| {
            r.width > 0
                && r.height > 0
                && r.x.checked_add(r.width).is_some_and(|v| v <= width)
                && r.y.checked_add(r.height).is_some_and(|v| v <= height)
                && r.offset_x
                    .checked_add(r.width)
                    .is_some_and(|v| v <= s.frame_width)
                && r.offset_y
                    .checked_add(r.height)
                    .is_some_and(|v| v <= s.frame_height)
        })
}

/// Compact all root/companion textures. Legacy generators remain unchanged.
/// Repeated calls are harmless. No animation, physics or visual pixels change.
pub fn compact(generated: &mut GeneratedMonster) -> Result<(), Error> {
    for package in &mut generated.packages {
        compact_package(package)?;
    }
    Ok(())
}

pub(crate) fn compact_package(p: &mut GeneratedPackage) -> Result<(), Error> {
    crate::validate(&p.monster, &p.sprites)?;
    let s = p.monster.sprites.as_ref().expect("validated sprites");
    if !s.frame_regions.is_empty() {
        return Ok(());
    }
    // The packed format uses the modern four-view coordinate contract only.
    if p.monster.format_version != 7 {
        return Ok(());
    }
    let mut unique: Vec<RgbaImage> = Vec::new();
    let mut lookup: HashMap<(u32, u32, Vec<u8>), usize> = HashMap::new();
    let mut frames = Vec::with_capacity(s.frame_count as usize);
    for id in 0..s.frame_count {
        let sx = id % s.columns * s.frame_width;
        let sy = id / s.columns * s.frame_height;
        let mut bounds = (s.frame_width, s.frame_height, 0, 0);
        for y in 0..s.frame_height {
            for x in 0..s.frame_width {
                // Preserve even invisible RGB bytes for exact reconstruction tests.
                if p.sprites.get_pixel(sx + x, sy + y).0 != [0; 4] {
                    bounds.0 = bounds.0.min(x);
                    bounds.1 = bounds.1.min(y);
                    bounds.2 = bounds.2.max(x + 1);
                    bounds.3 = bounds.3.max(y + 1);
                }
            }
        }
        if bounds.2 == 0 {
            bounds = (0, 0, 1, 1);
        }
        let image = image::imageops::crop_imm(
            &p.sprites,
            sx + bounds.0,
            sy + bounds.1,
            bounds.2 - bounds.0,
            bounds.3 - bounds.1,
        )
        .to_image();
        let key = (image.width(), image.height(), image.as_raw().clone());
        let index = *lookup.entry(key).or_insert_with(|| {
            let index = unique.len();
            unique.push(image);
            index
        });
        frames.push((index, bounds.0, bounds.1));
    }
    drop(lookup);
    // Deterministic height-first shelves, with a transparent pixel gutter to
    // prevent neighboring frames bleeding under linear filtering. Try several
    // widths rather than rounding a sparse atlas up to a power of two.
    let mut order: Vec<usize> = (0..unique.len()).collect();
    order.sort_by_key(|&i| {
        (
            std::cmp::Reverse(unique[i].height()),
            std::cmp::Reverse(unique[i].width()),
            i,
        )
    });
    let area: u64 = unique
        .iter()
        .map(|i| u64::from(i.width() + 2) * u64::from(i.height() + 2))
        .sum();
    let minimum = unique.iter().map(|i| i.width() + 2).max().unwrap_or(1);
    let square = (area as f64).sqrt().ceil() as u32;
    let mut best = None;
    for numerator in [8, 10, 12, 14, 16, 20, 24, 32] {
        let width = (square * numerator / 16).max(minimum);
        let (mut x, mut y, mut row, mut used) = (0, 0, 0, 0);
        let mut locations = vec![(0, 0); unique.len()];
        for &i in &order {
            let w = unique[i].width() + 2;
            let h = unique[i].height() + 2;
            if x + w > width {
                y += row;
                x = 0;
                row = 0;
            }
            locations[i] = (x + 1, y + 1);
            x += w;
            row = row.max(h);
            used = used.max(x);
        }
        let height = y + row;
        if best.as_ref().is_none_or(|(w, h, _)| {
            u64::from(used) * u64::from(height) < u64::from(*w) * u64::from(*h)
        }) {
            best = Some((used, height, locations));
        }
    }
    let (width, height, locations) = best.expect("packing candidates");
    let mut sprites = RgbaImage::new(width, height);
    for (i, image) in unique.iter().enumerate() {
        image::imageops::replace(
            &mut sprites,
            image,
            i64::from(locations[i].0),
            i64::from(locations[i].1),
        );
    }
    let regions = frames
        .into_iter()
        .map(|(i, offset_x, offset_y)| FrameRegion {
            x: locations[i].0,
            y: locations[i].1,
            width: unique[i].width(),
            height: unique[i].height(),
            offset_x,
            offset_y,
        })
        .collect();
    let emission = crate::render::emission_sheet(&sprites);
    let empty = !emission.pixels().any(|pixel| pixel[3] > 0);
    p.emission = if empty {
        RgbaImage::new(1, 1)
    } else {
        emission
    };
    p.sprites = sprites;
    p.monster.format_version = 8;
    let s = p.monster.sprites.as_mut().expect("validated sprites");
    s.frame_regions = regions;
    s.palette_size = crate::render::color_count(&p.sprites) as u32;
    if empty {
        s.emission_file.clear();
    }
    crate::validate(&p.monster, &p.sprites)
}
