use infernal_diffusion::{generate_hybrid_in_memory, v2::hybrid::HybridRecipe};

fn recipe_value() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"upper_body":"HUMANOID","lower_body":"HORSE",
        "head":"HUMANOID","lower_limb_count":6,"upper_arm_count":2})
}

#[test]
fn hybrid_schema_rejects_unknown_fields_versions_and_invalid_enums() {
    let valid: HybridRecipe = serde_json::from_value(recipe_value()).unwrap();
    valid.validate().unwrap();
    for (field, value) in [
        ("unexpected", serde_json::json!(true)),
        ("head", serde_json::json!("ALIEN")),
    ] {
        let mut bad = recipe_value();
        bad[field] = value;
        assert!(serde_json::from_value::<HybridRecipe>(bad).is_err());
    }
    let mut missing = recipe_value();
    missing.as_object_mut().unwrap().remove("schema_version");
    assert!(serde_json::from_value::<HybridRecipe>(missing).is_err());
    for (field, value) in [
        ("schema_version", 99),
        ("lower_limb_count", 255),
        ("upper_arm_count", 255),
    ] {
        let mut bad = recipe_value();
        bad[field] = serde_json::json!(value);
        let recipe: HybridRecipe = serde_json::from_value(bad).unwrap();
        assert!(recipe.validate().is_err());
        assert!(generate_hybrid_in_memory(&recipe, 42).is_err());
    }
}

#[test]
fn structured_hybrid_generation_preserves_authoritative_json_and_is_deterministic() {
    let recipe: HybridRecipe = serde_json::from_value(recipe_value()).unwrap();
    let a = generate_hybrid_in_memory(&recipe, 42).unwrap();
    let b = generate_hybrid_in_memory(&recipe, 42).unwrap();
    let a = a.packages.last().unwrap();
    let b = b.packages.last().unwrap();
    let descriptor = a
        .monster
        .v2_profile
        .as_ref()
        .unwrap()
        .hybrid
        .as_ref()
        .unwrap();
    assert_eq!(serde_json::to_value(descriptor).unwrap(), recipe_value());

    assert_eq!(a.monster, b.monster);
    assert_eq!(a.sprites, b.sprites);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&a.monster.generation.as_ref().unwrap().prompt)
            .unwrap(),
        recipe_value()
    );
    infernal_diffusion::validate(&a.monster, &a.sprites).unwrap();
    let mut inconsistent = a.monster.clone();
    inconsistent
        .v2_profile
        .as_mut()
        .unwrap()
        .hybrid
        .as_mut()
        .unwrap()
        .lower_limb_count = 5;
    assert!(infernal_diffusion::validate(&inconsistent, &a.sprites).is_err());
    let mut missing = a.monster.clone();
    missing.v2_profile.as_mut().unwrap().hybrid = None;
    assert!(infernal_diffusion::validate(&missing, &a.sprites).is_err());
}

#[test]
fn hybrid_cli_rejects_explicit_v1_and_unknown_schema_without_writing() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("recipe.json");
    let output = directory.path().join("output");
    std::fs::write(&input, recipe_value().to_string()).unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_infernal"))
        .args([
            "--hybrid-json",
            input.to_str().unwrap(),
            "42",
            output.to_str().unwrap(),
            "--profile",
            "v1",
        ])
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("requires V2"));
    assert!(!output.exists());
    let mut bad = recipe_value();
    bad["schema_version"] = serde_json::json!(99);
    std::fs::write(&input, bad.to_string()).unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_infernal"))
        .args([
            "--hybrid-json",
            input.to_str().unwrap(),
            "42",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!run.status.success());
    assert!(!output.exists());
}

#[test]
fn hybrid_c_api_rejects_invalid_json_with_owned_error() {
    use std::ffi::{CStr, CString};
    let json = CString::new(r#"{"schema_version":99}"#).unwrap();
    let mut error = std::ptr::null_mut();
    let object =
        unsafe { infernal_diffusion::infernal_generate_hybrid_json(json.as_ptr(), 42, &mut error) };
    assert!(object.is_null());
    assert!(!error.is_null());
    assert!(!unsafe { CStr::from_ptr(error) }.to_bytes().is_empty());
    unsafe { infernal_diffusion::infernal_free_string(error) };
}

#[test]
fn hybrid_cli_generates_a_valid_package_with_explicit_recipe() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("recipe.json");
    let output = directory.path().join("output");
    std::fs::write(&input, recipe_value().to_string()).unwrap();
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_infernal"))
        .args([
            "--hybrid-json",
            input.to_str().unwrap(),
            "42",
            output.to_str().unwrap(),
            "--profile",
            "v2",
        ])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let monster = infernal_diffusion::load_package(&output).unwrap();
    assert_eq!(
        monster.v2_profile.unwrap().hybrid.unwrap().lower_limb_count,
        6
    );
}
