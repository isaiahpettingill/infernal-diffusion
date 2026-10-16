//! Public generation-path checks, run with embedded BERT and vocabulary/WASM features.
#[test]
fn named_dinosaurs_keep_their_species_with_explicit_modifiers() {
    for (name, affinity) in [
        ("t rex", "TREX"),
        ("velociraptor", "RAPTOR"),
        ("brachiosaurus", "LONGNECK_DINO"),
        ("stegosaurus", "STEGO"),
        ("triceratops", "TRICERATOPS"),
        ("ankylosaurus", "ANKYLOSAUR"),
    ] {
        let prompt = format!("three-headed {name} with eight arms");
        let parsed = infernal_diffusion::parser::parse_vocabulary(&prompt);
        let mut spec = infernal_diffusion::v2::interpret(parsed, &prompt);
        infernal_diffusion::v2::prepare(&mut spec);
        for seed in [0, 7, 42, 123] {
            let mut seeded = spec.clone();
            infernal_diffusion::v2::anatomy::prepare_seeded(&mut seeded, seed);
            assert_eq!(seeded.affinity, affinity, "{prompt}");
            assert_eq!(
                (seeded.heads, seeded.arm_count),
                (3, 8),
                "{prompt}, seed {seed}"
            );
        }
    }
}
