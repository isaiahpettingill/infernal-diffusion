//! Reproducible real-render encounter evidence, all four directions and three seeds.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(
        args.get(1)
            .map_or("output/encounter-catalog", String::as_str),
    );
    let filter = args.get(2).map(String::as_str).unwrap_or("");
    let seeds: Vec<u64> = args
        .get(3)
        .map(|s| {
            s.split(',')
                .map(|v| v.parse().expect("valid seed"))
                .collect()
        })
        .unwrap_or_else(|| vec![0, 7, 42]);
    for (name, prompt) in [
        ("slime", "splitting slime"),
        ("bush", "bush mimic"),
        ("banana", "large banana mimic"),
        ("vase", "amphora vase mimic"),
        ("sentinel", "bronze shield sentinel"),
        ("pusher", "hunched boulder pusher"),
    ] {
        if !filter.is_empty() && !filter.split(',').any(|f| f == name) {
            continue;
        }
        for &seed in &seeds {
            let path = root.join(format!("{name}_{seed}"));
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
            println!("{}: {}", path.display(), monster.id);
        }
    }
    Ok(())
}
