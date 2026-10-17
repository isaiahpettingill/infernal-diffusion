//! Reproducible in-memory generation timing and complete package digests.
//!
//! cargo run --release --example bench_generation -- OUTPUT_DIR [RUNS] [FIXTURE]
//!
//! Set INFERNAL_PROFILE=1 for renderer phase timings and reused frame counts.
//! Compare saved JSON Lines from two revisions; disregard run zero for warmed
//! timings and run without concurrent builds. Metadata JSON is saved separately
//! for review; protobuf and raw RGBA SHA-256 digests detect any output change.
use infernal_diffusion::{
    generate_hybrid_in_memory, generate_in_memory_with_profile, GenerationProfile,
};
use prost::Message;
use sha2::{Digest, Sha256};
use std::{path::PathBuf, time::Instant};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let out = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("output/bench-generation"),
    );
    std::fs::create_dir_all(&out)?;
    let runs: usize = args.get(2).map(|s| s.parse()).transpose()?.unwrap_or(3);
    if runs == 0 {
        return Err("run count must be positive".into());
    }
    let fixture_filter = args.get(3).map(String::as_str).unwrap_or("");
    for (name, prompt) in [
        ("wolf", "wolf"),
        ("slime", "small ice slime"),
        ("orc", "orc with a sword"),
        ("mounted", "giant orc with a huge axe riding a giant spider"),
        ("shadow", "shadow wolf"),
        ("shade", "shade humanoid"),
        ("hybrid", ""),
    ] {
        if !name.contains(fixture_filter) {
            continue;
        }
        for (version, profile) in [("v1", GenerationProfile::V1), ("v2", GenerationProfile::V2)] {
            if name == "hybrid" && version == "v1" {
                continue;
            }
            for run in 0..runs {
                eprintln!("BENCH {name} {version} {run}");
                let started = Instant::now();
                let generated = if name == "hybrid" {
                    let recipe = serde_json::from_str(
                        r#"{"schema_version":1,"upper_body":"STORMTROOPER","lower_body":"SPIDER","head":"FISH","lower_limb_count":8,"upper_arm_count":4}"#,
                    )?;
                    generate_hybrid_in_memory(&recipe, 42)?
                } else {
                    generate_in_memory_with_profile(prompt, 42, profile)?
                };
                let elapsed = started.elapsed().as_secs_f64();
                let mut packages = Vec::new();
                for (index, pkg) in generated.packages.iter().enumerate() {
                    let mut hash = Sha256::new();
                    hash.update(pkg.sprites.as_raw());
                    let sprites = format!("{:x}", hash.finalize());
                    let mut hash = Sha256::new();
                    hash.update(pkg.emission.as_raw());
                    let emission = format!("{:x}", hash.finalize());
                    let projectile = pkg
                        .projectiles
                        .as_ref()
                        .map(|p| format!("{:x}", Sha256::digest(p.as_raw())));
                    // Value serialization orders map keys, so metadata comparison is deterministic.
                    let metadata = serde_json::to_vec(&serde_json::to_value(&pkg.monster)?)?;
                    let meta = format!("{:x}", Sha256::digest(&metadata));
                    let proto = format!("{:x}", Sha256::digest(pkg.monster.encode_to_vec()));
                    if run == 0 {
                        std::fs::write(
                            out.join(format!("{name}_{version}_{index}.json")),
                            metadata,
                        )?;
                    }
                    packages.push(serde_json::json!({"index":index,"width":pkg.sprites.width(),"height":pkg.sprites.height(),"sprites":sprites,"emission":emission,"projectile":projectile,"metadata":meta,"protobuf":proto}));
                }
                println!(
                    "{}",
                    serde_json::json!({"name":name,"version":version,"run":run,"seconds":elapsed,"packages":packages})
                );
            }
        }
    }
    Ok(())
}
