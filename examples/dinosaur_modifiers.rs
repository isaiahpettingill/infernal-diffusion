//! Actual renderer evidence for dinosaur recipes and exact anatomy modifiers.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(
        args.get(1)
            .map_or("output/dinosaur-modifiers", String::as_str),
    );
    let filter = args.get(2).map(String::as_str).unwrap_or("");
    for (name, prompt) in [
        ("dinosaur", "dinosaur"),
        ("trex", "t rex"),
        ("raptor", "velociraptor"),
        ("longneck", "brachiosaurus"),
        ("stego", "stegosaurus"),
        ("triceratops", "triceratops"),
        ("ankylosaur", "ankylosaurus"),
        ("three_heads", "three headed dinosaur"),
        ("eight_arms", "dinosaur with eight arms"),
        ("combined", "three-headed dinosaur with eight arms"),
        ("skeleton_mermaid", "skeleton mermaid with eight arms"),
        ("numeric", "3-headed dinosaur with 8 arms"),
    ] {
        if !filter.is_empty() && !filter.split(',').any(|f| f == name) {
            continue;
        }
        for seed in [0, 7, 42] {
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
