//! Exercise actual parser entry points in both native BERT and vocabulary/WASM
//! configurations. Explicit counts are scoped to their adjacent body-part noun.
use infernal_diffusion::{
    generate_in_memory_with_profile,
    parser::{self, MonsterSpec},
    v2, GenerationProfile,
};

fn counts(spec: &MonsterSpec) -> (u8, u8, u8) {
    (spec.heads, spec.arm_count, spec.limb_count)
}

fn parse_current(prompt: &str) -> MonsterSpec {
    #[cfg(feature = "bert")]
    let spec = parser::parse_with_embedded_bert(prompt).unwrap();
    #[cfg(not(feature = "bert"))]
    let spec = parser::parse_vocabulary(prompt);
    v2::interpret(spec, prompt)
}

#[test]
fn count_words_digits_and_hyphens_bind_to_their_own_parts() {
    for (prompt, expected) in [
        ("three headed dinosaur with eight arms", (3, 8, 4)),
        ("three-headed dinosaur with eight arms", (3, 8, 4)),
        ("3 headed dinosaur with 8 arms", (3, 8, 4)),
        ("3-headed 8-armed dinosaur", (3, 8, 4)),
        (
            "dinosaur with three heads and eight arms and two tails",
            (3, 8, 4),
        ),
        ("dinosaur with 3 heads, 8 arms and 6 legs", (3, 8, 6)),
        ("dinosaur with eight arms and three heads", (3, 8, 4)),
        ("one-headed three-legged dinosaur with one arm", (1, 1, 3)),
        ("dinosaur with two arms and zero legs", (1, 2, 0)),
        ("dinosaur with no arms and twelve legs", (1, 0, 12)),
        ("dinosaur with seven legs and five heads", (5, 0, 7)),
        // An unrelated number cannot donate its value to a distant body part.
        ("dinosaur with three horns and arms", (1, 0, 4)),
        ("orc with eight swords and two heads", (2, 2, 2)),
        ("orc with three heads and arms", (3, 2, 2)),
    ] {
        let vocabulary = parser::parse_vocabulary(prompt);
        assert_eq!(counts(&vocabulary), expected, "vocabulary: {prompt}");
        let interpreted = parse_current(prompt);
        assert_eq!(counts(&interpreted), expected, "public parser: {prompt}");
    }
}

#[test]
fn unmodified_species_and_generic_multiple_head_defaults_are_unchanged() {
    for (prompt, affinity, expected) in [
        ("dinosaur", "LONGNECK_DINO", (1, 0, 4)),
        ("dino", "LONGNECK_DINO", (1, 0, 4)),
        ("t rex", "TREX", (1, 2, 2)),
        ("velociraptor", "RAPTOR", (1, 2, 2)),
        ("triceratops", "TRICERATOPS", (1, 0, 4)),
        ("orc", "ORC", (1, 2, 2)),
        ("hydra", "HYDRA", (3, 0, 0)),
        ("multi-headed orc", "ORC", (3, 2, 2)),
        ("orc with multiple heads", "ORC", (3, 2, 2)),
    ] {
        let spec = parser::parse_vocabulary(prompt);
        assert_eq!(spec.affinity, affinity, "{prompt}");
        assert_eq!(counts(&spec), expected, "{prompt}");
        assert!(!spec.features.iter().any(|f| f.starts_with("EXPLICIT_")));
    }
}

