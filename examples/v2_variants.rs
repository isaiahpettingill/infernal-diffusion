//! Reproducible gameplay/novelty specimens with complete baked metadata.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "output/v2-variants".into());
    for (name, prompt, seed) in [
        ("mermaid_ordinary", "friendly mermaid", 0),
        ("mermaid_extra_tail", "friendly mermaid", 10),
        ("mermaid_four_arms", "friendly mermaid", 36),
        ("mermaid_both", "friendly mermaid", 42),
        ("harpy_mutant", "harpy", 61),
        ("tree_ordinary", "friendly tree", 0),
        ("tree_mutant", "friendly tree", 1),
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
            "{name}: seed {seed}; {:?}",
            monster.v2_profile.as_ref().unwrap().modifiers
        );
    }
    Ok(())
}
