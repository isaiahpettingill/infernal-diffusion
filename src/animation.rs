use crate::{anatomy::Body, parser::MonsterSpec, proto::*};

#[derive(Clone, Debug)]
pub struct Pose {
    pub clip_id: String,
    pub state: String,
    pub phase: f32,
    pub root_x: f32,
    pub root_y: f32,
    pub root_angle: f32,
    pub physical_positions: Vec<(f32, f32, f32)>,
    pub motion_archetype: String,
}

fn clip(
    id: &str,
    state: &str,
    count: u32,
    duration: u32,
    looped: bool,
    movement: &str,
    poses: &mut Vec<Pose>,
) -> Animation {
    let mut frames = Vec::new();
    for i in 0..count {
        let sprite_frame_id = poses.len() as u32;
        poses.push(Pose {
            clip_id: id.into(),
            state: state.into(),
            phase: i as f32 / count as f32,
            root_x: 0.0,
            root_y: 0.0,
            root_angle: 0.0,
            physical_positions: Vec::new(),
            motion_archetype: String::new(),
        });
        frames.push(AnimationFrame {
            sprite_frame_id,
            duration_ms: duration,
            root_dx: if movement.is_empty() { 0.0 } else { 1.5 },
            root_dy: 0.0,
        });
    }
    Animation {
        id: id.into(),
        semantic_state: state.into(),
        frames,
        events: vec![],
        looped,
        movement_mode: movement.into(),
    }
}

fn step(primitive: &str, at_ms: u32, value: f32) -> AttackStep {
    AttackStep {
        primitive: primitive.into(),
        at_ms,
        value,
        ..Default::default()
    }
}

#[allow(clippy::too_many_arguments)]
fn mobility_attack(
    animations: &mut Vec<Animation>,
    poses: &mut Vec<Pose>,
    id: &str,
    motion: &str,
    impact: &str,
    origin: &str,
    damage: u32,
    radius: f32,
    telegraph_ms: u32,
    travel_ms: u32,
) -> Attack {
    let animation_id = format!("attack_{id}");
    animations.push(clip(&animation_id, "ATTACK", 10, 100, false, "", poses));
    let mut movement = step(motion, telegraph_ms, 1.0);
    movement.target = if motion == "DODGE" {
        "AWAY_FROM_PLAYER"
    } else {
        "LAST_PLAYER_POSITION"
    }
    .into();
    movement.speed = match motion {
        "DASH" => 300.0,
        "SWOOP" => 165.0,
        "DODGE" => 220.0,
        _ => 170.0,
    };
    movement.duration_ms = travel_ms;
    let impact_fraction = match motion {
        "SWOOP" => 0.78,
        "TELEPORT" => 0.58,
        _ => 0.88,
    };
    let mut hit = step(
        impact,
        telegraph_ms + (travel_ms as f32 * impact_fraction) as u32,
        1.0,
    );
    let hit_time_ms = hit.at_ms;
    hit.target = "PLAYER".into();
    hit.radius = radius;
    hit.duration_ms = 120;
    let attack = Attack {
        id: id.into(),
        delivery: "MELEE".into(),
        element: "PHYSICAL".into(),
        origin_node: origin.into(),
        damage,
        telegraph_ms,
        cooldown_ms: match motion {
            "TELEPORT" => 24000,
            "BURROW" | "SWOOP" => 3800,
            "LEAP" => 3200,
            _ => 2400,
        },
        range: match motion {
            "DODGE" => 180.0,
            "SWOOP" => 220.0,
            _ => 440.0,
        },
        animation_id,
        steps: vec![
            step("TELEGRAPH", 0, 1.0),
            movement,
            hit,
            step("COOLDOWN", (telegraph_ms + travel_ms + 80).min(990), 1.0),
        ],
        projectile_id: String::new(),
        spawn: None,
    };
    bind_attack(animations, &attack);
    animations
        .iter_mut()
        .find(|clip| clip.id == attack.animation_id)
        .expect("mobility clip exists")
        .events
        .push(AnimationEvent {
            time_ms: hit_time_ms,
            kind: "ATTACK_HIT".into(),
            reference_id: attack.id.clone(),
        });
    attack
}
fn bind_attack(animations: &mut [Animation], attack: &Attack) {
    if let Some(clip) = animations.iter_mut().find(|a| a.id == attack.animation_id) {
        clip.events.push(AnimationEvent {
            time_ms: 0,
            kind: "TELEGRAPH".into(),
            reference_id: attack.id.clone(),
        });
        clip.events.push(AnimationEvent {
            time_ms: attack.telegraph_ms,
            kind: "ATTACK".into(),
            reference_id: attack.id.clone(),
        });
        if ["PROJECTILE", "FIELD", "SUMMON"].contains(&attack.delivery.as_str()) {
            clip.events.push(AnimationEvent {
                time_ms: attack.telegraph_ms,
                kind: match attack.delivery.as_str() {
                    "PROJECTILE" => "PROJECTILE_SPAWN",
                    "FIELD" => "FIELD_SPAWN",
                    _ => "MINION_SPAWN",
                }
                .into(),
                reference_id: attack.id.clone(),
            });
        }
    }
}

