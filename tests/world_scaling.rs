use infernal_diffusion::{
    anatomy, generate_from_spec_with_profile, parser, proto, render3d, v2,
    world_scale::{self, SizeModifier},
    GenerationProfile, PackageFormat,
};
use prost::Message;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn spec(prompt: &str, profile: GenerationProfile) -> parser::MonsterSpec {
    let spec = parser::parse_vocabulary(prompt);
    if profile == GenerationProfile::V2 {
        v2::interpret(spec, prompt)
    } else {
        spec
    }
}
fn body(spec: &mut parser::MonsterSpec, profile: GenerationProfile, seed: u64) -> anatomy::Body {
    if profile == GenerationProfile::V2 {
        v2::prepare(spec);
        v2::anatomy::prepare_seeded(spec, seed);
    }
    let mut body = anatomy::build(spec, &mut ChaCha8Rng::seed_from_u64(seed));
    if profile == GenerationProfile::V2 {
        v2::anatomy::refine(spec, &mut body, seed);
        v2::grip::prepare_body(spec, &mut body);
    }
    body
}
#[test]
fn modifiers_are_universal_seeded_and_survive_recipe_selection_in_both_profiles() {
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        for name in [
            "beetle",
            "wolf",
            "humanoid",
            "elephant",
            "trex",
            "fish",
            "bat",
            "splitting slime",
            "bronze shield sentinel",
            "boulder pusher",
        ] {
            let normal = world_scale::target_height(&spec(name, profile), 42);
            for seed in [0, 1, 7, 42, u64::MAX] {
                for (word, low, high) in [
                    ("giant", 1.65, 2.5),
                    ("mini", 0.25, 0.45),
                    ("tiny", 0.25, 0.45),
                    ("small", 0.55, 0.8),
                ] {
                    let s = spec(&format!("{word} {name}"), profile);
                    let height = world_scale::target_height(&s, seed);
                    assert!(
                        height >= normal * low - 0.001 && height <= normal * high + 0.001,
                        "{profile:?} {word} {name}: {height}/{normal}"
                    );
                    assert_eq!(height, world_scale::target_height(&s, seed));
                }
            }
            assert_ne!(
                world_scale::target_height(&spec(&format!("giant {name}"), profile), 1),
                world_scale::target_height(&spec(&format!("giant {name}"), profile), 2)
            );
        }
    }
}
#[test]
fn cute_and_chibi_are_64px_before_modifiers_and_explicit_override_wins() {
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        for name in ["beetle", "humanoid", "elephant", "trex", "mermaid", "bat"] {
            for style in ["cute", "chibi"] {
                assert_eq!(
                    world_scale::target_height(&spec(&format!("{style} {name}"), profile), 42),
                    64.0
                );
                for word in ["giant", "mini"] {
                    let mut s = spec(&format!("{word} {style} {name}"), profile);
                    assert_eq!(
                        world_scale::target_height(&s, 42),
                        64.0 * SizeModifier::from_word(word).factor(42)
                    );
                    s.world_height_px = Some(123.0);
                    assert_eq!(world_scale::target_height(&s, 42), 123.0);
                }
            }
        }
    }
}
#[test]
fn part_modifiers_do_not_resize_the_entire_creature_and_last_body_modifier_wins() {
    for (prompt, modifier) in [
        ("orc with giant sword", SizeModifier::Normal),
        ("bat with huge wings", SizeModifier::Normal),
        ("giant orc with mini sword", SizeModifier::Giant),
        ("giant mini wolf", SizeModifier::Mini),
        ("mini giant wolf", SizeModifier::Giant),
    ] {
        assert_eq!(
            parser::parse_vocabulary(prompt).world_size_modifier,
            modifier,
            "{prompt}"
        );
    }
}
#[test]
fn species_default_sizes_are_distinct_and_player_relative() {
    let h = |name| world_scale::target_height(&parser::parse_vocabulary(name), 42);
    assert_eq!(h("humanoid"), 96.0);
    assert!(h("beetle") < h("wolf") && h("wolf") < h("humanoid"));
    assert!(h("elephant") > 128.0 && h("trex") > h("elephant"));
    assert!(h("fish") < 70.0 && h("bat") < 70.0);
}
#[test]
fn reference_height_ignores_oversized_wings_weapons_tails_and_canvas() {
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        for prompt in ["dragon with sword", "bat", "wolf"] {
            let mut s = spec(prompt, profile);
            let mut b = body(&mut s, profile, 7);
            let before = render3d::reference_body_height(&s, &b, 7);
            for node in &mut b.nodes {
                if ["WING", "WEAPON", "TAIL"].contains(&node.kind.as_str()) {
                    node.x += 1000.0;
                    node.y -= 1000.0;
                    node.rx *= 50.0;
                    node.ry *= 50.0;
                }
            }
            assert_eq!(
                before,
                render3d::reference_body_height(&s, &b, 7),
                "{profile:?} {prompt}"
            );
            let info = world_scale::size_info(&s, &b, 7);
            assert!(
                (info.reference_height_px * info.world_scale - info.target_height_px).abs() < 0.001
            );
        }
    }
}
#[test]
fn generated_packages_publish_one_scale_for_sprites_colliders_and_attack_tracks() {
    let dir = tempfile::tempdir().unwrap();
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        for prompt in [
            "mini beetle",
            "humanoid",
            "giant wolf",
            "cute elephant",
            "chibi bat",
            "fish",
        ] {
            let s = spec(prompt, profile);
            let expected = world_scale::target_height(&s, 7);
            let monster =
                generate_from_spec_with_profile(s, 7, dir.path(), PackageFormat::Protobuf, profile)
                    .unwrap();
            let size = monster.size.as_ref().unwrap();
            assert!(world_scale::valid_size_info(size));
            assert_eq!(size.target_height_px, expected);
            assert_eq!(size.reference_player_height_px, 96.0);
            assert!(monster.colliders.iter().filter(|c| c.hurtbox).all(|c| c
                .views
                .iter()
                .all(|v| (v.radius * size.world_scale).is_finite())));
            assert_eq!(
                monster.v2_profile.is_some(),
                profile == GenerationProfile::V2
            );
            assert_eq!(
                proto::Monster::decode(monster.encode_to_vec().as_slice()).unwrap(),
                monster
            );
            let mut bytes = Vec::new();
            ciborium::into_writer(&monster, &mut bytes).unwrap();
            assert_eq!(
                ciborium::from_reader::<proto::Monster, _>(&bytes[..]).unwrap(),
                monster
            );
            assert_eq!(
                serde_json::from_str::<proto::Monster>(&serde_json::to_string(&monster).unwrap())
                    .unwrap(),
                monster
            );
        }
    }
}
#[test]
fn invalid_explicit_heights_and_corrupt_metadata_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    for height in [0.0, -1.0, 1025.0, f32::NAN, f32::INFINITY] {
        let mut s = parser::parse_vocabulary("humanoid");
        s.world_height_px = Some(height);
        assert!(generate_from_spec_with_profile(
            s,
            0,
            dir.path(),
            PackageFormat::Protobuf,
            GenerationProfile::V1
        )
        .is_err());
    }
    assert!(world_scale::valid_size_info(&proto::SizeInfo::default()));
    assert!(!world_scale::valid_size_info(&proto::SizeInfo {
        world_scale: f32::NAN,
        ..Default::default()
    }));
    assert!(!world_scale::valid_size_info(&proto::SizeInfo {
        world_scale: 2.0,
        reference_height_px: 30.0,
        target_height_px: 64.0,
        reference_player_height_px: 96.0,
        ..Default::default()
    }));
}
#[cfg(feature = "bert")]
#[test]
fn native_bert_keeps_explicit_world_size_intent() {
    for prompt in [
        "giant elephant",
        "mini humanoid",
        "cute trex",
        "giant cute bat",
    ] {
        let vocabulary = parser::parse_vocabulary(prompt);
        let bert = parser::parse_with_embedded_bert(prompt).unwrap();
        assert_eq!(bert.world_size_modifier, vocabulary.world_size_modifier);
        assert_eq!(
            bert.features.contains(&"CHIBI".to_owned()),
            vocabulary.features.contains(&"CHIBI".to_owned())
        );
    }
}
