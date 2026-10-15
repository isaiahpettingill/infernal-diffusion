//! Actual renderer evidence: species, deterministic ordinary/mutant seeds and all baked states.
use infernal_diffusion::{generate_with_profile, GenerationProfile, PackageFormat};
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let root = Path::new(args.get(1).map_or("output/species-catalog", String::as_str));
    let filter = args.get(2).map(String::as_str).unwrap_or("");
    for (name, prompt) in [
        ("orc", "orc"),
        ("troll", "troll"),
        ("wolf", "wolf"),
        ("lion", "lion"),
        ("bear", "bear"),
        ("horse", "horse"),
        ("boar", "boar"),
        ("rabbit", "rabbit"),
        ("crab", "crab"),
        ("lobster", "lobster"),
        ("hydra", "hydra"),
        ("cyclops", "cyclops"),
        ("minotaur", "minotaur"),
        ("cerberus", "cerberus"),
        ("historical", "adolph hitler"),
        ("anime", "anime girl"),
        ("crocodile", "crocodile"),
        ("eagle", "eagle"),
        ("elephant", "elephant"),
        ("mammoth", "mammoth"),
        ("dragon", "dragon"),
        ("trex", "t rex"),
        ("stego", "stegosaurus"),
        ("triceratops", "triceratops"),
        ("snake", "snake"),
        ("gorgon", "gorgon"),
        ("worm", "worm"),
        ("sandworm", "sand worm"),
        ("grendel", "grendel"),
        ("potato", "potato monster"),
        ("reaper", "reaper"),
        ("cacodaemon", "cacodaemon"),
        ("devil", "devil"),
        ("alien", "alien"),
        ("robot", "robot"),
        ("cute_elephant", "cute elephant"),
        ("chibi_wolf", "chibi wolf"),
        ("zombie_wolf", "zombie wolf"),
        ("round", "sphere"),
        ("ninja", "ninja wolf"),
        ("fire_orc", "flaming orc with sword"),
        ("water_bear", "water bear"),
        ("rhino", "rhino"),
        ("tiger", "tiger"),
        ("goat", "goat"),
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
