use infernal_diffusion::{parser, proto::*, v2};
use prost::Message;

fn spec(prompt: &str) -> parser::MonsterSpec {
    let mut s = v2::interpret(parser::parse_vocabulary(prompt), prompt);
    v2::prepare(&mut s);
    s
}
fn fixture(prompt: &str) -> Monster {
    let s = spec(prompt);
    let d = v2::encounters::describe(&s).unwrap();
    let mut monster = Monster {
        tags: s.features.clone(),
        ..Default::default()
    };
    let mut clips = vec![];
    let mut add = |id: &str, ms: u32, events: Vec<AnimationEvent>| {
        clips.push(Animation {
            id: id.into(),
            frames: vec![AnimationFrame {
                duration_ms: ms,
                ..Default::default()
            }],
            events,
            ..Default::default()
        })
    };
    let mut lure = None;
    if let Some(s) = &d.split {
        add(&s.emergence_animation_id, s.emergence_ms, vec![]);
        add(&s.burst_animation_id, 600, vec![]);
    }
    if let Some(s) = &d.shield {
        add(&s.guard_animation_id, 960, vec![]);
        add(&s.bash_animation_id, 800, vec![]);
        add(&s.recovery_animation_id, 720, vec![]);
    }
    if let Some(p) = &d.pushed_prop {
        add(
            &p.push_animation_id,
            960,
            vec![AnimationEvent {
                time_ms: 720,
                kind: "PUSH_RELEASE".into(),
                reference_id: "boulder".into(),
            }],
        );
        add(&p.recovery_animation_id, 720, vec![]);
    }
    if let Some(m) = &d.mimic {
        lure = Some(LureDescriptor {
            trigger_radius: m.trigger_radius,
            telegraph_ms: m.reveal_ms,
            effect: "OBJECT_DISGUISE".into(),
            reset_policy: "STAY_REVEALED".into(),
            ..Default::default()
        });
    }
    monster.animations = clips;
    monster.v2_profile = Some(V2Profile {
        encounter: Some(d),
        lure,
        ..Default::default()
    });
    monster
}

#[test]
fn identity_recipes_are_v2_only_and_disguises_never_chests() {
    for (prompt, affinity, disguise) in [
        ("splitting slime", "SLIME", ""),
        ("bush mimic", "OBJECT_MIMIC", "BUSH"),
        ("oversized banana mimic", "OBJECT_MIMIC", "LARGE_BANANA"),
        ("vase mimic", "OBJECT_MIMIC", "VASE"),
        ("amphora mimic", "OBJECT_MIMIC", "VASE"),
        ("amorpha", "OBJECT_MIMIC", "VASE"),
        ("chest mimic", "OBJECT_MIMIC", "VASE"),
        ("bronze shield sentinel", "SHIELD_SENTINEL", ""),
        ("boulder pusher", "BOULDER_PUSHER", ""),
        ("Sisyphus pushing a rock", "BOULDER_PUSHER", ""),
    ] {
        let legacy = parser::parse_vocabulary(prompt);
        assert!(!legacy.features.iter().any(|f| [
            "SPLITTING_SLIME",
            "OBJECT_MIMIC",
            "SHIELD_SENTINEL",
            "BOULDER_PUSHER"
        ]
        .contains(&f.as_str())));
        let s = spec(prompt);
        assert_eq!(s.affinity, affinity, "{prompt}");
        let encounter = v2::encounters::describe(&s).unwrap();
        if !disguise.is_empty() {
            assert_eq!(encounter.mimic.unwrap().disguise, disguise);
        }
        assert!(s.spawn.is_none());
        v2::encounters::validate(&fixture(prompt)).unwrap();
    }
    assert!(v2::encounters::describe(&spec("ordinary slime")).is_none());
    assert!(v2::encounters::describe(&spec("goblin beside a rock")).is_none());
}

