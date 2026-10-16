use infernal_diffusion::{
    generate_in_memory_compact_with_budget, generate_in_memory_compact_with_profile,
    generate_in_memory_with_profile, Error, GenerationBudget, GenerationProfile,
};

#[test]
fn runaway_chain_fails_before_parsing_and_c_abi_returns_error() {
    let prompt = "orc riding an orc riding an orc riding a mammoth riding a mammoth";
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        assert!(matches!(
            generate_in_memory_compact_with_profile(prompt, 42, profile),
            Err(Error::ResourceLimit {
                resource: "mount depth",
                requested: 4,
                limit: 3
            })
        ));
    }
    let prompt = std::ffi::CString::new(prompt).unwrap();
    let mut error = std::ptr::null_mut();
    unsafe {
        let result = infernal_diffusion::infernal_generate_in_memory_profile_compact(
            prompt.as_ptr(),
            42,
            2,
            &mut error,
        );
        assert!(result.is_null());
        assert!(!error.is_null());
        let message = std::ffi::CStr::from_ptr(error).to_str().unwrap();
        assert!(message.contains("generation budget exceeded: mount depth"));
        assert!(!message.contains("panic"));
        infernal_diffusion::infernal_free_string(error);
    }
}

#[test]
fn explicit_limits_reject_without_reducing_output() {
    for (limits, resource) in [
        (
            GenerationBudget {
                max_prompt_bytes: 2,
                ..Default::default()
            },
            "prompt bytes",
        ),
        (
            GenerationBudget {
                max_arms_per_creature: 1,
                ..Default::default()
            },
            "arms per creature",
        ),
        (
            GenerationBudget {
                max_body_nodes: 1,
                ..Default::default()
            },
            "anatomy nodes",
        ),
        (
            GenerationBudget {
                max_poses: 1,
                ..Default::default()
            },
            "animation poses",
        ),
        (
            GenerationBudget {
                max_dense_pixels: 1,
                ..Default::default()
            },
            "dense atlas pixels",
        ),
        (
            GenerationBudget {
                max_estimated_working_bytes: 1,
                ..Default::default()
            },
            "estimated working bytes",
        ),
    ] {
        let result = generate_in_memory_compact_with_budget(
            "orc with a sword",
            42,
            GenerationProfile::V2,
            limits,
        );
        assert!(
            matches!(result, Err(Error::ResourceLimit { resource: r, .. }) if r == resource),
            "expected {resource}"
        );
    }
}

#[test]
fn guarded_normal_and_companions_preserve_all_attacks_frames_and_pixels() {
    for (prompt, count) in [
        ("orc with a sword that shoots fire", 1),
        ("goblin riding a wolf; mount survives", 3),
        ("spider that summons spiders", 2),
    ] {
        let mut legacy =
            generate_in_memory_with_profile(prompt, 42, GenerationProfile::V2).unwrap();
        infernal_diffusion::atlas::compact(&mut legacy).unwrap();
        let guarded =
            generate_in_memory_compact_with_profile(prompt, 42, GenerationProfile::V2).unwrap();
        assert_eq!(guarded.packages.len(), count, "{prompt}");
        for (actual, expected) in guarded.packages.iter().zip(&legacy.packages) {
            assert_eq!(actual.path, expected.path);
            assert_eq!(
                actual.monster, expected.monster,
                "all attacks, specials, frames, events, companions and metadata"
            );
            assert_eq!(actual.sprites, expected.sprites, "all sprite pixels");
            assert_eq!(actual.emission, expected.emission, "all emission pixels");
            assert_eq!(
                actual.projectiles, expected.projectiles,
                "all projectile pixels"
            );
        }
    }
}

#[test]
fn generous_defaults_admit_large_and_nested_ordinary_creatures() {
    for (prompt, packages) in [
        ("mammoth", 1),
        (
            "anime girl with axe riding a two headed ghost pig riding a slime pig",
            5,
        ),
    ] {
        let output = generate_in_memory_compact_with_profile(prompt, 42, GenerationProfile::V2)
            .unwrap_or_else(|error| panic!("{prompt}: {error}"));
        assert_eq!(output.packages.len(), packages);
        for package in output.packages {
            infernal_diffusion::validate(&package.monster, &package.sprites).unwrap();
        }
    }
}
