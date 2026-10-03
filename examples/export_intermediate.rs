//! Export actual baker triangles for Blender inspection. Uses an explicit anatomy seed,
//! independent of package recipe hashes so comparisons isolate geometry changes.
use infernal_diffusion::{anatomy, animation, parser, render3d};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::{error::Error, path::PathBuf};
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() < 3 {
        return Err("usage: export_intermediate PROMPT SEED OUTPUT_DIR [CLIP]".into());
    }
    let seed: u64 = args[1].parse()?;
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let spec = parser::parse_vocabulary(&args[0]);
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let body = anatomy::build(&spec, &mut rng);
    let (_, _, _, poses) = animation::make(&spec, &body);
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
