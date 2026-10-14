use infernal_diffusion::proto::{DamageRule, PhaseWindow, SpectralDescriptor, V2Profile};
use prost::Message;

#[test]
fn spectral_rules_and_windows_roundtrip_without_losing_precedence_or_references() {
    let original = V2Profile {
        profile_version: 1,
        spectral: Some(SpectralDescriptor {
            kind: "SHADE".into(),
            visual_style: "SMOKY".into(),
            opacity: 0.6,
            collision_mode: "RULE_DRIVEN".into(),
            damage_rules: vec![
                DamageRule {
                    priority: 100,
                    elements: vec!["FIRE".into(), "WATER".into()],
                    damage_multiplier: 1.0,
                    ..Default::default()
                },
                DamageRule {
                    priority: 10,
                    delivery: "PROJECTILE".into(),
                    projectile_pass_through: true,
                    ..Default::default()
                },
            ],
            default_damage_multiplier: 1.0,
            default_projectile_pass_through: false,
            phase_windows: vec![PhaseWindow {
                animation_id: "phase".into(),
                start_ms: 120,
                end_ms: 360,
                solid_pass_through: true,
            }],
        }),
        ..Default::default()
    };
    assert_eq!(
        V2Profile::decode(original.encode_to_vec().as_slice()).unwrap(),
        original
    );
    let mut cbor = Vec::new();
    ciborium::into_writer(&original, &mut cbor).unwrap();
    let decoded: V2Profile = ciborium::from_reader(cbor.as_slice()).unwrap();
    assert_eq!(decoded, original);
    let json = serde_json::to_value(&original).unwrap();
    assert_eq!(json["spectral"]["damage_rules"][0]["priority"], 100);
    assert_eq!(serde_json::from_value::<V2Profile>(json).unwrap(), original);
}

