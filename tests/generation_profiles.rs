use infernal_diffusion::{
    generate_in_memory, generate_in_memory_with_profile, v2, GenerationProfile,
};
#[test]
fn absent_profile_is_exact_v1_and_invalid_profiles_are_rejected() {
    let legacy = generate_in_memory("wolf", 42).unwrap();
    let explicit = generate_in_memory_with_profile("wolf", 42, GenerationProfile::V1).unwrap();
    let a = legacy.packages.last().unwrap();
    let b = explicit.packages.last().unwrap();
    assert_eq!(a.monster, b.monster);
    assert_eq!(a.sprites, b.sprites);
    assert!(a.monster.v2_profile.is_none());
    assert!(serde_json::to_value(&a.monster)
        .unwrap()
        .get("v2_profile")
        .is_none());
    assert!("v3".parse::<GenerationProfile>().is_err());
    assert!(GenerationProfile::try_from(0).is_err());
}
#[test]
fn v2_has_deterministic_versioned_output_and_real_lure_clips() {
    let a = generate_in_memory_with_profile(
        "friendly frog that sprays acid",
        42,
        GenerationProfile::V2,
    )
    .unwrap();
    let b = generate_in_memory_with_profile(
        "friendly frog that sprays acid",
        42,
        GenerationProfile::V2,
    )
    .unwrap();
    let a = a.packages.last().unwrap();
    let b = b.packages.last().unwrap();
    assert_eq!(a.monster, b.monster);
    assert_eq!(a.sprites, b.sprites);
    let profile = a.monster.v2_profile.as_ref().unwrap();
    assert_eq!(profile.profile_version, 1);
    let lure = profile.lure.as_ref().unwrap();
    assert!(lure.harmless);
    assert!(a
        .monster
        .animations
        .iter()
        .any(|c| c.id == lure.lure_animation_id));
    assert!(a
        .monster
        .animations
        .iter()
        .any(|c| c.id == lure.reveal_animation_id));
    assert!(a.monster.attacks.iter().any(|a| a.element == "ACID"));
    let mut bad = a.monster.clone();
    bad.v2_profile
        .as_mut()
        .unwrap()
        .lure
        .as_mut()
        .unwrap()
        .attack_id = "missing".into();
    assert!(v2::validate_profile(&bad).is_err());
}
#[test]
fn v2_never_accidentally_opts_in_from_prompt_text() {
    let a = generate_in_memory("friendly frog profile v2", 1).unwrap();
    assert!(a.packages.last().unwrap().monster.v2_profile.is_none());
}

/// Captured from untouched main 6f0c9b31 with the vocabulary-only parser.
/// Aquatic outputs intentionally change under motion.aquatic_navigation@1.
#[cfg(not(feature = "bert"))]
#[test]
fn terrestrial_v1_matches_pre_v2_golden_bytes() {
    use prost::Message;
    use sha2::{Digest, Sha256};
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("v1_terrestrial_goldens.json")).unwrap();
    for row in rows.as_array().unwrap() {
        let prompt = row["prompt"].as_str().unwrap();
        let generated = generate_in_memory(prompt, row["seed"].as_u64().unwrap()).unwrap();
        let root = generated.packages.last().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(root.sprites.as_raw())),
            row["rgba_sha256"].as_str().unwrap(),
            "{prompt} pixels"
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(root.monster.encode_to_vec())),
            row["protobuf_sha256"].as_str().unwrap(),
            "{prompt} metadata"
        );
    }
}
