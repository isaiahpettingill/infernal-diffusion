//! Lossless palette PNG storage. Runtime/Godot/WASM atlas buffers remain RGBA8.
use image::{ImageEncoder, ImageError, ImageResult, RgbaImage};
use std::{collections::HashMap, io::Write, path::Path};

/// Palette storage never changes pixels: images with more than 256 RGBA colors
/// fall back to RGBA. Watermarked exports explicitly select `Rgba` instead.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    #[default]
    Palette,
    Rgba,
}

struct Palette {
    colors: Vec<[u8; 4]>,
    indices: HashMap<u32, u8>,
}

fn palette(image: &RgbaImage) -> Option<Palette> {
    let mut colors = Vec::new();
    let mut indices = HashMap::new();
    let mut previous = None;
    for pixel in image.pixels() {
        let key = u32::from_le_bytes(pixel.0);
        if previous == Some(key) {
            continue;
        }
        previous = Some(key);
        if let std::collections::hash_map::Entry::Vacant(entry) = indices.entry(key) {
            if colors.len() == 256 {
                return None;
            }
            entry.insert(colors.len() as u8);
            colors.push(pixel.0);
        }
    }
    Some(Palette { colors, indices })
}

fn png_error(error: png::EncodingError) -> ImageError {
    ImageError::IoError(error.into())
}

/// Stream rows instead of allocating another full atlas of palette indices.
/// RGB under alpha=0 and every partial-alpha value survive exactly.
pub fn write<W: Write>(image: &RgbaImage, output: W, encoding: Encoding) -> ImageResult<()> {
    let palette = (encoding == Encoding::Palette)
        .then(|| palette(image))
        .flatten();
    let Some(Palette { colors, indices }) = palette else {
        return image::codecs::png::PngEncoder::new(output).write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        );
    };
    let mut encoder = png::Encoder::new(output, image.width(), image.height());
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Balanced);
    encoder.set_filter(png::Filter::NoFilter);
    encoder.set_palette(
        colors
            .iter()
            .flat_map(|c| c[..3].iter().copied())
            .collect::<Vec<_>>(),
    );
    encoder.set_trns(colors.iter().map(|c| c[3]).collect::<Vec<_>>());
    let mut writer = encoder.write_header().map_err(png_error)?;
    {
        let mut stream = writer.stream_writer().map_err(png_error)?;
        let mut row = vec![0; image.width() as usize];
        let mut previous = None;
        let mut index = 0;
        for pixels in image.as_raw().chunks_exact(image.width() as usize * 4) {
            for (pixel, destination) in pixels.as_chunks::<4>().0.iter().zip(&mut row) {
                let key = u32::from_le_bytes(*pixel);
                if previous != Some(key) {
                    index = indices[&key];
                    previous = Some(key);
                }
                *destination = index;
            }
            stream.write_all(&row)?;
        }
        stream.finish().map_err(png_error)?;
    }
    writer.finish().map_err(png_error)
}

pub fn save(image: &RgbaImage, path: impl AsRef<Path>, encoding: Encoding) -> ImageResult<()> {
    let file = std::fs::File::create(path)?;
    write(image, std::io::BufWriter::new(file), encoding)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn roundtrip(image: &RgbaImage, encoding: Encoding, color_type: u8) {
        let mut bytes = Vec::new();
        write(image, &mut bytes, encoding).unwrap();
        assert_eq!(bytes[25], color_type);
        assert_eq!(
            image::load_from_memory(&bytes).unwrap().into_rgba8(),
            *image
        );
        let mut again = Vec::new();
        write(image, &mut again, encoding).unwrap();
        assert_eq!(bytes, again);
    }
    #[test]
    fn palette_keeps_hidden_rgb_and_exact_alpha() {
        let image = RgbaImage::from_raw(
            3,
            2,
            vec![
                0, 0, 0, 0, 20, 30, 40, 0, 20, 30, 40, 64, 20, 30, 40, 128, 255, 0, 1, 192, 255, 0,
                1, 255,
            ],
        )
        .unwrap();
        roundtrip(&image, Encoding::Palette, 3);
        roundtrip(&image, Encoding::Rgba, 6);
    }
    #[test]
    fn palette_limit_is_lossless_and_rgba_can_be_forced() {
        let image = RgbaImage::from_fn(256, 1, |x, _| image::Rgba([x as u8, 0, 0, 255]));
        roundtrip(&image, Encoding::Palette, 3);
        roundtrip(&image, Encoding::Rgba, 6);
        let image = RgbaImage::from_fn(257, 1, |x, _| {
            image::Rgba([x as u8, (x / 256) as u8, 0, 255])
        });
        roundtrip(&image, Encoding::Palette, 6);
    }
    #[test]
    fn package_export_defaults_to_palette_but_can_explicitly_bypass_it() {
        let image = RgbaImage::from_raw(2, 1, vec![3, 4, 5, 0, 30, 40, 50, 128]).unwrap();
        let generated = crate::GeneratedMonster {
            packages: vec![crate::GeneratedPackage {
                path: ".".into(),
                monster: crate::proto::Monster::default(),
                sprites: image.clone(),
                emission: RgbaImage::new(2, 1),
                projectiles: None,
            }],
        };
        let root = tempfile::tempdir().unwrap();
        crate::save_generated(&generated, &root.path().join("ordinary")).unwrap();
        crate::save_generated_with_format_and_encoding(
            &generated,
            &root.path().join("unquantized"),
            crate::PackageFormat::Protobuf,
            Encoding::Rgba,
        )
        .unwrap();
        for (directory, color_type) in [("ordinary", 3), ("unquantized", 6)] {
            let bytes = std::fs::read(root.path().join(directory).join("sprites.png")).unwrap();
            assert_eq!(bytes[25], color_type);
            assert_eq!(image::load_from_memory(&bytes).unwrap().into_rgba8(), image);
        }
    }

    #[test]
    fn single_color_and_empty_image_are_handled() {
        roundtrip(&RgbaImage::new(4, 4), Encoding::Palette, 3);
        assert!(write(&RgbaImage::new(0, 0), Vec::new(), Encoding::Palette).is_err());
    }
}
