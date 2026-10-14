//! Validate mutated serialized metadata through the same entry point used on import.
use infernal_diffusion::{generate_in_memory_with_profile, proto, validate, GenerationProfile};
use std::sync::OnceLock;

fn fixture() -> &'static infernal_diffusion::GeneratedMonster {
    static FIXTURE: OnceLock<infernal_diffusion::GeneratedMonster> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        generate_in_memory_with_profile("friendly frog that sprays acid", 42, GenerationProfile::V2)
            .unwrap()
    })
}

fn reject_mutation(mutate: impl FnOnce(&mut proto::Monster)) {
    let root = fixture().packages.last().unwrap();
    validate(&root.monster, &root.sprites).unwrap();
    // Use a JSON roundtrip to exercise the public imported descriptor surface.
    let mut monster: proto::Monster =
        serde_json::from_value(serde_json::to_value(&root.monster).unwrap()).unwrap();
    mutate(&mut monster);
    assert!(
        validate(&monster, &root.sprites).is_err(),
        "malformed imported metadata was accepted"
    );
}

#[test]
fn imported_v2_rejects_dangling_extra_attack_events() {
    reject_mutation(|monster| {
        let attack = monster
            .attacks
            .iter()
            .find(|attack| attack.id.starts_with("v2_pattern_"))
            .unwrap();
        let clip = monster
            .animations
            .iter_mut()
            .find(|clip| clip.id == attack.animation_id)
            .unwrap();
        let mut event = clip
            .events
            .iter()
            .find(|event| event.kind == "PROJECTILE_SPAWN")
            .unwrap()
            .clone();
        event.reference_id = "nonexistent_attack".into();
        clip.events.push(event);
    });
}

#[test]
fn imported_v2_rejects_extra_projectile_events_without_recipe_step() {
    reject_mutation(|monster| {
        let attack = monster
            .attacks
            .iter()
            .find(|attack| attack.id.starts_with("v2_pattern_"))
            .unwrap();
        let clip = monster
            .animations
            .iter_mut()
            .find(|clip| clip.id == attack.animation_id)
            .unwrap();
        let mut event = clip
            .events
            .iter()
            .find(|event| event.kind == "PROJECTILE_SPAWN")
            .unwrap()
            .clone();
        event.time_ms += 1;
        clip.events.push(event);
    });
}

#[test]
fn imported_v2_rejects_inconsistent_mutant_coordination_modifiers() {
    reject_mutation(|monster| {
        let modifiers = &mut monster.v2_profile.as_mut().unwrap().modifiers;
        modifiers.retain(|modifier| modifier != "AWKWARD_COORDINATION");
        if !modifiers
            .iter()
            .any(|modifier| modifier == "MUTANT_UNSTABLE")
        {
            modifiers.push("MUTANT_UNSTABLE".into());
        }
    });
}

#[test]
fn imported_v2_rejects_lure_category_contradiction() {
    reject_mutation(|monster| {
        monster.v2_profile.as_mut().unwrap().categories =
            vec!["QUADRUPED".into(), "HOSTILE".into()];
    });
}

#[test]
fn imported_v2_rejects_dangling_extra_lure_event_reference() {
    reject_mutation(|monster| {
        let lure_id = &monster
            .v2_profile
            .as_ref()
            .unwrap()
            .lure
            .as_ref()
            .unwrap()
            .lure_animation_id;
        let clip = monster
            .animations
            .iter_mut()
            .find(|clip| &clip.id == lure_id)
            .unwrap();
        let mut event = clip
            .events
            .iter()
            .find(|event| event.kind == "LURE_HEARTS")
            .unwrap()
            .clone();
        event.reference_id = "nonexistent_clip".into();
        clip.events.push(event);
    });
}

fn mutant_fixture() -> &'static infernal_diffusion::GeneratedMonster {
    static FIXTURE: OnceLock<infernal_diffusion::GeneratedMonster> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let seed = (0..100)
            .find(|seed| {
                let mut spec = infernal_diffusion::v2::interpret(
                    infernal_diffusion::parser::parse_vocabulary("orc"),
                    "orc",
                );
                infernal_diffusion::v2::prepare(&mut spec);
                infernal_diffusion::v2::anatomy::prepare_seeded(&mut spec, *seed);
                spec.features.iter().any(|feature| feature == "FOUR_ARMED")
            })
            .unwrap();
        generate_in_memory_with_profile("orc", seed, GenerationProfile::V2).unwrap()
    })
}

