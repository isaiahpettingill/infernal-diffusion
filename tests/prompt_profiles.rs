use infernal_diffusion::{
    generate_in_memory_with_profile,
    prompt::{random_prompt_with_profile, PromptProfile},
    GenerationProfile,
};

#[test]
fn public_profiles_generate_real_v1_v2_packages_and_boss_gameplay() {
    for profile in PromptProfile::ALL {
        let prompt = random_prompt_with_profile(42, 1, profile).unwrap();
        for generation in [GenerationProfile::V1, GenerationProfile::V2] {
            let generated = generate_in_memory_with_profile(&prompt, 42, generation)
                .unwrap_or_else(|error| panic!("{profile:?} {generation:?}: {prompt}: {error}"));
            let root = &generated.packages.last().unwrap().monster;
            assert_eq!(
                root.v2_profile.is_some(),
                generation == GenerationProfile::V2
            );
            if profile == PromptProfile::Boss {
                let gameplay = root.gameplay.as_ref().unwrap();
                // These bonuses exclude size: a tall silhouette alone is not a boss.
                assert!(gameplay.health_armor_bonus >= 31, "{prompt}: {gameplay:?}");
                assert!(gameplay.health_magic_bonus >= 12, "{prompt}: {gameplay:?}");
                assert!(gameplay.defense >= 7, "{prompt}: {gameplay:?}");
                assert!(
                    root.attacks
                        .iter()
                        .any(|a| a.delivery == "PROJECTILE" && a.element != "PHYSICAL"),
                    "{prompt}"
                );
            }
            if profile == PromptProfile::Ocean {
                assert!(
                    root.movement_modes
                        .iter()
                        .any(|m| m.id == "SWIM" && m.category == "SWIMMING"),
                    "{prompt}"
                );
            }
        }
    }
}

#[test]
fn additive_c_api_replays_default_and_rejects_invalid_values() {
    use infernal_diffusion::{
        infernal_free_string, infernal_random_prompt, infernal_random_prompt_profile,
    };
    use std::{ffi::CStr, ptr};
    unsafe {
        for profile in PromptProfile::ALL {
            let mut error = ptr::null_mut();
            let p = infernal_random_prompt_profile(u64::MAX, 2, profile as u32, &mut error);
            assert!(!p.is_null() && error.is_null());
            assert_eq!(
                CStr::from_ptr(p).to_str().unwrap(),
                random_prompt_with_profile(u64::MAX, 2, profile).unwrap()
            );
            infernal_free_string(p);
        }
        let p = infernal_random_prompt(42, 2, ptr::null_mut());
        assert_eq!(
            CStr::from_ptr(p).to_str().unwrap(),
            random_prompt_with_profile(42, 2, PromptProfile::Default).unwrap()
        );
        infernal_free_string(p);
        for (difficulty, profile) in [(0, 0), (4, 0), (257, 0), (2, 6), (2, u32::MAX)] {
            let mut error = ptr::null_mut();
            assert!(infernal_random_prompt_profile(42, difficulty, profile, &mut error).is_null());
            assert!(!error.is_null());
            infernal_free_string(error);
            assert!(
                infernal_random_prompt_profile(42, difficulty, profile, ptr::null_mut()).is_null()
            );
        }
    }
}

#[cfg(feature = "bert")]
#[test]
fn embedded_parser_preserves_profile_constraints() {
    use infernal_diffusion::{parser::parse_with_embedded_bert, v2};
    for profile in PromptProfile::ALL {
        for seed in 0..24 {
            let prompt = random_prompt_with_profile(seed, (seed % 3 + 1) as u8, profile).unwrap();
            let base = parse_with_embedded_bert(&prompt).unwrap();
            for spec in [base.clone(), v2::interpret(base, &prompt)] {
                assert_ne!(spec.affinity, "UNKNOWN", "{prompt}");
                match profile {
                    PromptProfile::Boss => {
                        assert!(spec.bulk >= 0.8, "{prompt}");
                        assert_eq!(spec.attack.delivery, "PROJECTILE", "{prompt}");
                        assert_ne!(spec.attack.element, "PHYSICAL", "{prompt}");
                    }
                    PromptProfile::Necromancer => assert!(
                        v2::damage_traits::describe(&spec)
                            .iter()
                            .any(|t| ["SKELETAL", "UNDEAD", "SPECTRAL"].contains(&t.as_str())),
                        "{prompt}"
                    ),
                    PromptProfile::Ocean => assert!(
                        [
                            "FISH",
                            "EEL",
                            "SHARK",
                            "WHALE",
                            "ORCA",
                            "JELLYFISH",
                            "CEPHALOPOD",
                            "KRAKEN",
                            "SEA_SERPENT",
                            "SEA_DRAGON",
                            "MERMAID",
                            "MERMAN",
                            "FISHMAN"
                        ]
                        .contains(&spec.affinity.as_str()),
                        "{prompt}"
                    ),
                    _ => {}
                }
            }
        }
    }
}