pub fn make(
    spec: &MonsterSpec,
    body: &Body,
) -> (Vec<Animation>, Vec<MovementMode>, Vec<Attack>, Vec<Pose>) {
    let mut poses = Vec::new();
    let anchored = spec.features.iter().any(|feature| feature == "ANCHORED");
    let can_fly = !anchored
        && (spec.body_plan == "WINGED"
            || spec.affinity == "DRONE"
            || spec.features.iter().any(|f| f == "WING")
            || spec
                .mount
                .as_ref()
                .is_some_and(|mount| mount.body_plan == "WINGED"));
    let has_special =
        spec.attack.element != "PHYSICAL" || spec.size > 0.8 || spec.affinity == "MUSHROOM";
    let mut animations = vec![clip("idle", "IDLE", 4, 160, true, "", &mut poses)];
    let movement = if let Some(mount) = &spec.mount {
        match mount.body_plan.as_str() {
            "WINGED" => "QUADRUPED_WALK",
            "ARACHNID" | "INSECT" => "MULTILEG_SCUTTLE",
            "SERPENT" if spec.features.iter().any(|f| f == "FIN") => "FLOAT",
            "SERPENT" if mount.affinity == "ORCA" || mount.affinity == "WHALE" => "BEACHED_FLOP",
            "SERPENT" => "SERPENTINE_SLITHER",
            _ => "QUADRUPED_WALK",
        }
    } else if anchored {
        "ANCHORED_SWAY"
    } else if spec.features.iter().any(|f| f == "LIMP") {
        "LIMP"
    } else if spec.features.iter().any(|f| f == "HOP") {
        "HOP"
    } else {
        match spec.body_plan.as_str() {
            "ARACHNID" | "INSECT" => "MULTILEG_SCUTTLE",
            "FUNGUS" => "ROOT_SHUFFLE",
            "TANK" => "TREAD_ROLL",
            "SERPENT" if spec.affinity == "ORCA" || spec.affinity == "WHALE" => "BEACHED_FLOP",
            "SERPENT" => "SERPENTINE_SLITHER",
            "GASTROPOD" if spec.affinity == "SNAIL" => "SNAIL_CRAWL",
            "GASTROPOD" => "GLIDE",
            "QUADRUPED" => "QUADRUPED_WALK",
            "THEROPOD" => "THEROPOD_STRIDE",
            "WINGED" if spec.is_dragon() => "QUADRUPED_WALK",
            "WINGED" if spec.is_bird() => "HOP",
            "WINGED" if spec.affinity == "BAT" => "CRAWL",
            "WINGED" => "BIPED_WALK",
            "AMORPHOUS" if spec.is_blob() => "OOZE_CRAWL",
            "FLOATING" | "AMORPHOUS" => "FLOAT",
            _ => "BIPED_WALK",
        }
    };
    let fast_mode = match movement {
        "BIPED_WALK" => "BIPED_RUN",
        "QUADRUPED_WALK" => "QUADRUPED_RUN",
        "MULTILEG_SCUTTLE" => "FAST_SCUTTLE",
        "SERPENTINE_SLITHER" => "RAPID_SLITHER",
        "BEACHED_FLOP" => "BEACHED_THRASH",
        "THEROPOD_STRIDE" => "THEROPOD_RUN",
        "FLOAT" => "FAST_FLOAT",
        "GLIDE" => "RAPID_GLIDE",
        "SNAIL_CRAWL" => "SNAIL_SURGE",
        "OOZE_CRAWL" => "OOZE_SURGE",
        "LIMP" => "LIMP_FAST",
        "HOP" => "FAST_HOP",
        "ROOT_SHUFFLE" => "ROOT_SURGE",
        "ANCHORED_SWAY" => "ANCHORED_THRASH",
        "TREAD_ROLL" => "TREAD_CHARGE",
        _ => "FAST_MOVE",
    };
    animations.push(clip("move", "MOVE", 8, 90, true, movement, &mut poses));
    animations.push(clip(
        "fast_move",
        "FAST_MOVE",
        8,
        58,
        true,
        fast_mode,
        &mut poses,
    ));
    animations.push(clip("turn", "TURN", 3, 90, false, "", &mut poses));
    if can_fly {
        animations.push(clip("takeoff", "TAKEOFF", 3, 110, false, "", &mut poses));
        animations.push(clip("fly", "FLY", 6, 105, true, "FLY", &mut poses));
        animations.push(clip("fly_turn", "FLY_TURN", 3, 105, false, "", &mut poses));
        animations.push(clip(
            "land_from_flight",
            "LAND_FROM_FLIGHT",
            3,
            110,
            false,
            "",
            &mut poses,
        ));
    } else if ![
        "FLOAT",
        "SERPENTINE_SLITHER",
        "BEACHED_FLOP",
        "GLIDE",
        "SNAIL_CRAWL",
        "OOZE_CRAWL",
    ]
    .contains(&movement)
    {
        animations.push(clip(
            "jump_start",
            "JUMP_START",
            2,
            105,
            false,
            "",
            &mut poses,
        ));
        animations.push(clip("jump_air", "JUMP_AIR", 2, 125, true, "", &mut poses));
        animations.push(clip("land", "LAND", 3, 90, false, "", &mut poses));
    }
    if spec.features.iter().any(|f| f == "CLIMB") || spec.body_plan == "ARACHNID" {
        animations.push(clip("climb", "CLIMB", 6, 115, true, "CLIMB", &mut poses));
    }
    if spec.features.iter().any(|f| f == "BURROW") {
        animations.push(clip("burrow", "BURROW", 4, 110, false, "", &mut poses));
        animations.push(clip(
            "burrowed_move",
            "BURROWED_MOVE",
            4,
            140,
            true,
            "BURROW",
            &mut poses,
        ));
        animations.push(clip("emerge", "EMERGE", 4, 110, false, "", &mut poses));
    }
    animations.push(clip(
        "attack_primary",
        "ATTACK",
        6,
        85,
        false,
        "",
        &mut poses,
    ));
    animations.push(clip(
        "attack_secondary",
        "ATTACK_SECONDARY",
        5,
        85,
        false,
        "",
        &mut poses,
    ));
    if has_special {
        animations.push(clip(
            "attack_special",
            "SPECIAL_ATTACK",
            8,
            90,
            false,
            "",
            &mut poses,
        ));
    }
    if spec.spawn.is_some() {
        animations.push(clip(
            "attack_summon",
            "SPECIAL_ATTACK",
            8,
            100,
            false,
            "",
            &mut poses,
        ));
    }
    for (id, state, n) in [
        ("impact_light_front", "IMPACT_LIGHT", 3),
        ("impact_light_back", "IMPACT_LIGHT", 3),
        ("impact_heavy_front", "IMPACT_HEAVY", 4),
        ("impact_heavy_back", "IMPACT_HEAVY", 4),
        ("knockback", "KNOCKBACK", 4),
        ("stun", "STUN", 3),
        ("death", "DEATH", 8),
    ] {
        animations.push(clip(
            id,
            state,
            n,
            if state == "DEATH" { 125 } else { 95 },
            false,
            "",
            &mut poses,
        ));
    }
    let origin = if spec.affinity == "KRAKEN" && body.nodes.iter().any(|n| n.id == "sea_tip_0") {
        "sea_tip_0".into()
    } else if spec.features.iter().any(|x| x == "EYE_LASER")
        && spec.mount.is_some()
        && body.nodes.iter().any(|n| n.id == "rider_eye_0")
    {
        "rider_eye_0".into()
    } else if spec.mount.is_some() && body.nodes.iter().any(|n| n.id == "rider_weapon") {
        "rider_weapon".into()
    } else if spec.mount.is_some() && body.nodes.iter().any(|n| n.id == "rider_hand_0") {
        "rider_hand_0".into()
    } else if spec.affinity == "MUSHROOM" {
        "spore_sac_0".into()
    } else if spec.features.iter().any(|x| x == "EYE_LASER") {
        "eye_0".into()
    } else if body.nodes.iter().any(|n| n.id == "weapon") {
        "weapon".into()
    } else if ["RHINO", "BULL", "BUFFALO", "RAM"].contains(&spec.affinity.as_str())
        && body.nodes.iter().any(|n| n.id == "horn_0")
    {
        "horn_0".into()
    } else if spec.attack.delivery == "PROJECTILE"
        || spec.attack.concept == "BITE"
        || ["ARACHNID", "INSECT", "QUADRUPED", "SERPENT", "GASTROPOD"]
            .contains(&spec.body_plan.as_str())
    {
        "mouth_0".into()
    } else {
        body.nodes
            .iter()
            .find(|n| n.kind == "HAND")
            .or_else(|| body.nodes.iter().find(|n| n.kind == "FOOT"))
            .map(|n| n.id.clone())
            .unwrap_or("torso".into())
    };
    let mut primary_steps = vec![step("TELEGRAPH", 0, 1.0)];
    let mut action = step(
        if spec.attack.delivery == "PROJECTILE" {
            "SPAWN_PROJECTILE"
        } else if spec.attack.concept == "SPEAR" || origin == "horn_0" {
            "THRUST"
        } else if spec.attack.concept == "CLUB" {
            "SLAM"
        } else {
            "SWIPE"
        },
        255,
        1.0,
    );
    action.target = "PLAYER".into();
    action.count = if spec.attack.concept == "SHOTGUN" {
        5
    } else {
        1
    };
    action.spread_deg = if action.count > 1 { 30.0 } else { 0.0 };
    action.speed = if spec.attack.delivery == "PROJECTILE" {
        150.0
    } else {
        0.0
    };
    action.radius = if spec.attack.delivery == "PROJECTILE" {
        4.0
    } else if spec.affinity == "KRAKEN" {
        95.0
    } else {
        22.0
    };
    action.duration_ms = if spec.attack.delivery == "PROJECTILE" {
        950
    } else {
        85
    };
    primary_steps.push(action);
    primary_steps.push(step("COOLDOWN", 510, 1.0));
    let attack = Attack {
        id: "primary".into(),
        delivery: spec.attack.delivery.clone(),
        element: spec.attack.element.clone(),
        origin_node: origin,
        damage: ((6.0 + spec.size * 15.0) * if spec.affinity == "KRAKEN" { 1.55 } else { 1.0 })
            as u32,
        telegraph_ms: 255,
        cooldown_ms: 950,
        range: if spec.attack.delivery == "PROJECTILE" {
            190.0
        } else if spec.affinity == "KRAKEN" {
            125.0
        } else {
            38.0
        },
        animation_id: "attack_primary".into(),
        steps: primary_steps,
        projectile_id: String::new(),
        spawn: None,
    };
    let secondary_origin = body
        .nodes
        .iter()
        .find(|n| n.kind == "STINGER")
        .or_else(|| body.nodes.iter().find(|n| n.kind == "TAIL"))
        .map(|n| n.id.clone())
        .unwrap_or("mouth_0".into());
    let mut secondary_action = step(
        if secondary_origin == "stinger" {
            "THRUST"
        } else if secondary_origin.starts_with("tail") {
            "SWIPE"
        } else {
            "BITE"
        },
        170,
        1.0,
    );
    secondary_action.target = "PLAYER".into();
    secondary_action.count = 1;
    secondary_action.radius = if secondary_origin.starts_with("tail") {
        44.0
    } else {
        24.0
    };
    secondary_action.duration_ms = 100;
    let secondary = Attack {
        id: "secondary".into(),
        delivery: "MELEE".into(),
        element: if secondary_origin == "stinger" {
            "POISON"
        } else {
            "PHYSICAL"
        }
        .into(),
        origin_node: secondary_origin,
        damage: (5.0 + spec.bulk * 10.0) as u32,
        telegraph_ms: 170,
        cooldown_ms: 1550,
        range: secondary_action.radius,
        animation_id: "attack_secondary".into(),
        steps: vec![
            step("TELEGRAPH", 0, 1.0),
            secondary_action,
            step("COOLDOWN", 425, 1.0),
        ],
        projectile_id: String::new(),
        spawn: None,
    };
    bind_attack(&mut animations, &attack);
    bind_attack(&mut animations, &secondary);
    let mut attacks = vec![attack, secondary];
    if has_special {
        let magical = spec.attack.element != "PHYSICAL";
        let mut action = step(if magical { "SPAWN_FIELD" } else { "SLAM" }, 360, 1.0);
        action.target = if magical {
            "LAST_PLAYER_POSITION"
        } else {
            "PLAYER"
        }
        .into();
        action.count = 1;
        action.radius = if magical { 34.0 } else { 42.0 };
        action.duration_ms = if magical { 1200 } else { 180 };
        let special = Attack {
            id: "special".into(),
            delivery: if magical { "FIELD" } else { "MELEE" }.into(),
            element: spec.attack.element.clone(),
            origin_node: if magical {
                attacks[0].origin_node.clone()
            } else {
                "torso".into()
            },
            damage: (10.0 + spec.size * 16.0) as u32,
            telegraph_ms: 360,
            cooldown_ms: 4200,
            range: if magical { 190.0 } else { 42.0 },
            animation_id: "attack_special".into(),
            steps: vec![
                step("TELEGRAPH", 0, 1.0),
                action,
                step("COOLDOWN", 720, 1.0),
            ],
            projectile_id: String::new(),
            spawn: None,
        };
        bind_attack(&mut animations, &special);
        attacks.push(special);
    }
    if let Some(intent) = &spec.spawn {
        let mut action = step("SPAWN_MINION", 420, 1.0);
        action.target = "NEAR_SELF".into();
        action.count = intent.count as u32;
        action.radius = 28.0;
        action.duration_ms = 1;
        let summon = Attack {
            id: "summon".into(),
            delivery: "SUMMON".into(),
            element: "PHYSICAL".into(),
            origin_node: "torso".into(),
            damage: 0,
            telegraph_ms: 420,
            cooldown_ms: 6200,
            range: 160.0,
            animation_id: "attack_summon".into(),
            steps: vec![
                step("TELEGRAPH", 0, 1.0),
                action,
                step("COOLDOWN", 700, 1.0),
            ],
            projectile_id: String::new(),
            spawn: None,
        };
        bind_attack(&mut animations, &summon);
        attacks.push(summon);
    }
    let can_burrow = spec.features.iter().any(|f| f == "BURROW");
    let spectral = spec.body_plan == "FLOATING"
        || spec.materials.iter().any(|material| material == "SPECTRAL")
        || ["GHOST", "PHANTOM", "BANSHEE"].contains(&spec.affinity.as_str());
    let heavy = spec.size > 0.72 && body.nodes.iter().any(|node| node.kind == "FOOT");
    let mobile = body.nodes.iter().any(|node| node.kind == "FOOT") || spec.body_plan == "SERPENT";
    let mut patterns = Vec::new();
    if can_fly {
        patterns.push(("swoop", "SWOOP", "SWIPE", 74.0, 350, 800));
    }
    if can_burrow {
        patterns.push(("burrow_ambush", "BURROW", "BITE", 84.0, 380, 430));
    }
    if spectral {
        patterns.push(("blink_strike", "TELEPORT", "SWIPE", 67.0, 330, 250));
    }
    if spec.body_plan == "QUADRUPED"
        || spec.body_plan == "TANK"
        || ["BULL", "RHINO", "BUFFALO"].contains(&spec.affinity.as_str())
    {
        patterns.push(("charge", "DASH", "SLAM", 68.0, 300, 440));
    }
    if heavy {
        patterns.push(("leap_slam", "LEAP", "SLAM", 92.0, 380, 490));
    }
    if mobile && spec.size < 0.8 && !can_burrow && !spectral {
        patterns.push(("dodge_counter", "DODGE", "SWIPE", 56.0, 250, 390));
    }
    for (id, motion, impact, radius, telegraph, travel) in patterns.into_iter().take(2) {
        let origin = if body.nodes.iter().any(|node| node.id == "weapon") {
            "weapon"
        } else if body.nodes.iter().any(|node| node.id == "mouth_0") {
            "mouth_0"
        } else {
            "torso"
        };
        attacks.push(mobility_attack(
            &mut animations,
            &mut poses,
            id,
            motion,
            impact,
            origin,
            (8.0 + spec.size * 16.0) as u32,
            radius,
            telegraph,
            travel,
        ));
    }
    for (id, duration) in [("move", 90), ("fast_move", 58)] {
        if [
            "FLOAT",
            "SERPENTINE_SLITHER",
            "GLIDE",
            "SNAIL_CRAWL",
            "OOZE_CRAWL",
        ]
        .contains(&movement)
        {
            continue;
        }
        if let Some(a) = animations.iter_mut().find(|a| a.id == id) {
            let contacts: &[u32] = match movement {
                "QUADRUPED_WALK" | "MULTILEG_SCUTTLE" => &[0, 2, 4, 6],
                _ => &[0, 4],
            };
            for frame in contacts {
                a.events.push(AnimationEvent {
                    time_ms: frame * duration,
                    kind: "FOOT_CONTACT".into(),
                    reference_id: String::new(),
                });
            }
        }
    }
    animations
        .iter_mut()
        .find(|a| a.id == "death")
        .unwrap()
        .events
        .push(AnimationEvent {
            time_ms: 1000,
            kind: "DEATH_COMPLETE".into(),
            reference_id: String::new(),
        });
    let archetype = if spec.mount.is_some() {
        "MOUNTED"
    } else if spec.affinity == "SAND_WORM" {
        "SAND_WORM"
    } else if spec.features.iter().any(|f| f == "WHIRLWIND") {
        "WHIRLWIND"
    } else if spec.features.iter().any(|f| f == "FIN") {
        "AQUATIC"
    } else if spec.affinity == "CENTIPEDE" {
        "CENTIPEDE"
    } else if spec.affinity == "DRONE" {
        "DRONE"
    } else if spec.body_plan == "FUNGUS" {
        "FUNGUS"
    } else if spec.body_plan == "TANK" {
        "TANK"
    } else if spec.is_dragon() {
        "DRAGON"
    } else if spec.is_blob() {
        "BLOB"
    } else {
        spec.body_plan.as_str()
    };
    for pose in &mut poses {
        pose.motion_archetype = archetype.into();
    }
    crate::physics::apply_simulated_reactions(body, &mut poses);
    let mut modes = vec![
        MovementMode {
            id: movement.into(),
            speed: (28.0 + spec.size * 15.0)
                * if anchored { 0.0 } else { 1.0 }
                * if spec.affinity == "ORCA" || spec.affinity == "WHALE" {
                    0.18
                } else if spec.affinity == "SNAIL" {
                    0.35
                } else if spec.is_gastropod() {
                    0.7
                } else {
                    1.0
                },
            animation_id: "move".into(),
        },
        MovementMode {
            id: fast_mode.into(),
            speed: (55.0 + spec.size * 22.0)
                * if anchored { 0.0 } else { 1.0 }
                * if spec.affinity == "ORCA" || spec.affinity == "WHALE" {
                    0.25
                } else if spec.affinity == "SNAIL" {
                    0.4
                } else if spec.is_gastropod() {
                    0.75
                } else {
                    1.0
                },
            animation_id: "fast_move".into(),
        },
    ];
    if can_fly {
        modes.push(MovementMode {
            id: "FLY".into(),
            speed: 65.0 + spec.size * 20.0,
            animation_id: "fly".into(),
        });
    }
    if animations.iter().any(|a| a.id == "climb") {
        modes.push(MovementMode {
            id: "CLIMB".into(),
            speed: 25.0,
            animation_id: "climb".into(),
        });
    }
    if animations.iter().any(|a| a.id == "burrowed_move") {
        modes.push(MovementMode {
            id: "BURROW".into(),
            speed: 23.0,
            animation_id: "burrowed_move".into(),
        });
    }
    for mode in &modes {
        if let Some(clip) = animations.iter_mut().find(|a| a.id == mode.animation_id) {
            for frame in &mut clip.frames {
                let distance = mode.speed * frame.duration_ms as f32 / 1000.0;
                if mode.id == "CLIMB" {
                    frame.root_dy = -distance;
                    frame.root_dx = 0.0;
                } else {
                    frame.root_dx = distance;
                }
            }
        }
    }
    (animations, modes, attacks, poses)
}

