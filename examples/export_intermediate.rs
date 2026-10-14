//! Export actual baker triangles for Blender inspection. Uses an explicit anatomy seed,
//! independent of package recipe hashes so comparisons isolate geometry changes.
use infernal_diffusion::{anatomy, animation, parser, render3d};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::{error::Error, path::PathBuf};
fn main() -> Result<(), Box<dyn Error>> {
    let mut args: Vec<_> = std::env::args().skip(1).collect();
    let profile = if let Some(index) = args.iter().position(|arg| arg == "--profile") {
        if index + 1 >= args.len() {
            return Err("--profile requires v1 or v2".into());
        }
        let profile: infernal_diffusion::GenerationProfile = args[index + 1].parse()?;
        args.drain(index..=index + 1);
        profile
    } else {
        infernal_diffusion::GenerationProfile::V1
    };
    if args.len() < 3 {
        return Err(
            "usage: export_intermediate PROMPT SEED OUTPUT_DIR [CLIP] [--profile v1|v2]".into(),
        );
    }
    let seed: u64 = args[1].parse()?;
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let mut spec = parser::parse_vocabulary(&args[0]);
    if profile == infernal_diffusion::GenerationProfile::V2 {
        spec = infernal_diffusion::v2::interpret(spec, &args[0]);
        infernal_diffusion::v2::prepare(&mut spec);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut body = anatomy::build(&spec, &mut rng);
    let (_, _, _, poses) = if profile == infernal_diffusion::GenerationProfile::V2 {
        infernal_diffusion::v2::anatomy::refine(&spec, &mut body, seed);
        infernal_diffusion::v2::combat::animate(&spec, &body, seed)
    } else {
        animation::make(&spec, &body)
    };
    let clip = args.get(3).map(String::as_str).unwrap_or("idle");
    for (i, pose) in poses.iter().filter(|p| p.clip_id == clip).enumerate() {
        let data = render3d::debug_scene_json(&spec, &body, pose, seed);
        std::fs::write(
            out.join(format!("{clip}_{i:02}.json")),
            serde_json::to_vec(&data)?,
        )?;
    }
    std::fs::write(out.join("spec.json"), serde_json::to_vec_pretty(&spec)?)?;
    Ok(())
}
