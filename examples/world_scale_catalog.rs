//! Actual baked V1/V2 evidence at player-relative display scale.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(
        args.get(1)
            .map_or("output/world-scale-catalog", String::as_str),
    );
    for (name, prompt) in [
        ("beetle", "beetle"),
        ("humanoid", "humanoid"),
        ("wolf", "wolf"),
        ("elephant", "elephant"),
        ("fish", "fish"),
        ("bat", "bat"),
        ("giant_elephant", "giant elephant"),
        ("mini_elephant", "mini elephant"),
        ("cute_elephant", "cute elephant"),
        ("chibi_bat", "chibi bat"),
        ("giant_slime", "giant splitting slime"),
        ("mini_trex", "mini trex"),
    ] {
        for (version, profile) in [("v1", GenerationProfile::V1), ("v2", GenerationProfile::V2)] {
            let seeds: &[u64] = if name == "giant_elephant" {
                &[0, 7, 42]
            } else {
                &[7]
            };
            for &seed in seeds {
                let path = root.join(version).join(format!("{name}_{seed}"));
                let monster =
                    generate_with_profile(prompt, seed, &path, None, PackageFormat::Cbor, profile)?;
                std::fs::write(
                    path.join("metadata.json"),
                    serde_json::to_vec_pretty(&monster)?,
                )?;
                println!(
                    "{}: {:.1}px target",
                    path.display(),
                    monster.size.as_ref().unwrap().target_height_px
                );
            }
        }
    }
    Ok(())
}