pub fn node_offset(
    kind: &str,
    id: &str,
    _index: usize,
    pose: &Pose,
    gravity: f32,
) -> (f32, f32, f32) {
    let t = pose.phase * std::f32::consts::TAU;
    let ordinal = id
        .rsplit('_')
        .next()
        .and_then(|part| part.parse::<usize>().ok())
        .unwrap_or(0);
    let side_phase = if ordinal.is_multiple_of(2) {
        0.0
    } else {
        std::f32::consts::PI
    };
    let gait = (t + side_phase + (ordinal / 2) as f32 * 0.7).sin();
    let pace = if pose.state == "FAST_MOVE" { 1.3 } else { 1.0 };
    let mut x = 0.0;
    let mut y = 0.0;
    let mut angle = 0.0;
    match pose.state.as_str() {
        "IDLE" => {
            y = t.sin()
                * if pose.motion_archetype == "BLOB" {
                    1.2
                } else {
                    0.45
                };
            if matches!(kind, "HEAD" | "NECK" | "TENTACLE") {
                angle = (t + ordinal as f32 * 0.8).sin() * 0.055;
            }
        }
        "MOVE" | "FAST_MOVE" | "FLY" | "CLIMB" => {
            match pose.motion_archetype.as_str() {
                "SERPENT" | "SAND_WORM" | "AQUATIC" => {
                    let wave = (t - ordinal as f32 * 0.72).sin();
                    if kind == "TAIL" {
                        x = wave * 2.4 * pace;
                        y = wave * 3.2 * pace;
                        angle = wave * 0.14;
                    } else if matches!(kind, "TORSO" | "HEAD" | "NECK") {
                        x = (t + 0.8).sin() * 1.1 * pace;
                        y = (t + 0.8).cos() * 1.4 * pace;
                        angle = t.sin() * 0.045;
                    }
                }
                "GASTROPOD" => {
                    if kind == "SOLE" {
                        x = t.sin() * 2.2 * pace;
                        y = -t.cos().max(0.0) * 1.2;
                    } else if kind == "TENTACLE" || id.starts_with("stalk_eye") {
                        x = (t + ordinal as f32).sin() * 1.7;
                        angle = t.sin() * 0.13;
                    } else if matches!(kind, "HEAD" | "NECK" | "MOUTH") {
                        x = t.sin() * 2.7 * pace;
                        y = -t.cos().max(0.0) * 0.9;
                    } else if kind == "TORSO" {
                        x = t.sin() * 0.8;
                        y = t.cos() * 0.6;
                    } else if kind == "SHELL" {
                        y = t.cos() * 0.35;
                    }
                }
                "BLOB" => {
                    if kind == "LOBE" {
                        y = (t + ordinal as f32 * 0.9).sin() * 1.6;
                        x = (t + ordinal as f32).cos() * 0.6;
                    } else {
                        y = t.sin().abs() * -1.4;
                    }
                }
                "FLOATING" | "WHIRLWIND" => {
                    y = t.sin() * 1.8;
                    if kind == "TENTACLE" {
                        x = (t + ordinal as f32).sin() * 1.4;
                    } else if kind == "TAIL" {
                        x = (t - ordinal as f32 * 0.65).sin() * 2.0 * pace;
                        angle = t.sin() * 0.12;
                    }
                }
                "FUNGUS" => {
                    if kind == "ROOT" {
                        x = gait * 1.2;
                        y = -gait.max(0.0) * 0.9;
                    } else if kind == "HEAD" {
                        angle = t.sin() * 0.08;
                    } else if kind == "TORSO" {
                        y = t.sin().abs() * -0.6;
                    }
                }
                "TANK" => {
                    if kind == "TREAD" {
                        x = t.sin() * 0.5;
                    } else if kind == "HEAD" {
                        angle = t.sin() * 0.04;
                    }
                }
                "DRONE" => {
                    y = t.sin() * 1.5;
                    if kind == "ROTOR" {
                        angle = t * 2.0;
                    }
                }
                "ARACHNID" | "INSECT" | "CENTIPEDE" => {
                    let leg_phase = if pose.motion_archetype == "CENTIPEDE" {
                        t + (ordinal / 2) as f32 * 0.8 + side_phase
                    } else if pose.motion_archetype == "ARACHNID" {
                        let group = (ordinal % 2 + usize::from(ordinal >= 4)) % 2;
                        t + group as f32 * std::f32::consts::PI + (ordinal % 4) as f32 * 0.18
                    } else {
                        t + (ordinal % 3) as f32 * std::f32::consts::PI
                    };
                    let step = leg_phase.sin();
                    let lift = leg_phase.cos().max(0.0);
                    match kind {
                        "HIP" => x = step * 0.35 * pace,
                        "LIMB" => {
                            x = step * 2.8 * pace;
                            y = -lift * 1.0;
                            angle = step * 0.12;
                        }
                        "SHIN" => {
                            x = step * 4.0 * pace;
                            y = -lift * 2.0;
                        }
                        "FOOT" => {
                            x = step * 5.8 * pace;
                            y = -lift * 3.8 * pace;
                        }
                        "TORSO" => y = -(2.0 * t).cos().abs() * 0.7,
                        "HEAD" | "NECK" => angle = t.sin() * 0.04,
                        _ => {}
                    }
                }
                "MOUNTED" if id.starts_with("rider_") => {
                    y = -(2.0 * t).cos().abs() * 1.2;
                    if kind == "ARM" || kind == "HAND" {
                        x = t.sin() * 1.0;
                    }
                }
                "QUADRUPED" | "DRAGON" | "MOUNTED" => {
                    let phase = if pose.state == "FAST_MOVE" {
                        [0.0, 0.35, std::f32::consts::PI, std::f32::consts::PI + 0.35][ordinal % 4]
                    } else {
                        [
                            0.0,
                            std::f32::consts::PI,
                            std::f32::consts::FRAC_PI_2,
                            3.0 * std::f32::consts::FRAC_PI_2,
                        ][ordinal % 4]
                    };
                    let step = (t + phase).sin();
                    let lift = (t + phase).cos().max(0.0);
                    match kind {
                        "HIP" => x = step * 0.45,
                        "LIMB" => {
                            x = step * 2.7 * pace;
                            y = -lift * 1.0;
                            angle = step * 0.1;
                        }
                        "SHIN" => {
                            x = step * 4.3 * pace;
                            y = -lift * 2.0;
                        }
                        "FOOT" => {
                            x = step * 6.3 * pace;
                            y = -lift * 4.0 * pace;
                        }
                        "TORSO" => {
                            y = -(2.0 * t).cos().abs() * 1.0;
                            angle = t.sin() * 0.025;
                        }
                        "HEAD" | "NECK" => {
                            y = -(2.0 * t).cos().abs() * 0.5;
                            angle = t.sin() * 0.065;
                        }
                        "TAIL" => y = (t - ordinal as f32 * 0.35).sin() * 1.5,
                        _ => {}
                    }
                }
                _ => {
                    let limb_phase = t + side_phase + 0.3 + (ordinal / 2) as f32 * 0.25;
                    let step = limb_phase.sin();
                    let lift = limb_phase.cos().max(0.0);
                    match kind {
                        "HIP" => x = step * 0.45,
                        "LIMB" => {
                            x = step * 2.6 * pace;
                            y = -lift * 0.9;
                            angle = step * 0.12;
                        }
                        "SHIN" => {
                            x = step * 4.2 * pace;
                            y = -lift * 2.0;
                        }
                        "FOOT" => {
                            x = step * 6.2 * pace;
                            y = -lift * 4.0 * pace;
                        }
                        "ARM" => {
                            x = -step * 2.8 * pace;
                            angle = -step * 0.12;
                        }
                        "HAND" => {
                            x = -step * 4.0 * pace;
                            y = step.abs() * 0.7;
                        }
                        "TAIL" => y = (t + ordinal as f32 * 0.4).sin() * 1.4,
                        "TORSO" => {
                            y = -(2.0 * t).cos().abs() * 1.1;
                            angle = t.sin() * 0.035;
                        }
                        "HEAD" | "NECK" => {
                            y = -(2.0 * t).cos().abs() * 0.8;
                            angle = t.sin() * 0.055;
                        }
                        _ => {}
                    }
                }
            }
            if kind == "WING" {
                let wing_phase = (t + 0.3).sin();
                y = wing_phase * if pose.state == "FLY" { 5.2 } else { 2.0 };
                angle = wing_phase * 0.45 * if ordinal.is_multiple_of(2) { 1.0 } else { -1.0 };
            }
            if pose.state == "FLY" {
                y -= 3.5;
            }
        }
        "ATTACK" | "ATTACK_SECONDARY" | "SPECIAL_ATTACK" => {
            if ["HEAD", "MOUTH", "WEAPON", "FOOT", "ARM", "HAND"].contains(&kind) {
                x = (t.sin().max(0.0)) * 4.0;
            }
        }
        "IMPACT_LIGHT" => {
            x = -2.0 * (1.0 - pose.phase);
            angle = -0.1 * (1.0 - pose.phase);
        }
        "IMPACT_HEAVY" | "KNOCKBACK" => {
            x = -5.0 * (1.0 - pose.phase);
            angle = -0.27 * (1.0 - pose.phase);
        }
        "STUN" => {
            angle = t.sin() * 0.08;
            y = t.sin().abs();
        }
        "DEATH" => {
            let p = pose.phase;
            y = p * 3.0 * gravity;
            angle = p * 0.15;
            if kind == "LIMB" || kind == "FOOT" {
                x = gait * 2.0 * p;
            }
            if gravity == 0.0 {
                y = p * 2.0;
            }
        }
        "BURROW" => {
            y = pose.phase
                * if pose.motion_archetype == "SAND_WORM" {
                    35.0
                } else {
                    10.0
                };
        }
        "BURROWED_MOVE" => {
            y = if pose.motion_archetype == "SAND_WORM" {
                35.0
            } else {
                10.0
            } + t.sin() * 0.4;
        }
        "EMERGE" => {
            y = (1.0 - pose.phase)
                * if pose.motion_archetype == "SAND_WORM" {
                    35.0
                } else {
                    10.0
                };
        }
        "TAKEOFF" | "JUMP_START" => {
            y = -pose.phase * 5.0;
        }
        "JUMP_AIR" => {
            y = -5.0;
        }
        "LAND" | "LAND_FROM_FLIGHT" => {
            y = (1.0 - pose.phase) * -4.0;
        }
        _ => {}
    }
    match pose.clip_id.as_str() {
        "attack_swoop" => {
            y -= (pose.phase * std::f32::consts::PI).sin() * 4.0;
            if kind == "WING" {
                angle += t.sin() * 0.45;
            }
        }
        "attack_leap_slam" => y -= (pose.phase * std::f32::consts::PI).sin() * 6.0,
        "attack_charge" if matches!(kind, "HEAD" | "NECK" | "TORSO") => {
            x += pose.phase * 3.0;
            angle -= 0.08;
        }
        "attack_dodge_counter" => x -= (pose.phase * std::f32::consts::PI).sin() * 3.0,
        "attack_burrow_ambush" => y += (pose.phase * std::f32::consts::PI).sin() * 7.0,
        "attack_blink_strike" => x += if pose.phase < 0.5 { -2.0 } else { 2.0 },
        _ => {}
    }
    if kind == "VORTEX" {
        x += (t + ordinal as f32 * 1.4).sin() * (1.2 + ordinal as f32 * 0.3);
        y += (t + ordinal as f32 * 0.8).cos() * 0.5;
        angle += (t + ordinal as f32).sin() * 0.17;
    }
    (x, y, angle)
}

