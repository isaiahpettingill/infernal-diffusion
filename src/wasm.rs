//! Browser entry point: only the encoder is removed, not the generation pipeline.
use serde::Serialize;
use wasm_bindgen::prelude::*;
#[allow(unused_imports)] // Retain the exported wasm-bindgen thread-pool initializer.
pub use wasm_bindgen_rayon::init_thread_pool;

#[derive(Serialize)]
struct Atlas {
    width: u32,
    height: u32,
    #[serde(with = "serde_bytes")]
    rgba: Vec<u8>,
}
impl From<image::RgbaImage> for Atlas {
    fn from(image: image::RgbaImage) -> Self {
        Self { width: image.width(), height: image.height(), rgba: image.into_raw() }
    }
}
#[derive(Serialize)]
struct Package {
    path: String,
    monster: crate::proto::Monster,
    sprites: Atlas,
    emission: Atlas,
    projectiles: Option<Atlas>,
}
#[derive(Serialize)]
struct Output { packages: Vec<Package> }

/// Synchronous inside a dedicated Web Worker. The JS client owns cancellation.
/// `seed` is a JS BigInt so the full unsigned 64-bit seed is preserved.
#[wasm_bindgen]
pub fn generate_monster(prompt: &str, seed: u64) -> Result<JsValue, JsValue> {
    let generated = crate::generate_in_memory(prompt, seed)
        .map_err(|error| js_error(&error.to_string()))?;
    let output = Output { packages: generated.packages.into_iter().map(|package| Package {
        path: package.path.to_string_lossy().replace('\\', "/"),
        monster: package.monster,
        sprites: package.sprites.into(),
        emission: package.emission.into(),
        projectiles: package.projectiles.map(Into::into),
    }).collect() };
    output.serialize(&serde_wasm_bindgen::Serializer::new().serialize_large_number_types_as_bigints(true))
        .map_err(|error| js_error(&error.to_string()))
}

fn js_error(message: &str) -> JsValue { JsValue::from_str(message) }
