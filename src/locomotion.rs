//! Engine-facing capabilities, independent of demo AI, targets, or path planning.
//!
//! `MovementMode::speed` is a reference in generator length units per second;
//! `AnimationFrame::root_dx/root_dy` use generator length units per frame.
//! An engine may use either, rescale them, or drive its own controller entirely.

use crate::{
    parser::MonsterSpec,
    proto::{Animation, MovementMode},
};

/// The bottom creature supplies support and propulsion for a mounted stack.
/// Resolve each layer as anatomy does, including legacy mount summaries and the
/// size inherited from its rider. Rider-only fins, wings or anchoring cannot
/// change the substrate's physical locomotion.
pub(crate) fn substrate(spec: &MonsterSpec) -> std::borrow::Cow<'_, MonsterSpec> {
    let mut source = std::borrow::Cow::Borrowed(spec);
    while let Some(mount) = &source.mount {
        let next = crate::parser::mount_spec(&source, mount);
        source = std::borrow::Cow::Owned(next);
    }
    source
}

/// Annotate existing clip-linked modes without changing the legacy gait IDs.
pub(crate) fn describe(spec: &MonsterSpec, animations: &[Animation], modes: &mut [MovementMode]) {
    let mounted = spec.mount.is_some();
    let source = substrate(spec);
    let anchored = source.features.iter().any(|feature| feature == "ANCHORED");
    let aquatic = source.features.iter().any(|feature| feature == "FIN");
    let can_jump = animations.iter().any(|clip| clip.id == "jump_air");
    let base_gait = modes
        .iter()
        .find(|mode| mode.animation_id == "move")
        .map(|mode| mode.id.clone())
        .unwrap_or_default();
    for mode in modes {
        mode.category = match mode.id.as_str() {
            _ if anchored => "STATIONARY",
            "FLY" => "AERIAL",
            "FLOAT" | "FAST_FLOAT" | "SERPENTINE_SLITHER" | "RAPID_SLITHER" if aquatic => {
                "SWIMMING"
            }
            "FLOAT" | "FAST_FLOAT" => "AERIAL",
            "CLIMB" => "CLIMBING",
            "BURROW" => "BURROWING",
            _ => "GROUNDED",
        }
        .into();
        // FAST_MOVE is the legacy fallback ID (e.g. a bat's faster crawl).
        // It changes pacing, not the physical style of its matching base gait.
        let gait_id = if mode.id == "FAST_MOVE" {
            &base_gait
        } else {
            &mode.id
        };
        let gait = match gait_id.as_str() {
            "BIPED_WALK" | "BIPED_RUN" | "THEROPOD_STRIDE" | "THEROPOD_RUN" => "BIPEDAL",
            "QUADRUPED_WALK" | "QUADRUPED_RUN" => "QUADRUPEDAL",
            "MULTILEG_SCUTTLE" | "FAST_SCUTTLE" => "MULTILEGGED",
            "SERPENTINE_SLITHER" | "RAPID_SLITHER" if aquatic => "UNDULATING",
            "SERPENTINE_SLITHER" | "RAPID_SLITHER" => "SLITHERING",
            "BEACHED_FLOP" | "BEACHED_THRASH" => "FLOPPING",
            "SNAIL_CRAWL" | "SNAIL_SURGE" | "GLIDE" | "RAPID_GLIDE" | "CRAWL" => "CRAWLING",
            "OOZE_CRAWL" | "OOZE_SURGE" => "OOZING",
            "ROOT_SHUFFLE" | "ROOT_SURGE" => "ROOTED_GAIT",
            "TREAD_ROLL" | "TREAD_CHARGE" => "TRACKED",
            "LIMP" | "LIMP_FAST" => "LIMPING",
            "HOP" | "FAST_HOP" => "HOPPING",
            "FLOAT" | "FAST_FLOAT" if aquatic => "UNDULATING",
            "FLOAT" | "FAST_FLOAT" => "LEVITATING",
            "FLY" if source.affinity == "DRONE" => "POWERED_FLIGHT",
            "FLY" => "WINGED",
            _ => "",
        };
        mode.modifiers.clear();
        if !gait.is_empty() && !anchored {
            mode.modifiers.push(gait.into());
        }
        if !anchored && mode.animation_id == "fast_move" {
            mode.modifiers.push("FAST".into());
        }
        if !anchored && can_jump && mode.category == "GROUNDED" {
            mode.modifiers.push("JUMP_CAPABLE".into());
        }
        if mounted {
            mode.modifiers.push("MOUNTED".into());
        }
        // Preserve independent physical constraints on every mobile controller.
        if !anchored && source.features.iter().any(|feature| feature == "LIMP") {
            mode.modifiers.push("LIMPING".into());
        }
        mode.modifiers.sort();
        mode.modifiers.dedup();
    }
}