#[cfg(test)]
mod mobility_tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    #[test]
    fn movement_attacks_follow_generated_anatomy() {
        for (prompt, expected) in [
            ("dragon", "SWOOP"),
            ("sand worm", "BURROW"),
            ("ghost", "TELEPORT"),
            ("rhino", "DASH"),
            ("giant troll", "LEAP"),
            ("spider", "DODGE"),
        ] {
            let spec = crate::parser::parse_vocabulary(prompt);
            let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(4));
            let (clips, _, attacks, _) = make(&spec, &body);
            let attack = attacks
                .iter()
                .find(|attack| attack.steps.iter().any(|step| step.primitive == expected))
                .unwrap_or_else(|| panic!("{prompt} lacks {expected}"));
            let clip = clips
                .iter()
                .find(|clip| clip.id == attack.animation_id)
                .unwrap();
            assert!(clip.events.iter().any(|event| {
                event.kind == "ATTACK"
                    && event.reference_id == attack.id
                    && event.time_ms == attack.telegraph_ms
            }));
            assert!(body.nodes.iter().any(|node| node.id == attack.origin_node));
            let movement = attack
                .steps
                .iter()
                .find(|step| step.primitive == expected)
                .unwrap();
            let hit = attack
                .steps
                .iter()
                .find(|step| ["SWIPE", "BITE", "THRUST", "SLAM"].contains(&step.primitive.as_str()))
                .unwrap();
            assert!(hit.at_ms > movement.at_ms);
            assert!(hit.at_ms < movement.at_ms + movement.duration_ms);
            assert!(clip.events.iter().any(|event| {
                event.kind == "ATTACK_HIT"
                    && event.reference_id == attack.id
                    && event.time_ms == hit.at_ms
            }));
        }
    }
}
