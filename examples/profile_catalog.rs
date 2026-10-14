//! Reproducible real rendered catalog for visual acceptance and V1 regression.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let root = Path::new(args.get(1).map_or("output/profile-catalog", String::as_str));
    for (name, prompt) in [
        ("wolf", "wolf"),
        ("beetle", "beetle"),
        ("spider", "spider that shoots fire"),
        ("orc", "orc with a sword"),
        ("dragon", "dragon that breathes fire"),
        ("frog", "friendly frog"),
        ("mermaid", "friendly mermaid that sprays acid"),
        ("tree", "friendly tree"),
        ("mammoth", "mammoth"),
        ("serpent", "serpent"),
        ("slime", "slime"),
        ("snail", "snail"),
    ] {
        for (version, profile) in [("v1", GenerationProfile::V1), ("v2", GenerationProfile::V2)] {
            let path = root.join(name).join(version);
            let monster =
                generate_with_profile(prompt, 42, &path, None, PackageFormat::Cbor, profile)?;
            std::fs::write(
                path.join("metadata.json"),
                serde_json::to_vec_pretty(&monster)?,
            )?;
            println!("{}: {}", path.display(), monster.id);
        }
    }
    Ok(())
}
