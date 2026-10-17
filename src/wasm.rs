//! Full browser generation with the bundled Candle BERT encoder.
use serde::Serialize;
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm-threads")]
#[allow(unused_imports)] // Retain the exported wasm-bindgen thread-pool initializer.
pub use wasm_bindgen_rayon::init_thread_pool;

thread_local! {
    static PROGRESS: std::cell::RefCell<Option<js_sys::Function>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn report_progress(phase: &str, completed: usize, total: usize) {
    PROGRESS.with(|slot| {
        if let Some(callback) = slot.borrow().as_ref() {
            let _ = callback.call3(
                &JsValue::NULL,
                &phase.into(),
                &(completed as f64).into(),
                &(total as f64).into(),
            );
        }
    });
}

struct ProgressGuard;
impl Drop for ProgressGuard {
    fn drop(&mut self) {
        PROGRESS.with(|slot| *slot.borrow_mut() = None);
    }
}
fn with_progress(
    callback: js_sys::Function,
    generate: impl FnOnce() -> Result<JsValue, JsValue>,
) -> Result<JsValue, JsValue> {
    PROGRESS.with(|slot| *slot.borrow_mut() = Some(callback));
    let _guard = ProgressGuard;
    let result = generate();
    if result.is_ok() {
        report_progress("complete", 1, 1);
    }
    result
}

/// Loads the bundled model once per worker and performs a real encoder forward pass.
/// The thread-local cache is reused by all subsequent generations.
#[wasm_bindgen]
pub fn initialize_model() -> Result<String, JsValue> {
    #[cfg(feature = "bert")]
    {
        crate::parser::parse_with_embedded_bert("slime")
            .map(|spec| spec.parser)
            .map_err(|error| js_error(&error.to_string()))
    }
    #[cfg(not(feature = "bert"))]
    Err(js_error(
        "This WASM build has no bundled BERT model; rebuild with --features wasm,bert",
    ))
}

#[wasm_bindgen]
pub fn generator_info() -> Result<JsValue, JsValue> {
    serde_json::json!({
        "model": "google/bert_uncased_L-2_H-128_A-2",
        "parser": "candle-bert-prototypes-v2",
        "bundledModel": cfg!(feature = "bert"),
        "threaded": cfg!(feature = "wasm-threads"),
        "modelSha256": "7fb69ad9f6866d8983183c930e33828f326470bf6ad8bbb2ad4ed957a92e9414"
    })
    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
    .map_err(|error| js_error(&error.to_string()))
}

#[wasm_bindgen]
pub fn generate_monster_with_profile_progress(
    prompt: &str,
    seed: u64,
    profile: &str,
    callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    with_progress(callback, || {
        generate_monster_with_profile(prompt, seed, profile)
    })
}

#[wasm_bindgen]
pub fn generate_hybrid_json_progress(
    recipe_json: &str,
    seed: u64,
    callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    with_progress(callback, || generate_hybrid_json(recipe_json, seed))
}

#[derive(Serialize)]
struct Atlas {
    width: u32,
    height: u32,
    #[serde(with = "serde_bytes")]
    rgba: Vec<u8>,
}
impl From<image::RgbaImage> for Atlas {
    fn from(image: image::RgbaImage) -> Self {
        Self {
            width: image.width(),
            height: image.height(),
            rgba: image.into_raw(),
        }
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
struct Output {
    packages: Vec<Package>,
}

/// Synchronous inside a dedicated Web Worker. The JS client owns cancellation.
/// `seed` is a JS BigInt so the full unsigned 64-bit seed is preserved.
#[wasm_bindgen]
pub fn generate_monster(prompt: &str, seed: u64) -> Result<JsValue, JsValue> {
    generate_monster_with_profile(prompt, seed, "v1")
}

/// Explicit opt-in profile. Only "v1" and "v2" are accepted.
#[wasm_bindgen]
pub fn generate_monster_with_profile(
    prompt: &str,
    seed: u64,
    profile: &str,
) -> Result<JsValue, JsValue> {
    let profile = profile
        .parse::<crate::GenerationProfile>()
        .map_err(|error| js_error(&error.to_string()))?;
    let generated = crate::generate_in_memory_with_profile(prompt, seed, profile)
        .map_err(|error| js_error(&error.to_string()))?;
    serialize_generated(generated)
}

/// Structured hybrid JSON bypasses natural-language parsing and always uses V2.
#[wasm_bindgen]
pub fn generate_hybrid_json(recipe_json: &str, seed: u64) -> Result<JsValue, JsValue> {
    let recipe: crate::v2::hybrid::HybridRecipe =
        serde_json::from_str(recipe_json).map_err(|error| js_error(&error.to_string()))?;
    let generated = crate::generate_hybrid_in_memory(&recipe, seed)
        .map_err(|error| js_error(&error.to_string()))?;
    serialize_generated(generated)
}

/// Opt-in packed textures; logical sprite coordinates remain unchanged.
#[wasm_bindgen]
pub fn generate_monster_compact_progress(
    prompt: &str,
    seed: u64,
    profile: &str,
    callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    with_progress(callback, || {
        let profile = profile
            .parse::<crate::GenerationProfile>()
            .map_err(|e| js_error(&e.to_string()))?;
        let generated = crate::generate_in_memory_compact_with_profile(prompt, seed, profile)
            .map_err(|e| js_error(&e.to_string()))?;
        serialize_generated(generated)
    })
}

#[wasm_bindgen]
pub fn generate_hybrid_compact_progress(
    recipe_json: &str,
    seed: u64,
    callback: js_sys::Function,
) -> Result<JsValue, JsValue> {
    with_progress(callback, || {
        let recipe = serde_json::from_str(recipe_json).map_err(|e| js_error(&e.to_string()))?;
        let generated = crate::generate_hybrid_in_memory_compact(&recipe, seed)
            .map_err(|e| js_error(&e.to_string()))?;
        serialize_generated(generated)
    })
}

/// Suggest editable monster text using the shared Rust summoner vocabulary.
/// Prompt profiles are independent of the V1/V2 monster generation profile.
/// A floating-point argument avoids wasm-bindgen's silent narrowing to u8.
#[wasm_bindgen]
pub fn suggest_prompt(seed: u64, difficulty: f64, prompt_profile: &str) -> Result<String, JsValue> {
    if !difficulty.is_finite() || difficulty.fract() != 0.0 || !(1.0..=3.0).contains(&difficulty) {
        return Err(js_error("difficulty must be 1, 2, or 3"));
    }
    let profile = prompt_profile
        .parse::<crate::prompt::PromptProfile>()
        .map_err(|error| js_error(&error.to_string()))?;
    crate::prompt::random_prompt_with_profile(seed, difficulty as u8, profile)
        .map_err(|error| js_error(&error.to_string()))
}

fn serialize_generated(generated: crate::GeneratedMonster) -> Result<JsValue, JsValue> {
    report_progress("packaging", 0, 1);
    let output = Output {
        packages: generated
            .packages
            .into_iter()
            .map(|package| Package {
                path: package.path.to_string_lossy().replace('\\', "/"),
                monster: package.monster,
                sprites: package.sprites.into(),
                emission: package.emission.into(),
                projectiles: package.projectiles.map(Into::into),
            })
            .collect(),
    };
    output
        .serialize(
            &serde_wasm_bindgen::Serializer::new().serialize_large_number_types_as_bigints(true),
        )
        .map_err(|error| js_error(&error.to_string()))
}

fn js_error(message: &str) -> JsValue {
    JsValue::from_str(message)
}