#[test]
fn encounter_descriptors_roundtrip_all_wire_formats() {
    for prompt in [
        "splitting slime",
        "large banana mimic",
        "bronze shield sentinel",
        "boulder pusher",
    ] {
        let mut original = fixture(prompt);
        original.v2_profile.as_mut().unwrap().damage_traits =
            v2::damage_traits::describe(&spec(prompt));
        let pb = Monster::decode(original.encode_to_vec().as_slice()).unwrap();
        assert_eq!(original, pb);
        let mut cbor = Vec::new();
        ciborium::into_writer(&original, &mut cbor).unwrap();
        let decoded: Monster = ciborium::from_reader(cbor.as_slice()).unwrap();
        assert_eq!(original, decoded);
        let json = serde_json::to_value(&original).unwrap();
        assert_eq!(json["v2_profile"]["encounter"]["contract_version"], 1);
        assert_eq!(serde_json::from_value::<Monster>(json).unwrap(), original);
    }
    let legacy: V2Profile = serde_json::from_str(r#"{"profile_version":1}"#).unwrap();
    assert!(legacy.encounter.is_none());
    assert!(serde_json::to_value(legacy)
        .unwrap()
        .get("encounter")
        .is_none());
}

#[test]
fn rejects_unbounded_or_ambiguous_split_contracts() {
    for change in 0..8 {
        let mut m = fixture("splitting slime");
        let d = m.v2_profile.as_mut().unwrap().encounter.as_mut().unwrap();
        let s = d.split.as_mut().unwrap();
        match change {
            0 => s.count = 20,
            1 => s.max_generations = 50,
            2 => s.max_offspring = 100,
            3 => s.child_health_fraction = 0.5,
            4 => s.child_scale = f32::NAN,
            5 => s.emergence_ms = 0,
            6 => s.emergence_animation_id = "missing".into(),
            _ => d.mimic = Some(MimicDescriptor::default()),
        }
        assert!(v2::encounters::validate(&m).is_err(), "case {change}");
    }
}

#[test]
fn rejects_unknown_disguises_and_missing_physical_prop_events() {
    let mut mimic = fixture("vase mimic");
    mimic
        .v2_profile
        .as_mut()
        .unwrap()
        .encounter
        .as_mut()
        .unwrap()
        .mimic
        .as_mut()
        .unwrap()
        .disguise = "CHEST".into();
    assert!(v2::encounters::validate(&mimic).is_err());
    for change in 0..5 {
        let mut m = fixture("boulder pusher");
        let p = m
            .v2_profile
            .as_mut()
            .unwrap()
            .encounter
            .as_mut()
            .unwrap()
            .pushed_prop
            .as_mut()
            .unwrap();
        match change {
            0 => p.coordinate_space = "UNKNOWN".into(),
            1 => p.radius = f32::INFINITY,
            2 => m.animations[0].events.clear(),
            3 => m.animations[0].events[0].time_ms = 960,
            _ => {
                let e = m.animations[0].events[0].clone();
                m.animations[0].events.push(e);
            }
        }
        assert!(v2::encounters::validate(&m).is_err(), "case {change}");
    }
    let mut m = fixture("splitting slime");
    m.v2_profile.as_mut().unwrap().encounter = None;
    assert!(v2::encounters::validate(&m).is_err());
}

#[test]
fn actual_generated_encounters_validate_and_keep_actor_prop_separate() {
    use infernal_diffusion::{generate_in_memory_with_profile, validate, GenerationProfile};
    for prompt in [
        "splitting slime",
        "bush mimic",
        "large banana mimic",
        "vase mimic",
        "bronze shield sentinel",
        "boulder pusher",
    ] {
        let generated = generate_in_memory_with_profile(prompt, 7, GenerationProfile::V2).unwrap();
        let root = generated.packages.last().unwrap();
        validate(&root.monster, &root.sprites).unwrap();
        let profile = root.monster.v2_profile.as_ref().unwrap();
        let descriptor = profile.encounter.as_ref().unwrap();
        assert_eq!(descriptor.contract_version, 1);
        assert_eq!(
            generated.packages.len(),
            1,
            "encounter descendants/rock are engine entities, not packaged minions: {prompt}"
        );
        assert!(!profile
            .modifiers
            .iter()
            .any(|m| ["DANCER", "MUTANT_UNSTABLE"].contains(&m.as_str())));
        if descriptor.mimic.is_some() {
            assert_eq!(profile.lure.as_ref().unwrap().reset_policy, "STAY_REVEALED");
            assert!(!root
                .monster
                .animations
                .iter()
                .flat_map(|a| &a.events)
                .any(|e| e.kind == "LURE_HEARTS"));
        }
        if descriptor.pushed_prop.is_some() {
            assert_eq!(
                root.monster
                    .animations
                    .iter()
                    .flat_map(|a| &a.events)
                    .filter(|e| e.kind == "PUSH_RELEASE")
                    .count(),
                1
            );
            assert!(!root
                .monster
                .attacks
                .iter()
                .flat_map(|a| &a.steps)
                .any(|s| s.primitive == "SPAWN_MINION" || s.primitive == "SPAWN_PROJECTILE"));
        }
    }
}

#[test]
fn unsupported_explicit_encounter_topology_fails_before_rendering() {
    for prompt in [
        "three-headed bronze shield sentinel",
        "boulder pusher with eight arms",
        "splitting slime with four legs",
    ] {
        assert!(
            v2::encounters::validate_spec(&spec(prompt)).is_err(),
            "{prompt}"
        );
    }
    assert!(v2::encounters::validate_spec(&spec("bronze shield sentinel")).is_ok());
}