#[test]
fn absent_spectral_extension_preserves_existing_json_shape() {
    let legacy = V2Profile {
        profile_version: 1,
        ..Default::default()
    };
    assert!(serde_json::to_value(&legacy)
        .unwrap()
        .get("spectral")
        .is_none());
    let decoded: V2Profile = serde_json::from_str(r#"{"profile_version":1}"#).unwrap();
    assert!(decoded.spectral.is_none());
}

fn spectral_monster(kind: &str) -> infernal_diffusion::proto::Monster {
    use infernal_diffusion::proto::{Animation, AnimationEvent, AnimationFrame, Attack, Monster};
    let mut spec = infernal_diffusion::parser::parse_vocabulary("humanoid");
    if kind == "GHOUL" {
        spec.affinity = "GHOUL".into();
    } else {
        spec.features.push(kind.into());
    }
    let animations = vec![Animation {
        id: "phase".into(),
        frames: vec![AnimationFrame {
            duration_ms: 1000,
            ..Default::default()
        }],
        events: vec![
            AnimationEvent {
                kind: "PHASE_BEGIN".into(),
                time_ms: 100,
                reference_id: "phase_attack".into(),
            },
            AnimationEvent {
                kind: "PHASE_END".into(),
                time_ms: 700,
                reference_id: "phase_attack".into(),
            },
        ],
        ..Default::default()
    }];
    let spectral = infernal_diffusion::v2::spectral::describe(&spec, &animations);
    Monster {
        tags: vec![kind.into()],
        attacks: vec![Attack {
            id: "phase_attack".into(),
            animation_id: "phase".into(),
            ..Default::default()
        }],
        animations,
        v2_profile: Some(V2Profile {
            profile_version: 1,
            spectral,
            ..Default::default()
        }),
        ..Default::default()
    }
}

#[test]
fn spectral_damage_precedence_and_ghoul_vulnerability_are_independent_of_opacity() {
    use infernal_diffusion::v2::spectral::damage_response;
    for kind in ["SHADE", "GHOST", "GHOUL"] {
        let monster = spectral_monster(kind);
        let spectral = monster
            .v2_profile
            .as_ref()
            .unwrap()
            .spectral
            .as_ref()
            .unwrap();
        for delivery in ["PROJECTILE", "MELEE"] {
            for element in ["FIRE", "WATER", "PHYSICAL", "MAGIC"] {
                let response = damage_response(spectral, element, delivery, &[]);
                let immune = match kind {
                    "GHOUL" => false,
                    "GHOST" => element != "MAGIC",
                    _ => delivery == "PROJECTILE" && !["FIRE", "WATER"].contains(&element),
                };
                assert_eq!(
                    response.damage_multiplier,
                    if immune { 0.0 } else { 1.0 },
                    "{kind}/{element}/{delivery}"
                );
                assert_eq!(
                    response.projectile_pass_through,
                    immune && delivery == "PROJECTILE"
                );
                if kind == "GHOST" {
                    let magic = damage_response(spectral, element, delivery, &["MAGIC"]);
                    assert_eq!(magic.damage_multiplier, 1.0);
                    assert!(!magic.projectile_pass_through);
                }
            }
        }
    }
}

#[test]
fn spectral_import_rejects_unpaired_phase_events_even_without_a_window() {
    let mut monster = spectral_monster("GHOUL");
    infernal_diffusion::v2::spectral::validate(&monster).unwrap();
    monster.animations[0]
        .events
        .retain(|event| event.kind != "PHASE_END");
    monster
        .v2_profile
        .as_mut()
        .unwrap()
        .spectral
        .as_mut()
        .unwrap()
        .phase_windows
        .clear();
    assert!(infernal_diffusion::v2::spectral::validate(&monster).is_err());
}

#[test]
fn spectral_import_rejects_phase_event_references_to_other_clips() {
    let mut monster = spectral_monster("GHOUL");
    monster.animations[0].events[0].reference_id = "missing_clip".into();
    assert!(infernal_diffusion::v2::spectral::validate(&monster).is_err());
}

#[test]
fn solid_phasing_uses_half_open_clip_relative_windows() {
    use infernal_diffusion::v2::spectral::passes_solids;
    let ghoul = spectral_monster("GHOUL");
    let profile = ghoul
        .v2_profile
        .as_ref()
        .unwrap()
        .spectral
        .as_ref()
        .unwrap();
    assert!(!passes_solids(profile, "phase", 99));
    assert!(passes_solids(profile, "phase", 100));
    assert!(passes_solids(profile, "phase", 699));
    assert!(!passes_solids(profile, "phase", 700));
    assert!(!passes_solids(profile, "other_clip", 200));
    let ghost = spectral_monster("GHOST");
    let profile = ghost
        .v2_profile
        .as_ref()
        .unwrap()
        .spectral
        .as_ref()
        .unwrap();
    assert!(passes_solids(profile, "other_clip", 0));
}

#[test]
fn posed_attack_origin_tracks_roundtrip_directional_views() {
    use infernal_diffusion::proto::{AttackOriginSample, AttackPatternDescriptor, ColliderView};
    let pattern = AttackPatternDescriptor {
        attack_id: "tree_smash".into(),
        motion_samples: vec![AttackOriginSample {
            time_ms: 650,
            sprite_frame_id: 12,
            node_id: "hand_l".into(),
            views: vec![
                ColliderView {
                    direction_index: 0,
                    x: 42.0,
                    y: 60.0,
                    radius: 8.0,
                },
                ColliderView {
                    direction_index: 1,
                    x: 48.0,
                    y: 60.0,
                    radius: 8.0,
                },
            ],
            active: true,
        }],
        ..Default::default()
    };
    assert_eq!(
        AttackPatternDescriptor::decode(pattern.encode_to_vec().as_slice()).unwrap(),
        pattern
    );
    let mut cbor = Vec::new();
    ciborium::into_writer(&pattern, &mut cbor).unwrap();
    let decoded: AttackPatternDescriptor = ciborium::from_reader(cbor.as_slice()).unwrap();
    assert_eq!(decoded, pattern);
}
