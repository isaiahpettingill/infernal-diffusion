//! Experimental export-time watermarking. No automatic model downloads or runtime hooks.
//!
//! A public fixed marker is a toy label, not authentication: anyone can reproduce it.
use image::{DynamicImage, RgbaImage};
use std::path::Path;
use trustmark::{Trustmark, Variant, Version};

/// ASCII7 `INFRNL1`, padded to the 61 data bits of TrustMark BCH_5.
pub const PAYLOAD: &str = "1001001100111010001101010010100111010011000110001000000000000";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Trustmark(#[from] trustmark::Error),
    #[error("watermark needs an image at least 256x256 pixels")]
    TooSmall,
    #[error("watermark did not survive PNG quantization; atlas was not changed")]
    RoundtripFailed,
}

/// A model instance is explicitly loaded and may be reused for many exports.
/// Both encoder_Q.onnx and decoder_Q.onnx must already exist in `models`.
pub struct AtlasWatermarker(Trustmark);

impl AtlasWatermarker {
    pub fn new(models: impl AsRef<Path>) -> Result<Self, Error> {
        Ok(Self(Trustmark::new(models, Variant::Q, Version::Bch5)?))
    }

    /// Returns a marked copy, preserving size and alpha exactly. RGB changes are
    /// inherent to invisible watermarking, so keep the original for pixel-perfect art.
    /// Sparse atlases can fail verification and are never reported as marked.
    pub fn encode(&self, original: &RgbaImage) -> Result<RgbaImage, Error> {
        if original.width() < 256 || original.height() < 256 {
            return Err(Error::TooSmall);
        }
        let encoded = self
            .0
            .encode(
                PAYLOAD.to_owned(),
                DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(original.clone()).to_rgb8()),
                0.95,
            )?
            .to_rgba8();
        let mut marked = encoded;
        for (out, input) in marked.pixels_mut().zip(original.pixels()) {
            // Keep raw RGB, including under alpha=0: PNG stores those bytes but
            // Canvas/premultiplication can discard them. The web reader decodes
            // PNG bytes directly, never using a canvas for watermark detection.
            out[3] = input[3];
        }
        if !self.detect(&marked)? {
            return Err(Error::RoundtripFailed);
        }
        Ok(marked)
    }

    /// False means no matching marker decoded, not proof of another origin.
    /// Inference/model errors remain errors rather than being reported as negatives.
    pub fn detect(&self, image: &RgbaImage) -> Result<bool, Error> {
        match self.0.decode(DynamicImage::ImageRgb8(
            DynamicImage::ImageRgba8(image.clone()).to_rgb8(),
        )) {
            Ok(bits) => Ok(bits == PAYLOAD),
            Err(trustmark::Error::CorruptWatermark) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// Mark main color atlases for the root and companions atomically in memory.
    /// Emission and projectile textures are deliberately untouched. Save only after
    /// this succeeds; default generation and existing C/Godot APIs never call it.
    pub fn mark_generated(&self, generated: &mut crate::GeneratedMonster) -> Result<(), Error> {
        let marked: Result<Vec<_>, _> = generated
            .packages
            .iter()
            .map(|package| self.encode(&package.sprites))
            .collect();
        for (package, sprites) in generated.packages.iter_mut().zip(marked?) {
            package.sprites = sprites;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_contract() {
        let expected: String = b"INFRNL1".iter().map(|b| format!("{b:07b}")).collect();
        assert_eq!(PAYLOAD, format!("{expected}{}", "0".repeat(12)));
        assert_eq!(PAYLOAD.len(), 61);
    }
    /// Run explicitly with downloaded Q models, including an actual generated atlas.
    #[test]
    #[ignore = "requires TRUSTMARK_MODELS and native ONNX runtime"]
    fn generated_atlas_roundtrip() {
        let models = std::env::var("TRUSTMARK_MODELS").expect("set TRUSTMARK_MODELS");
        let marker = AtlasWatermarker::new(models).unwrap();
        let mut generated = crate::generate_in_memory_with_profile(
            "giant spider",
            42,
            crate::GenerationProfile::V2,
        )
        .unwrap();
        let originals: Vec<_> = generated
            .packages
            .iter()
            .map(|p| p.sprites.clone())
            .collect();
        for original in &originals {
            assert!(!marker.detect(original).unwrap());
        }
        marker.mark_generated(&mut generated).unwrap();
        for (index, (package, original)) in generated.packages.iter().zip(originals).enumerate() {
            if let Ok(directory) = std::env::var("TRUSTMARK_FIXTURE_DIR") {
                let directory = std::path::Path::new(&directory);
                std::fs::create_dir_all(directory).unwrap();
                crate::atlas_png::save(
                    &original,
                    directory.join(format!("original-{index}.png")),
                    crate::atlas_png::Encoding::Palette,
                )
                .unwrap();
                crate::atlas_png::save(
                    &package.sprites,
                    directory.join(format!("marked-{index}.png")),
                    crate::atlas_png::Encoding::Rgba,
                )
                .unwrap();
            }
            assert_eq!(package.sprites.dimensions(), original.dimensions());
            assert!(package
                .sprites
                .pixels()
                .zip(original.pixels())
                .all(|(a, b)| a[3] == b[3]));
            let mut bytes = std::io::Cursor::new(Vec::new());
            crate::atlas_png::write(
                &package.sprites,
                &mut bytes,
                crate::atlas_png::Encoding::Rgba,
            )
            .unwrap();
            assert_eq!(bytes.get_ref()[25], 6, "marked exports must stay RGBA");
            assert!(marker
                .detect(&image::load_from_memory(bytes.get_ref()).unwrap().to_rgba8())
                .unwrap());
        }
    }
}
