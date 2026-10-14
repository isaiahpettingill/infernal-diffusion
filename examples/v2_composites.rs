//! Real-render inspection specimens for explicit composition and spectral rules.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "output/v2-composites".into());
    for (name, prompt, seed) in [
        ("six_legged_centaur", "six legged centaur", 0),
        ("spider_centaur", "spider centaur", 0),
        ("fish_head_humanoid", "fish head on humanoid legs", 0),
        ("trooper_mermaid", "stormtrooper top on a mermaid bottom", 0),
        ("walking_tree", "walking tree", 0),
        ("shade_humanoid", "shade humanoid", 0),
        ("shade_wolf", "shade wolf", 0),
        ("shade_tree", "shade tree", 0),
        ("shade_mermaid", "shade mermaid", 0),
        ("ghost", "ghost", 0),
        ("ghoul", "ghoul", 0),
        ("skeleton_0", "skeleton", 0),
        ("skeleton_7", "skeleton", 7),
        ("skeleton_42", "skeleton", 42),
    ] {
        let path = Path::new(&root).join(name);
        let monster = generate_with_profile(
            prompt,
            seed,
            &path,
            None,
            PackageFormat::Cbor,
            GenerationProfile::V2,
        )?;
        std::fs::write(
            path.join("metadata.json"),
            serde_json::to_vec_pretty(&monster)?,
        )?;
        println!(
            "{name}: seed {seed}; {} frames",
            monster.sprites.as_ref().unwrap().frame_count
        );
    }
    Ok(())
}