/// Accept old empty descriptors and extensible uppercase tokens; reject malformed
/// or contradictory stationary descriptors. Unknown tokens remain engine-defined.
pub(crate) fn valid_descriptor(mode: &MovementMode) -> bool {
    fn token(value: &str) -> bool {
        !value.is_empty()
            && value.len() <= 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
    }
    if mode.category.is_empty() {
        return mode.modifiers.is_empty();
    }
    let mut seen = std::collections::HashSet::new();
    token(&mode.category)
        && mode
            .modifiers
            .iter()
            .all(|modifier| token(modifier) && seen.insert(modifier))
        && (mode.category != "STATIONARY" || mode.speed == 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use rand::SeedableRng;

    fn build_modes(prompt: &str) -> (Vec<Animation>, Vec<MovementMode>) {
        let spec = crate::parser::parse_vocabulary(prompt);
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let body = crate::anatomy::build(&spec, &mut rng);
        let (animations, modes, _, _) = crate::animation::make(&spec, &body);
        (animations, modes)
    }

    #[test]
    fn capabilities_describe_physics_and_bind_clips_without_tactics() {
        for (prompt, category, modifier) in [
            ("wolf", "GROUNDED", "QUADRUPEDAL"),
            ("spider", "GROUNDED", "MULTILEGGED"),
            ("snail", "GROUNDED", "CRAWLING"),
            ("ghost", "AERIAL", "LEVITATING"),
            ("fish", "SWIMMING", "UNDULATING"),
            ("limping goblin", "GROUNDED", "LIMPING"),
        ] {
            let (animations, modes) = build_modes(prompt);
            assert_eq!(modes[0].category, category, "{prompt}");
            assert!(
                modes[0].modifiers.iter().any(|value| value == modifier),
                "{prompt}"
            );
            assert!(modes.iter().all(valid_descriptor));
            for mode in modes {
                let clip = animations
                    .iter()
                    .find(|clip| clip.id == mode.animation_id)
                    .unwrap();
                assert_eq!(clip.movement_mode, mode.id);
                for frame in &clip.frames {
                    let distance = frame.root_dx.hypot(frame.root_dy);
                    assert!(
                        (distance - mode.speed * frame.duration_ms as f32 / 1000.0).abs() < 0.0001
                    );
                }
            }
        }
        let (_, modes) = build_modes("winged demon");
        assert!(modes
            .iter()
            .any(|mode| mode.category == "AERIAL" && mode.modifiers.contains(&"WINGED".into())));
        let (_, modes) = build_modes("burrowing scarab");
        assert!(modes.iter().any(|mode| mode.category == "BURROWING"));
    }

    #[test]
    fn generic_fast_variant_retains_its_base_gait_modifier() {
        for prompt in ["bat", "goblin riding a bat"] {
            let (_, modes) = build_modes(prompt);
            assert_eq!(modes[0].id, "CRAWL");
            let fast = modes
                .iter()
                .find(|mode| mode.animation_id == "fast_move")
                .unwrap();
            assert_eq!(fast.id, "FAST_MOVE");
            assert!(fast.modifiers.contains(&"CRAWLING".into()), "{prompt}");
            assert!(fast.modifiers.contains(&"FAST".into()), "{prompt}");
        }
    }

    #[test]
    fn mounted_modes_and_pose_archetypes_follow_the_substrate() {
        for (prompt, id, category, modifier, archetype) in [
            (
                "goblin riding a fish",
                "SERPENTINE_SLITHER",
                "SWIMMING",
                "UNDULATING",
                "AQUATIC",
            ),
            (
                "goblin riding a snail",
                "SNAIL_CRAWL",
                "GROUNDED",
                "CRAWLING",
                "GASTROPOD",
            ),
            (
                "goblin riding a tyrannosaurus",
                "THEROPOD_STRIDE",
                "GROUNDED",
                "BIPEDAL",
                "THEROPOD",
            ),
            (
                "goblin riding a spider",
                "MULTILEG_SCUTTLE",
                "GROUNDED",
                "MULTILEGGED",
                "ARACHNID",
            ),
            (
                "goblin riding a dragon",
                "QUADRUPED_WALK",
                "GROUNDED",
                "QUADRUPEDAL",
                "DRAGON",
            ),
            (
                "goblin riding a wolf riding a fish",
                "SERPENTINE_SLITHER",
                "SWIMMING",
                "UNDULATING",
                "AQUATIC",
            ),
            (
                "goblin riding a wolf riding a spider",
                "MULTILEG_SCUTTLE",
                "GROUNDED",
                "MULTILEGGED",
                "ARACHNID",
            ),
            (
                "goblin riding a spider riding a dragon",
                "QUADRUPED_WALK",
                "GROUNDED",
                "QUADRUPEDAL",
                "DRAGON",
            ),
        ] {
            let spec = crate::parser::parse_vocabulary(prompt);
            assert!(spec.mount.is_some(), "{prompt}");
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
            let body = crate::anatomy::build(&spec, &mut rng);
            let (animations, modes, _, poses) = crate::animation::make(&spec, &body);
            assert_eq!(modes[0].id, id, "{prompt}");
            assert_eq!(modes[0].category, category, "{prompt}");
            assert!(modes[0].modifiers.contains(&modifier.into()), "{prompt}");
            assert!(modes
                .iter()
                .all(|mode| mode.modifiers.contains(&"MOUNTED".into())));
            assert!(
                poses.iter().all(|pose| pose.motion_archetype == archetype),
                "{prompt}"
            );
            assert!(modes
                .iter()
                .all(|mode| animations.iter().any(|clip| clip.id == mode.animation_id)));
            assert_eq!(
                modes.iter().any(|mode| mode.id == "FLY"),
                archetype == "DRAGON",
                "{prompt}"
            );
            if category == "SWIMMING" || id == "SNAIL_CRAWL" {
                assert!(
                    !modes[0].modifiers.contains(&"JUMP_CAPABLE".into()),
                    "{prompt}"
                );
            }
        }
    }

    #[test]
    fn rider_traits_do_not_override_mount_propulsion() {
        let mut spec = crate::parser::parse_vocabulary("goblin riding a wolf");
        // These affect the seated rider, not the support/propulsion underneath it.
        spec.features
            .extend(["FIN", "WING", "ANCHORED", "LIMP", "BURROW", "CLIMB"].map(str::to_owned));
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let body = crate::anatomy::build(&spec, &mut rng);
        let (_, modes, _, poses) = crate::animation::make(&spec, &body);
        assert_eq!(modes[0].id, "QUADRUPED_WALK");
        assert!(modes
            .iter()
            .all(|mode| mode.category == "GROUNDED" && mode.speed > 0.0));
        assert!(modes
            .iter()
            .all(|mode| !mode.modifiers.contains(&"LIMPING".into())));
        assert!(poses
            .iter()
            .all(|pose| pose.motion_archetype == "QUADRUPED"));

        // Conversely, a restriction on the actual mount applies to the stack.
        spec.features.clear();
        let mount = spec.mount.as_mut().unwrap().spec.as_deref_mut().unwrap();
        mount.features.push("ANCHORED".into());
        let body = crate::anatomy::build(&spec, &mut rng);
        let (animations, modes, attacks, _) = crate::animation::make(&spec, &body);
        assert!(modes
            .iter()
            .all(|mode| mode.category == "STATIONARY" && mode.speed == 0.0));
        assert!(!animations.iter().any(|clip| clip.id == "jump_air"));
        assert!(attacks
            .iter()
            .flat_map(|attack| &attack.steps)
            .all(|step| !["DASH", "LEAP", "DODGE"].contains(&step.primitive.as_str())));
    }

    #[test]
    fn legacy_mount_summaries_and_unvalidated_floating_mounts_are_consistent() {
        let mut fish = crate::parser::parse_vocabulary("goblin riding a fish");
        fish.mount.as_mut().unwrap().spec = None;
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let body = crate::anatomy::build(&fish, &mut rng);
        let (_, modes, _, _) = crate::animation::make(&fish, &body);
        assert_eq!(modes[0].category, "SWIMMING");

        // FLOATING mounts remain unsupported by public generation validation.
        // Lower-level animation still must not invent feet or quadruped jumps.
        let ghost = crate::parser::parse_vocabulary("goblin riding a ghost");
        let output = tempfile::tempdir().unwrap();
        assert!(crate::generate_from_spec(ghost.clone(), 7, output.path()).is_err());
        let body = crate::anatomy::build(&ghost, &mut rng);
        let (_, modes, _, poses) = crate::animation::make(&ghost, &body);
        assert_eq!(modes[0].id, "FLOAT");
        assert_eq!(modes[0].category, "AERIAL");
        assert!(modes[0].modifiers.contains(&"LEVITATING".into()));
        assert!(!modes[0].modifiers.contains(&"JUMP_CAPABLE".into()));
        assert!(poses.iter().all(|pose| pose.motion_archetype == "FLOATING"));
    }

    #[test]
    fn supported_aquatic_mount_exports_through_public_generation() {
        let spec = crate::parser::parse_vocabulary("goblin riding a fish");
        let output = tempfile::tempdir().unwrap();
        let monster = crate::generate_from_spec(spec, 7, output.path()).unwrap();
        assert_eq!(monster.movement_modes[0].category, "SWIMMING");
        assert!(monster.movement_modes[0]
            .modifiers
            .contains(&"UNDULATING".into()));
        let sheet = image::open(output.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        crate::validate(&monster, &sheet).unwrap();
    }

    #[test]
    fn aquatic_float_is_not_flight() {
        let mut spec = crate::parser::parse_vocabulary("ghost");
        spec.features.push("FIN".into());
        let mut modes = vec![MovementMode {
            id: "FLOAT".into(),
            speed: 20.0,
            ..Default::default()
        }];
        describe(&spec, &[], &mut modes);
        assert_eq!(modes[0].category, "SWIMMING");
        assert_eq!(modes[0].modifiers, ["UNDULATING"]);
    }

    #[test]
    fn anchored_creatures_never_advertise_displacement() {
        let mut spec = crate::parser::parse_vocabulary("winged burrowing spider");
        spec.features.push("ANCHORED".into());
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let body = crate::anatomy::build(&spec, &mut rng);
        let (animations, modes, attacks, _) = crate::animation::make(&spec, &body);
        assert!(modes
            .iter()
            .all(|mode| mode.category == "STATIONARY" && mode.speed == 0.0));
        assert!(animations
            .iter()
            .all(|clip| !["FLY", "CLIMB", "BURROWED_MOVE", "JUMP_AIR"]
                .contains(&clip.semantic_state.as_str())));
        assert!(attacks
            .iter()
            .flat_map(|attack| &attack.steps)
            .all(
                |step| !["DASH", "LEAP", "SWOOP", "BURROW", "TELEPORT", "DODGE"]
                    .contains(&step.primitive.as_str())
            ));
    }

    #[test]
    fn descriptors_roundtrip_and_legacy_fields_default() {
        let old = MovementMode::decode(&b"\x0a\x04WALK\x15\0\0\x80\x3f\x1a\x04move"[..]).unwrap();
        assert_eq!(old.id, "WALK");
        assert!(old.category.is_empty());
        assert!(valid_descriptor(&old));
        let json: MovementMode =
            serde_json::from_str(r#"{"id":"WALK","speed":1,"animation_id":"move"}"#).unwrap();
        assert_eq!(json, old);
        let mut mode = old;
        mode.category = "GROUNDED".into();
        mode.modifiers = vec!["BIPEDAL".into()];
        assert_eq!(
            MovementMode::decode(mode.encode_to_vec().as_slice()).unwrap(),
            mode
        );
        #[derive(Clone, PartialEq, Message)]
        struct LegacyMode {
            #[prost(string, tag = "1")]
            id: String,
            #[prost(float, tag = "2")]
            speed: f32,
            #[prost(string, tag = "3")]
            animation_id: String,
        }
        let legacy = LegacyMode::decode(mode.encode_to_vec().as_slice()).unwrap();
        assert_eq!(legacy.id, mode.id);
        assert_eq!(legacy.speed, mode.speed);
        assert_eq!(legacy.animation_id, mode.animation_id);

        let mut cbor = Vec::new();
        ciborium::into_writer(&mode, &mut cbor).unwrap();
        assert_eq!(
            ciborium::from_reader::<MovementMode, _>(cbor.as_slice()).unwrap(),
            mode
        );
        mode.modifiers.push("BIPEDAL".into());
        assert!(!valid_descriptor(&mode));
        mode.modifiers.clear();
        mode.category = "STATIONARY".into();
        assert!(!valid_descriptor(&mode));
        mode.speed = 0.0;
        assert!(valid_descriptor(&mode));
    }
}