#[test]
fn explicit_counts_override_species_recipes_and_every_seeded_mutation() {
    for species in [
        "human",
        "ghoul",
        "ghost",
        "frog",
        "lobster",
        "grendel",
        "zombie wolf",
        "mermaid",
        "skeleton mermaid",
        "dinosaur",
        "t rex",
        "velociraptor",
        "brachiosaurus",
        "stegosaurus",
        "triceratops",
        "ankylosaurus",
    ] {
        let prompt = format!("three-headed {species} with 8 arms and four legs");
        let base = parse_current(&prompt);
        for marker in ["EXPLICIT_HEADS", "EXPLICIT_ARMS", "EXPLICIT_LEGS"] {
            assert!(
                base.features.iter().any(|f| f == marker),
                "{prompt}: {marker}"
            );
        }
        for seed in 0..256 {
            let mut spec = base.clone();
            v2::prepare(&mut spec);
            v2::anatomy::prepare_seeded(&mut spec, seed);
            assert_eq!(counts(&spec), (3, 8, 4), "{prompt}, seed {seed}");
        }
    }
    // Tail cardinality has no general representation yet, but its adjacent
    // number must neither contaminate other counts nor allow random extra tails.
    let prompt = "three-headed dinosaur with eight arms and two tails";
    let base = parse_current(prompt);
    assert!(base.features.iter().any(|f| f == "EXPLICIT_TAILS"));
    for seed in 0..256 {
        let mut spec = base.clone();
        v2::prepare(&mut spec);
        v2::anatomy::prepare_seeded(&mut spec, seed);
        assert_eq!(counts(&spec), (3, 8, 4), "seed {seed}");
        assert!(!spec.features.iter().any(|f| f == "MUTATION_EXTRA_TAIL"));
    }
    // Two arms are particularly important: this is the random four-arm branch.
    let prompt = "two-armed orc with 2 legs";
    let base = parse_current(prompt);
    for seed in 0..256 {
        let mut spec = base.clone();
        v2::prepare(&mut spec);
        v2::anatomy::prepare_seeded(&mut spec, seed);
        assert_eq!(counts(&spec), (1, 2, 2), "seed {seed}");
    }
}

#[test]
fn rider_and_mount_counts_stay_on_their_own_clauses() {
    for prompt in [
        "eight-armed orc riding three-headed dinosaur with six legs",
        "three-headed dinosaur with six legs with eight-armed orc rider",
    ] {
        let rider = parse_current(prompt);
        assert_eq!(rider.affinity, "ORC");
        assert_eq!(counts(&rider), (1, 8, 2), "{prompt}");
        assert!(!rider.features.iter().any(|f| f == "EXPLICIT_HEADS"));
        let mount = rider.mount.as_ref().unwrap().spec.as_ref().unwrap();
        assert_eq!(mount.affinity, "LONGNECK_DINO");
        assert_eq!(counts(mount), (3, 0, 6), "{prompt}");
        assert!(!mount.features.iter().any(|f| f == "EXPLICIT_ARMS"));
    }
}

#[test]
fn public_generation_keeps_exact_heads_arms_and_materials() {
    for (prompt, heads, arms, affinity) in [
        ("three headed dinosaur", 3, 0, "LONGNECK_DINO"),
        ("dinosaur with eight arms", 1, 8, "LONGNECK_DINO"),
        (
            "three-headed dinosaur with eight arms",
            3,
            8,
            "LONGNECK_DINO",
        ),
        ("3-headed dinosaur with 8 arms", 3, 8, "LONGNECK_DINO"),
        ("3-headed 8-armed t rex", 3, 8, "TREX"),
        ("skeleton mermaid with eight arms", 1, 8, "MERMAID"),
    ] {
        let result = generate_in_memory_with_profile(prompt, 42, GenerationProfile::V2)
            .unwrap_or_else(|e| panic!("{prompt}: {e}"));
        let root = result.packages.last().unwrap();
        let monster = &root.monster;
        let count = |prefix: &str| {
            monster
                .colliders
                .iter()
                .filter(|c| {
                    c.node_id
                        .strip_prefix(prefix)
                        .is_some_and(|rank| rank.parse::<u8>().is_ok())
                })
                .count()
        };
        assert_eq!(count("head_"), heads, "{prompt}");
        for prefix in ["shoulder_", "arm_", "hand_"] {
            assert_eq!(count(prefix), arms, "{prompt}: {prefix}");
        }
        assert!(monster.tags.iter().any(|t| t == affinity), "{prompt}");
        let source = monster.generation.as_ref().unwrap();
        assert_eq!(source.prompt, prompt);
        assert_eq!(source.seed, 42);
        if cfg!(feature = "bert") {
            assert!(source.parser.starts_with("candle-bert"));
        } else {
            assert_eq!(source.parser, "vocabulary-v1");
        }
        if affinity == "MERMAID" {
            assert!(monster.tags.iter().any(|t| t == "BONE"));
            assert!(monster
                .colliders
                .iter()
                .any(|c| c.node_id.starts_with("v2_fish_tail_")));
            assert_eq!(count("foot_"), 0);
        }
        infernal_diffusion::validate(monster, &root.sprites).unwrap();
    }
}