#[test]
fn imported_v2_rejects_advertised_mutation_without_clip_markers() {
    let root = mutant_fixture().packages.last().unwrap();
    validate(&root.monster, &root.sprites).unwrap();
    for marker in [
        infernal_diffusion::v2::combat::AWKWARD_EVENT,
        infernal_diffusion::v2::combat::UNFOLD_EVENT,
    ] {
        let mut bad = root.monster.clone();
        assert!(bad
            .animations
            .iter()
            .flat_map(|clip| &clip.events)
            .any(|event| event.kind == marker));
        for clip in &mut bad.animations {
            clip.events.retain(|event| event.kind != marker);
        }
        assert!(
            validate(&bad, &root.sprites).is_err(),
            "accepted missing {marker} markers"
        );
    }
}

#[test]
fn imported_v2_rejects_duplicated_animation_modifier_markers() {
    let root = mutant_fixture().packages.last().unwrap();
    let mut bad = root.monster.clone();
    let clip = bad
        .animations
        .iter_mut()
        .find(|clip| {
            clip.events
                .iter()
                .any(|event| event.kind == infernal_diffusion::v2::combat::AWKWARD_EVENT)
        })
        .unwrap();
    let event = clip
        .events
        .iter()
        .find(|event| event.kind == infernal_diffusion::v2::combat::AWKWARD_EVENT)
        .unwrap()
        .clone();
    clip.events.push(event);
    assert!(
        validate(&bad, &root.sprites).is_err(),
        "accepted duplicate animation modifier marker"
    );
}

#[test]
fn imported_v2_rejects_missing_or_mistimed_tree_extension_marker() {
    let generated =
        generate_in_memory_with_profile("tree monster", 42, GenerationProfile::V2).unwrap();
    let root = generated.packages.last().unwrap();
    validate(&root.monster, &root.sprites).unwrap();
    for remove in [false, true] {
        let mut bad = root.monster.clone();
        let clip = bad
            .animations
            .iter_mut()
            .find(|clip| clip.id == "attack_v2_tree_bilateral_smash")
            .unwrap();
        if remove {
            clip.events
                .retain(|event| event.kind != infernal_diffusion::v2::combat::ARM_EXTENSION_EVENT);
        } else {
            clip.events
                .iter_mut()
                .find(|event| event.kind == infernal_diffusion::v2::combat::ARM_EXTENSION_EVENT)
                .unwrap()
                .time_ms = 1;
        }
        assert!(validate(&bad, &root.sprites).is_err());
    }
}

#[test]
fn imported_v2_rejects_mermaid_lunge_without_lunge_step() {
    let generated = generate_in_memory_with_profile("mermaid", 42, GenerationProfile::V2).unwrap();
    let root = generated.packages.last().unwrap();
    validate(&root.monster, &root.sprites).unwrap();
    let mut bad = root.monster.clone();
    let attack = bad
        .attacks
        .iter_mut()
        .find(|attack| attack.id == "v2_mermaid_lunge")
        .unwrap();
    assert!(attack.steps.iter().any(|step| step.primitive == "DASH"));
    attack.steps.retain(|step| step.primitive != "DASH");
    assert!(
        validate(&bad, &root.sprites).is_err(),
        "advertised lunge has no executable lunge step"
    );
}

#[test]
fn imported_v2_rejects_duplicate_reveal_completion_events() {
    reject_mutation(|monster| {
        let reveal_id = &monster
            .v2_profile
            .as_ref()
            .unwrap()
            .lure
            .as_ref()
            .unwrap()
            .reveal_animation_id;
        let clip = monster
            .animations
            .iter_mut()
            .find(|clip| &clip.id == reveal_id)
            .unwrap();
        let event = clip
            .events
            .iter()
            .find(|event| event.kind == "REVEAL_COMPLETE")
            .unwrap()
            .clone();
        clip.events.push(event);
    });
}