#[test]
fn impossible_explicit_counts_fail_instead_of_silently_falling_back() {
    for prompt in [
        "dinosaur with six heads",
        "dinosaur with 0 heads",
        "dinosaur with nine arms",
        "dinosaur with thirteen legs",
        "dinosaur with 999999999999999999999 arms",
    ] {
        assert!(
            generate_in_memory_with_profile(prompt, 42, GenerationProfile::V2).is_err(),
            "{prompt}"
        );
    }
}

#[test]
fn hybrid_provenance_uses_the_same_singular_and_numeric_count_intent() {
    for (prompt, arms, legs) in [
        ("centaur with one arm and one leg", 1, 1),
        ("3-headed 8-armed six-legged spider-centaur", 8, 6),
        ("centaur with four arms then two arms", 2, 4),
    ] {
        let spec = parse_current(prompt);
        let recipe = v2::hybrid::from_spec(&spec).unwrap();
        assert_eq!(spec.arm_count, arms, "{prompt}");
        assert_eq!(spec.limb_count, legs, "{prompt}");
        assert_eq!(recipe.upper_arm_count, arms, "{prompt}");
        assert_eq!(recipe.lower_limb_count, legs, "{prompt}");
        recipe.validate().unwrap();
    }
}

#[cfg(feature = "bert")]
#[test]
fn optional_semantic_classifier_cannot_replace_explicit_count_intent() {
    let model = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("models/bert-mini");
    let config = std::fs::read(model.join("config.json")).unwrap();
    let hidden_size = serde_json::from_slice::<serde_json::Value>(&config).unwrap()["hidden_size"]
        .as_u64()
        .unwrap() as usize;
    // Force higher-confidence semantic predictions than the vocabulary defaults,
    // exercising the optional semantic_head.json path rather than only prototypes.
    let head = serde_json::to_vec(&serde_json::json!({
        "labels": ["BODY_PLAN:HUMANOID", "AFFINITY:MERMAID", "FEATURE:FOUR_ARMED"],
        "weights": vec![vec![0.0; hidden_size]; 3],
        "biases": [12.0, 12.0, 12.0],
        "threshold": 0.7
    }))
    .unwrap();
    let prompt = "3-headed dinosaur with eight arms and four legs";
    let custom_model = tempfile::tempdir().unwrap();
    for file in ["config.json", "tokenizer.json", "model.safetensors"] {
        std::fs::copy(model.join(file), custom_model.path().join(file)).unwrap();
    }
    std::fs::write(custom_model.path().join("semantic_head.json"), head).unwrap();
    let parsed = parser::parse_with_bert(prompt, custom_model.path()).unwrap();
    assert_eq!(parsed.parser, "candle-bert-head-v1");
    assert_eq!(parsed.affinity, "MERMAID");
    assert_eq!(counts(&parsed), (3, 8, 4));
    let mut spec = v2::interpret(parsed, prompt);
    v2::prepare(&mut spec);
    v2::anatomy::prepare_seeded(&mut spec, 42);
    assert_eq!(counts(&spec), (3, 8, 4));
    assert!(!spec.features.iter().any(|f| f == "FOUR_ARMED"));
}