#[test]
fn imported_v2_cannot_remove_descriptor_while_keeping_provenance() {
    reject_mutation(|monster| monster.v2_profile = None);
}

#[test]
fn imported_v2_rejects_missing_or_malformed_posed_attack_tracks() {
    for corruption in 0..7 {
        reject_mutation(|monster| {
            let pattern = &mut monster.v2_profile.as_mut().unwrap().attack_patterns[0];
            assert!(
                !pattern.motion_samples.is_empty(),
                "generated attack lacks posed origin track"
            );
            match corruption {
                0 => pattern.motion_samples.clear(),
                1 => pattern.motion_samples[0].node_id = "missing_node".into(),
                2 => pattern.motion_samples[0].sprite_frame_id = u32::MAX,
                3 => pattern.motion_samples[0].time_ms = u32::MAX,
                4 => pattern.motion_samples[0].views[0].x = f32::NAN,
                5 => pattern.motion_samples[0].active = !pattern.motion_samples[0].active,
                _ => pattern.motion_samples[0].views[0].direction_index = 99,
            }
        });
    }
}

fn shade_fixture() -> &'static infernal_diffusion::GeneratedMonster {
    static FIXTURE: OnceLock<infernal_diffusion::GeneratedMonster> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        generate_in_memory_with_profile("shade humanoid", 42, GenerationProfile::V2).unwrap()
    })
}

#[test]
fn shade_receivers_cover_every_frame_and_reverse_exactly_after_roundtrip() {
    use prost::Message;
    let package = shade_fixture().packages.last().unwrap();
    let monster = &package.monster;
    validate(monster, &package.sprites).unwrap();
    let profile = monster.v2_profile.as_ref().unwrap();
    let expected: std::collections::BTreeSet<_> = monster
        .colliders
        .iter()
        .filter(|c| c.hurtbox)
        .map(|c| c.node_id.as_str())
        .collect();
    let mut cursor = 0;
    for attack in &monster.attacks {
        let clip = monster
            .animations
            .iter()
            .find(|c| c.id == attack.animation_id)
            .unwrap();
        let samples = &profile.body_motion_samples[cursor..cursor + clip.frames.len()];
        assert_eq!(samples.len(), 27);
        for (index, sample) in samples.iter().enumerate() {
            assert_eq!(sample.sprite_frame_id, clip.frames[index].sprite_frame_id);
            assert_eq!(
                sample
                    .hurtboxes
                    .iter()
                    .map(|h| h.node_id.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                expected
            );
            assert_eq!(sample.hurtboxes, samples[26 - index].hurtboxes);
        }
        cursor += samples.len();
    }
    assert_eq!(cursor, profile.body_motion_samples.len());
    let protobuf = proto::Monster::decode(monster.encode_to_vec().as_slice()).unwrap();
    let json: proto::Monster =
        serde_json::from_str(&serde_json::to_string(monster).unwrap()).unwrap();
    for decoded in [protobuf, json] {
        assert_eq!(
            decoded.v2_profile.as_ref().unwrap().body_motion_samples,
            profile.body_motion_samples
        );
        validate(&decoded, &package.sprites).unwrap();
    }
}

#[test]
fn shade_import_rejects_missing_reordered_nonfinite_and_dangling_receivers() {
    let package = shade_fixture().packages.last().unwrap();
    for corruption in 0..8 {
        let mut monster = package.monster.clone();
        let samples = &mut monster.v2_profile.as_mut().unwrap().body_motion_samples;
        match corruption {
            0 => samples.clear(),
            1 => {
                samples.remove(1);
            }
            2 => samples.swap(0, 1),
            3 => samples[0].hurtboxes[0].views[0].x = f32::NAN,
            4 => samples[0].hurtboxes[0].node_id = "dangling_receiver".into(),
            5 => {
                samples[0].hurtboxes.pop();
            }
            6 => samples[26].hurtboxes[0].views[0].x += 1.0,
            7 => samples[0].hurtboxes[0].views.swap(0, 1),
            _ => unreachable!(),
        }
        assert!(
            validate(&monster, &package.sprites).is_err(),
            "accepted receiver corruption {corruption}"
        );
    }
}
