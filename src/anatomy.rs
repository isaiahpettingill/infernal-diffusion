use crate::parser::MonsterSpec;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Debug)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub parent: Option<usize>,
    pub x: f32,
    pub y: f32,
    pub rx: f32,
    pub ry: f32,
    pub angle: f32,
    pub material: String,
    pub density: f32,
    pub feature: bool,
}
#[derive(Clone, Debug)]
pub struct Body {
    pub nodes: Vec<Node>,
    pub mass: f32,
    pub com: (f32, f32),
    pub gravity: f32,
    pub repairs: Vec<String>,
}

#[allow(clippy::too_many_arguments)]
fn add(
    nodes: &mut Vec<Node>,
    id: String,
    kind: &str,
    parent: Option<usize>,
    x: f32,
    y: f32,
    rx: f32,
    ry: f32,
    angle: f32,
    material: &str,
    feature: bool,
) -> usize {
    let density = match material {
        "BONE" => 1.35,
        "METAL" => 2.5,
        "STONE" => 2.0,
        "SPECTRAL" | "FIRE" | "SMOKE" => 0.12,
        material if material.starts_with("FEATHER") => 0.35,
        _ => 1.0,
    };
    nodes.push(Node {
        id,
        kind: kind.into(),
        parent,
        x,
        y,
        rx,
        ry,
        angle,
        material: material.into(),
        density,
        feature,
    });
    nodes.len() - 1
}

pub fn morphology_scale(size: f32) -> f32 {
    0.55 + size.clamp(0.0, 1.0) * 1.2
}

pub fn material_for_affinity(affinity: &str) -> &'static str {
    match affinity {
        "WOLF" | "JACKAL" | "DOG" | "BEAR" | "MOOSE" | "BUFFALO" | "LION" | "TIGER" | "LEOPARD"
        | "CHEETAH" | "PANTHER" | "HYENA" | "YETI" | "BIGFOOT" | "GORILLA" | "KELPIE" => "FUR",
        "MAMMOTH" => "FUR_BROWN",
        "SABERTOOTH_CAT" => "FUR_GOLD",
        "SAND_WORM" => "SAND_HIDE",
        "SHARK" => "SHARK_HIDE",
        "ORCA" => "ORCA_HIDE",
        "SHARKNADO" | "TORNADO" => "STORM",
        "JELLYFISH" => "MEMBRANE",
        "SERPENT" | "SNAKE" | "COBRA" | "DRAGON" | "WYVERN" | "SEA_DRAGON" | "SEA_SERPENT"
        | "FISH" | "EEL" | "WHALE" | "LONGNECK_DINO" | "STEGO" | "TRICERATOPS" | "ANKYLOSAUR"
        | "TREX" | "RAPTOR" | "QUETZALCOATL" | "CIPACTLI" | "CROCODILE" | "NAGA" | "GROOTSLANG" => {
            "SCALES"
        }
        "SPIDER" | "SCORPION" | "HORNET" | "CENTIPEDE" | "SCARAB" | "BEETLE" => "CHITIN",
        "BAT" => "FUR_DARK",
        "RABBIT" | "KILLER_RABBIT" => "FUR_WHITE",
        "KRAKEN" => "DEEP_SEA",
        "RAVEN" | "CROW" | "CONDOR" => "FEATHER_BLACK",
        "SIREN" | "HARPY" => "FEATHER_BROWN",
        "HAWK" | "OWL" | "VULTURE" | "HERON" => "FEATHER_BROWN",
        "EAGLE" | "ALBATROSS" | "PELICAN" => "FEATHER_WHITE",
        "HUMMINGBIRD" => "FEATHER_GOLD",
        "FALCON" | "IBIS" | "BENNU" | "STRIX" | "BIRD" | "PIGEON" | "SPARROW" => "FEATHER",
        "MUSHROOM" => "FUNGUS",
        "TANK" | "DRONE" | "ROBOT" => "METAL",
        "GHOST" | "PHANTOM" | "SPECTRAL" | "BANSHEE" | "LEMURE" => "SPECTRAL",
        "SKELETON" | "TZITZIMITL" => "BONE",
        "ZOMBIE" | "DRAUGR" | "PISACHA" => "ROTTEN",
        "ALIEN" => "ALIEN_SKIN",
        _ => "FLESH",
    }
}

fn apply_part_scales(spec: &MonsterSpec, nodes: &mut [Node]) {
    if spec.part_scales.is_empty() {
        return;
    }
    let original = nodes.to_vec();
    for (index, node) in nodes.iter_mut().enumerate() {
        let original_node = &original[index];
        let number = node
            .id
            .rsplit('_')
            .next()
            .and_then(|value| value.parse::<usize>().ok());
        let (whole, first) = match node.kind.as_str() {
            "HEAD" => ("HEAD", "HEAD_0"),
            "SHOULDER" | "ARM" | "HAND" => ("ARM", "ARM_0"),
            "HIP" | "LIMB" | "SHIN" | "FOOT" => ("LEG", "LEG_0"),
            "WING" => ("WING", "WING_0"),
            "TAIL" => ("TAIL", "TAIL"),
            "HORN" | "ANTLER" => ("HORN", "HORN_0"),
            "EYE" => ("EYE", "EYE_0"),
            "CLAW" => ("CLAW", "CLAW"),
            "WEAPON" => ("WEAPON", "WEAPON"),
            _ => continue,
        };
        let factor = if number == Some(0) {
            spec.part_scales
                .get(first)
                .or_else(|| spec.part_scales.get(whole))
        } else {
            spec.part_scales.get(whole)
        }
        .copied()
        .unwrap_or(1.0);
        node.rx *= factor;
        node.ry *= factor;
        if matches!(node.kind.as_str(), "ARM" | "HAND") {
            if let Some(number) = number {
                if let Some(shoulder) = original
                    .iter()
                    .find(|candidate| candidate.id == format!("shoulder_{number}"))
                {
                    node.x = shoulder.x + (original_node.x - shoulder.x) * factor;
                    node.y = shoulder.y + (original_node.y - shoulder.y) * factor;
                }
            }
        }
    }
}

fn build_mounted(spec: &MonsterSpec, rng: &mut ChaCha8Rng) -> Body {
    let mount = spec.mount.as_ref().expect("mounted spec");
    let mount_spec = crate::parser::mount_spec(spec, mount);
    let mut body = build(&mount_spec, rng);
    for node in &mut body.nodes {
        if node.id.starts_with("rider_") || node.id.starts_with("mount_rider_") {
            node.id = format!("mount_{}", node.id);
        }
    }
    let mut rider_spec = spec.clone();
    rider_spec.mount = None;
    rider_spec.spawn = None;
    rider_spec.secondary_affinities.clear();
    rider_spec.size = if ["GOBLIN", "TOKOLOSHE"].contains(&spec.affinity.as_str()) {
        0.26
    } else {
        (spec.size * 0.75).clamp(0.25, 0.72)
    };
    if rider_spec.body_plan == "HUMANOID" {
        rider_spec.limb_count = 2;
    }
    let rider = build(&rider_spec, rng);
    let offset = body.nodes.len();
    let mount_torso = body.nodes.iter().position(|n| n.id == "torso").unwrap_or(0);
    let vertical_shift = if mount_spec.mount.is_some() {
        -22.0
    } else {
        -13.0
    } * morphology_scale(mount_spec.size);
    for mut node in rider.nodes {
        node.id = format!("rider_{}", node.id);
        node.x -= 2.0 * morphology_scale(mount_spec.size);
        node.y += vertical_shift;
        node.parent = Some(node.parent.map_or(mount_torso, |parent| parent + offset));
        body.nodes.push(node);
    }
    let total_mass = body.mass + rider.mass;
    body.com = (
        (body.com.0 * body.mass + (rider.com.0 - 2.0) * rider.mass) / total_mass,
        (body.com.1 * body.mass + (rider.com.1 + vertical_shift) * rider.mass) / total_mass,
    );
    body.mass = total_mass;
    body.repairs.extend(rider.repairs);
    body.repairs.sort();
    body.repairs.dedup();
    body
}

pub fn build(spec: &MonsterSpec, rng: &mut ChaCha8Rng) -> Body {
    if spec.mount.is_some() {
        return build_mounted(spec, rng);
    }
    let mut nodes = Vec::new();
    let scale = morphology_scale(spec.size);
    let primary = &spec.materials[0];
    let arachnid = matches!(spec.body_plan.as_str(), "ARACHNID" | "INSECT");
    let quadruped = spec.body_plan == "QUADRUPED";
    let dragon = spec.is_dragon();
    let theropod = spec.body_plan == "THEROPOD";
    let longneck = spec.affinity == "LONGNECK_DINO";
    let avian = spec.is_bird();
    let bat = spec.affinity == "BAT";
    let winged_animal = avian || bat;
    let serpent = spec.body_plan == "SERPENT";
    let gastropod = spec.is_gastropod();
    let blob = spec.is_blob();
    let horizontal = arachnid
        || quadruped
        || dragon
        || theropod
        || serpent
        || gastropod
        || winged_animal
        || spec.body_plan == "TANK";
    let crocodile = spec.affinity == "CROCODILE";
    let torso = add(
        &mut nodes,
        "torso".into(),
        "TORSO",
        None,
        if arachnid || dragon {
            -3.0 * scale
        } else {
            0.0
        },
        if blob || gastropod {
            3.0 * scale
        } else {
            -2.0 * scale
        },
        if spec.affinity == "KRAKEN" {
            11.0 * scale
        } else if longneck {
            16.0 * scale
        } else if matches!(
            spec.affinity.as_str(),
            "TREX" | "STEGO" | "TRICERATOPS" | "ANKYLOSAUR" | "MAMMOTH"
        ) {
            14.0 * scale
        } else if spec.affinity == "TREE_MONSTER" {
            6.5 * scale
        } else if spec.body_plan == "TANK" || spec.affinity == "SAND_WORM" {
            15.0 * scale
        } else if spec.features.iter().any(|feature| feature == "FIN") {
            13.0 * scale
        } else if gastropod {
            14.0 * scale
        } else if horizontal {
            11.0 * scale
        } else {
            8.5 * scale
        },
        if spec.affinity == "KRAKEN" {
            6.5 * scale
        } else if matches!(spec.affinity.as_str(), "TREX" | "MAMMOTH" | "LONGNECK_DINO") {
            7.0 * scale
        } else if spec.affinity == "TREE_MONSTER" {
            15.0 * scale
        } else if spec.body_plan == "TANK" {
            5.0 * scale
        } else if blob {
            8.0 * scale
        } else if gastropod {
            4.8 * scale
        } else if horizontal {
            5.5 * scale
        } else {
            8.0 * scale
        },
        0.0,
        primary,
        false,
    );
    let heads = spec.heads.min(5);
    for h in 0..heads {
        let side_rank = h as f32 - (heads as f32 - 1.0) / 2.0;
        let spread = side_rank
            * if spec.affinity == "HYDRA" {
                5.5
            } else if heads > 1 {
                3.5
            } else {
                0.0
            }
            * scale;
        let (neck_x, neck_y, head_x, head_y) = if spec.body_plan == "TANK" {
            (1.0, -7.0, 4.0, -9.0)
        } else if spec.affinity == "TREE_MONSTER" {
            (0.0, -15.0, 1.5, -23.0)
        } else if spec.affinity == "KRAKEN" {
            (0.0, -6.0, 0.0, -10.0)
        } else if spec.body_plan == "FUNGUS" {
            (0.0, -9.0, 0.0, -13.0)
        } else if spec.affinity == "JAR_JAR" {
            (1.5, -11.0, 3.5, -18.0)
        } else if longneck {
            (11.0, -12.0, 14.0, -27.0)
        } else if spec.affinity == "GIRAFFE" {
            (8.0, -8.0, 11.0, -18.0)
        } else if theropod {
            (7.0, -8.0, 14.0, -9.0)
        } else if dragon {
            (8.0, -5.0, 14.0, -7.0)
        } else if spec.affinity == "HYDRA" {
            (4.5, -8.0, 8.0, -15.0)
        } else if matches!(spec.affinity.as_str(), "HERON" | "IBIS" | "PELICAN") {
            (5.0, -8.0, 10.0, -13.0)
        } else if winged_animal {
            (6.0, -4.5, 10.0, -7.0)
        } else if serpent {
            (7.0, -4.0, 12.0, -5.5)
        } else if gastropod {
            (8.0, -1.0, 12.0, -4.0)
        } else if blob {
            (2.0, -3.0, 4.0, -5.0)
        } else if horizontal {
            (7.0, -2.5, 11.0, -3.0)
        } else {
            (1.5, -9.0, 2.5, -15.0)
        };
        let head_scale = if heads > 1 { 0.78 } else { 1.0 };
        let neck = add(
            &mut nodes,
            format!("neck_{h}"),
            "NECK",
            Some(torso),
            (neck_x + spread) * scale,
            (neck_y
                + if spec.affinity == "HYDRA" {
                    side_rank.abs() * 1.0
                } else {
                    0.0
                })
                * scale,
            if blob { 2.0 * scale } else { 3.0 * scale },
            if longneck {
                10.0 * scale
            } else if spec.affinity == "GIRAFFE" {
                7.0 * scale
            } else if spec.affinity == "HYDRA" {
                5.5 * scale
            } else if horizontal {
                2.4 * scale
            } else {
                4.0 * scale
            },
            -0.3,
            primary,
            false,
        );
        let head = add(
            &mut nodes,
            format!("head_{h}"),
            "HEAD",
            Some(neck),
            (head_x + spread) * scale,
            (head_y
                + if spec.affinity == "HYDRA" {
                    side_rank.abs() * 1.7
                } else {
                    0.0
                })
                * scale,
            (if spec.affinity == "TREE_MONSTER" {
                8.0
            } else if spec.affinity == "KRAKEN" {
                10.0
            } else if spec.body_plan == "FUNGUS" {
                9.5
            } else if spec.affinity == "JAR_JAR" {
                4.5
            } else if spec.body_plan == "TANK" {
                5.0
            } else if crocodile || spec.affinity == "TREX" {
                7.0
            } else if spec.features.iter().any(|f| f == "LARGE_HEAD") {
                7.5
            } else if spec.affinity == "OWL" {
                6.4
            } else if winged_animal {
                4.8
            } else if blob {
                3.5
            } else if arachnid {
                4.0
            } else {
                5.2
            }) * scale
                * head_scale,
            (if spec.affinity == "TREE_MONSTER" {
                7.0
            } else if spec.affinity == "KRAKEN" {
                8.0
            } else if spec.body_plan == "FUNGUS" {
                3.0
            } else if spec.affinity == "JAR_JAR" {
                6.5
            } else if spec.body_plan == "TANK" {
                3.5
            } else if crocodile || spec.affinity == "TREX" {
                2.6
            } else if spec.affinity == "OWL" {
                5.0
            } else if winged_animal {
                3.5
            } else if blob || horizontal {
                3.0
            } else {
                4.5
            }) * scale
                * head_scale,
            -0.1,
            primary,
            false,
        );
        add(
            &mut nodes,
            format!("eye_{h}"),
            "EYE",
            Some(head),
            (head_x + spread + if horizontal { 2.0 } else { 1.5 }) * scale,
            (head_y - 1.1
                + if spec.affinity == "HYDRA" {
                    side_rank.abs() * 1.7
                } else {
                    0.0
                })
                * scale,
            1.1 * scale,
            1.1 * scale,
            0.0,
            "ENERGY",
            true,
        );
        add(
            &mut nodes,
            format!("mouth_{h}"),
            "MOUTH",
            Some(head),
            (head_x
                + spread
                + if crocodile {
                    6.0
                } else if horizontal {
                    4.0
                } else {
                    3.1
                })
                * scale,
            (head_y
                + if horizontal { 2.4 } else { 2.3 }
                + if spec.affinity == "HYDRA" {
                    side_rank.abs() * 1.7
                } else {
                    0.0
                })
                * scale,
            2.2 * scale,
            1.0 * scale,
            0.0,
            "BONE",
            true,
        );
        if arachnid && spec.affinity != "CENTIPEDE" {
            for eye in 1..4 {
                add(
                    &mut nodes,
                    format!("eye_{h}_{eye}"),
                    "EYE",
                    Some(head),
                    (10.0 + eye as f32 * 1.2) * scale,
                    (-5.7 + (eye % 2) as f32 * 2.2) * scale,
                    0.7 * scale,
                    0.7 * scale,
                    0.0,
                    "ENERGY",
                    true,
                );
            }
        }
        if spec.features.iter().any(|x| x == "HORN")
            || ["MINOTAUR", "DEMONIC"].contains(&spec.affinity.as_str())
        {
            add(
                &mut nodes,
                format!("horn_{h}"),
                "HORN",
                Some(head),
                (head_x + spread - 2.0) * scale,
                (head_y - 4.5) * scale,
                1.8 * scale,
                4.0 * scale,
                -0.45,
                "HORN",
                true,
            );
        }
    }
    if bat {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            for i in 0..2 {
                add(
                    &mut nodes,
                    format!("ear_{i}"),
                    "HORN",
                    Some(head),
                    (7.5 + i as f32 * 3.5) * scale,
                    -11.5 * scale,
                    1.5 * scale,
                    3.5 * scale,
                    0.0,
                    primary,
                    true,
                );
            }
        }
    }
    if spec.affinity == "OWL" {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            for i in 0..2 {
                add(
                    &mut nodes,
                    format!("owl_tuft_{i}"),
                    "HORN",
                    Some(head),
                    (8.0 + i as f32 * 4.0) * scale,
                    -12.0 * scale,
                    1.4 * scale,
                    3.0 * scale,
                    0.0,
                    primary,
                    true,
                );
            }
        }
    }
    if spec.features.iter().any(|feature| feature == "LONG_EARS") {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            let jar = spec.affinity == "JAR_JAR";
            for i in 0..2 {
                add(
                    &mut nodes,
                    format!("long_ear_{i}"),
                    "EAR",
                    Some(head),
                    (if jar {
                        -1.0 + i as f32 * 8.0
                    } else {
                        8.0 + i as f32 * 4.0
                    }) * scale,
                    (if jar { -19.0 } else { -13.0 }) * scale,
                    (if jar { 2.0 } else { 2.3 }) * scale,
                    (if jar { 8.0 } else { 7.0 }) * scale,
                    if jar {
                        -0.55 + i as f32 * 1.1
                    } else {
                        -0.16 + i as f32 * 0.32
                    },
                    primary,
                    true,
                );
            }
        }
    }
    if spec.affinity == "JAR_JAR" {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            for i in 0..2 {
                let stalk = add(
                    &mut nodes,
                    format!("gungan_eye_stalk_{i}"),
                    "TENTACLE",
                    Some(head),
                    (1.0 + i as f32 * 4.0) * scale,
                    -24.0 * scale,
                    0.8 * scale,
                    4.0 * scale,
                    0.0,
                    primary,
                    false,
                );
                add(
                    &mut nodes,
                    format!("gungan_eye_tip_{i}"),
                    "LOBE",
                    Some(stalk),
                    (1.0 + i as f32 * 4.0) * scale,
                    -27.0 * scale,
                    1.4 * scale,
                    1.2 * scale,
                    0.0,
                    "ENERGY",
                    true,
                );
            }
        }
    }
    if matches!(spec.affinity.as_str(), "RABBIT" | "KILLER_RABBIT") {
        add(
            &mut nodes,
            "cotton_tail".into(),
            "TAIL",
            Some(torso),
            -12.0 * scale,
            -2.0 * scale,
            2.5 * scale,
            2.5 * scale,
            0.0,
            "FUR_WHITE",
            true,
        );
    }
    if spec.affinity == "TREE_MONSTER" {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            for i in 0..5 {
                let side = i as f32 - 2.0;
                add(
                    &mut nodes,
                    format!("canopy_{i}"),
                    "LOBE",
                    Some(head),
                    side * 5.0 * scale,
                    (-25.0 - (2.0 - side.abs()) * 2.0) * scale,
                    5.5 * scale,
                    4.0 * scale,
                    0.0,
                    "LEAF",
                    false,
                );
            }
        }
    }
    if spec.features.iter().any(|feature| feature == "GILL") {
        if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
            for i in 0..3 {
                add(
                    &mut nodes,
                    format!("gill_{i}"),
                    "FIN",
                    Some(head),
                    (0.5 + i as f32 * 1.5) * scale,
                    (-10.0 + i as f32 * 1.8) * scale,
                    1.2 * scale,
                    2.1 * scale,
                    0.0,
                    primary,
                    true,
                );
            }
        }
    }
    match spec.body_plan.as_str() {
        "ARACHNID" | "INSECT" => {
            for i in 0..spec.limb_count.clamp(4, 12) {
                let centipede = spec.affinity == "CENTIPEDE";
                let insect = spec.body_plan == "INSECT";
                let side = if centipede {
                    if i.is_multiple_of(2) {
                        -1.0
                    } else {
                        1.0
                    }
                } else if i < if insect { 3 } else { 4 } {
                    -1.0
                } else {
                    1.0
                };
                let rank = if centipede {
                    (i / 2) as f32
                } else if insect {
                    (i % 3) as f32
                } else {
                    (i % 4) as f32
                };
                let anchor_x = (if centipede {
                    -13.0 + rank * 4.1
                } else if insect {
                    -6.0 + rank * 6.0
                } else {
                    -8.0 + rank * 4.0
                }) * scale;
                let anchor = add(
                    &mut nodes,
                    format!("hip_{i}"),
                    "HIP",
                    Some(torso),
                    anchor_x,
                    -scale,
                    1.5 * scale,
                    1.4 * scale,
                    0.0,
                    primary,
                    false,
                );
                let knee_x = anchor_x + side * (6.0 + rank * 1.2) * scale;
                let knee = add(
                    &mut nodes,
                    format!("limb_{i}"),
                    "LIMB",
                    Some(anchor),
                    knee_x,
                    (-7.0 + rank * 1.3 + rng.random_range(-0.8..0.8)) * scale,
                    1.5 * scale,
                    1.4 * scale,
                    0.0,
                    primary,
                    false,
                );
                add(
                    &mut nodes,
                    format!("foot_{i}"),
                    "FOOT",
                    Some(knee),
                    knee_x + side * (5.5 - rank * 0.4) * scale,
                    (9.0 + rank * 0.6) * scale,
                    1.2 * scale,
                    1.5 * scale,
                    0.0,
                    "HORN",
                    true,
                );
            }
        }
        "SERPENT" => {
            let mut parent = torso;
            let segments = if ["SHARK", "FISH", "WHALE", "ORCA"].contains(&spec.affinity.as_str()) {
                3
            } else if ["SAND_WORM", "JORMUNGANDR", "APOPHIS", "GROOTSLANG"]
                .contains(&spec.affinity.as_str())
            {
                9
            } else {
                6
            };
            for i in 0..segments {
                let taper = 1.0 - i as f32 * (0.82 / segments as f32);
                parent = add(
                    &mut nodes,
                    format!("tail_{i}"),
                    "TAIL",
                    Some(parent),
                    (-6.0 - i as f32 * 3.6) * scale,
                    (2.0 + (i as f32 * 0.8).sin() * 1.3) * scale,
                    5.7 * taper * scale,
                    4.0 * taper * scale,
                    0.0,
                    primary,
                    false,
                );
            }
        }
        "GASTROPOD" => {
            add(
                &mut nodes,
                "sole".into(),
                "SOLE",
                Some(torso),
                -scale,
                7.3 * scale,
                12.5 * scale,
                1.8 * scale,
                0.0,
                primary,
                false,
            );
            if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
                for i in 0..2 {
                    let stalk = add(
                        &mut nodes,
                        format!("stalk_{i}"),
                        "TENTACLE",
                        Some(head),
                        (11.0 + i as f32 * 2.0) * scale,
                        -8.0 * scale,
                        0.9 * scale,
                        4.0 * scale,
                        0.0,
                        primary,
                        false,
                    );
                    add(
                        &mut nodes,
                        format!("stalk_eye_{i}"),
                        "EYE",
                        Some(stalk),
                        (11.0 + i as f32 * 2.0) * scale,
                        -12.0 * scale,
                        1.1 * scale,
                        1.1 * scale,
                        0.0,
                        "ENERGY",
                        true,
                    );
                }
            }
        }
        "FLOATING" => {
            for i in 0..3 {
                add(
                    &mut nodes,
                    format!("wisp_{i}"),
                    "TENTACLE",
                    Some(torso),
                    (-7.0 + i as f32 * 7.0) * scale,
                    8.0 * scale,
                    2.0 * scale,
                    7.0 * scale,
                    0.2 * i as f32,
                    primary,
                    false,
                );
            }
        }
        "AMORPHOUS" => {
            for i in 0..5 {
                let offset = i as f32 - 2.0;
                add(
                    &mut nodes,
                    format!("lobe_{i}"),
                    "LOBE",
                    Some(torso),
                    offset * 4.2 * scale,
                    (6.0 - offset.abs() * 0.7) * scale,
                    (4.0 + rng.random_range(-0.3..0.5)) * scale,
                    (3.4 + rng.random_range(-0.3..0.5)) * scale,
                    0.0,
                    primary,
                    false,
                );
            }
        }
        "FUNGUS" => {
            for i in 0..5 {
                let spread = i as f32 - 2.0;
                add(
                    &mut nodes,
                    format!("root_{i}"),
                    "ROOT",
                    Some(torso),
                    spread * 3.1 * scale,
                    8.0 * scale,
                    1.2 * scale,
                    4.2 * scale,
                    spread * 0.15,
                    primary,
                    false,
                );
            }
        }
        "TANK" => {
            add(
                &mut nodes,
                "chassis".into(),
                "CHASSIS",
                Some(torso),
                0.0,
                4.5 * scale,
                12.5 * scale,
                3.4 * scale,
                0.0,
                "METAL",
                false,
            );
            for side in 0..2 {
                add(
                    &mut nodes,
                    format!("tread_{side}"),
                    "TREAD",
                    Some(torso),
                    0.0,
                    7.0 * scale,
                    13.0 * scale,
                    3.0 * scale,
                    0.0,
                    "METAL",
                    false,
                );
            }
        }
        _ => {
            let limb_count = spec.limb_count.clamp(1, 12);
            for i in 0..limb_count {
                let side = if i % 2 == 0 { -1.0 } else { 1.0 };
                let rank = (i / 2) as f32;
                let four_footed = quadruped || dragon;
                let anchor_x = if four_footed {
                    if rank < 1.0 {
                        6.0
                    } else {
                        -8.0
                    }
                } else {
                    side * (3.0 + rank * 1.3)
                };
                let hip = add(
                    &mut nodes,
                    format!("leg_root_{i}"),
                    "HIP",
                    Some(torso),
                    anchor_x * scale,
                    (if four_footed {
                        1.5
                    } else if winged_animal {
                        2.5
                    } else {
                        4.0
                    }) * scale,
                    2.3 * scale,
                    2.1 * scale,
                    0.0,
                    primary,
                    false,
                );
                let upper_x = if four_footed {
                    anchor_x + if rank < 1.0 { 1.0 } else { -1.0 }
                } else {
                    anchor_x + side * 0.5
                };
                let upper = add(
                    &mut nodes,
                    format!("limb_{i}"),
                    "LIMB",
                    Some(hip),
                    upper_x * scale,
                    (if four_footed {
                        5.0
                    } else if winged_animal {
                        4.5
                    } else {
                        7.5
                    }) * scale,
                    (2.1 + rng.random_range(-0.3..0.4)) * scale,
                    (if four_footed { 3.7 } else { 4.7 }) * scale,
                    side * 0.12,
                    primary,
                    false,
                );
                let lower = add(
                    &mut nodes,
                    format!("shin_{i}"),
                    "SHIN",
                    Some(upper),
                    (upper_x + side * if four_footed { 0.2 } else { 0.8 }) * scale,
                    (if four_footed {
                        8.5
                    } else if winged_animal {
                        7.0
                    } else {
                        11.0
                    }) * scale,
                    1.5 * scale,
                    3.6 * scale,
                    0.0,
                    primary,
                    false,
                );
                add(
                    &mut nodes,
                    format!("foot_{i}"),
                    "FOOT",
                    Some(lower),
                    (upper_x + if four_footed { 1.5 } else { side * 1.2 }) * scale,
                    (if four_footed {
                        11.5
                    } else if winged_animal {
                        9.5
                    } else {
                        14.0
                    }) * scale,
                    (if four_footed { 2.3 } else { 2.5 }) * scale,
                    1.4 * scale,
                    0.0,
                    if winged_animal || spec.features.iter().any(|x| x == "CLAW") {
                        "HORN"
                    } else {
                        primary
                    },
                    true,
                );
            }
        }
    }
    for i in 0..spec.arm_count.min(8) {
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        let rank = (i / 2) as f32;
        let shoulder_x = if horizontal {
            3.0 + rank * 2.0
        } else {
            side * (6.0 + rank * 0.4)
        };
        let shoulder_y = if horizontal {
            -4.0 + rank * 1.5
        } else {
            -5.0 + rank * 3.0
        };
        let shoulder = add(
            &mut nodes,
            format!("shoulder_{i}"),
            "SHOULDER",
            Some(torso),
            shoulder_x * scale,
            shoulder_y * scale,
            1.6 * scale,
            1.6 * scale,
            0.0,
            primary,
            false,
        );
        let elbow = add(
            &mut nodes,
            format!("arm_{i}"),
            "ARM",
            Some(shoulder),
            (shoulder_x + side * 4.0) * scale,
            (shoulder_y + 4.0) * scale,
            1.6 * scale,
            2.2 * scale,
            side * 0.4,
            primary,
            false,
        );
        let hand = add(
            &mut nodes,
            format!("hand_{i}"),
            "HAND",
            Some(elbow),
            (shoulder_x + side * 6.5) * scale,
            (shoulder_y + 8.0) * scale,
            (if spec
                .features
                .iter()
                .any(|feature| feature == "PINCER_HANDS")
            {
                3.0
            } else {
                1.8
            }) * scale,
            (if spec
                .features
                .iter()
                .any(|feature| feature == "PINCER_HANDS")
            {
                2.7
            } else {
                1.5
            }) * scale,
            0.0,
            if spec
                .features
                .iter()
                .any(|feature| feature == "PINCER_HANDS")
            {
                "CHITIN"
            } else if spec.features.iter().any(|feature| feature == "WEBBED") {
                "MEMBRANE"
            } else {
                primary
            },
            true,
        );
        if spec
            .features
            .iter()
            .any(|feature| feature == "PINCER_HANDS")
        {
            for tine in 0..2 {
                add(
                    &mut nodes,
                    format!("pincer_{i}_{tine}"),
                    "CLAW",
                    Some(hand),
                    (shoulder_x + side * (8.0 + tine as f32)) * scale,
                    (shoulder_y + 9.0 + tine as f32 * 1.6) * scale,
                    2.0 * scale,
                    4.0 * scale,
                    side * (0.4 - tine as f32 * 0.8),
                    "CHITIN",
                    true,
                );
            }
        }
    }
    if spec.body_plan == "WINGED"
        || spec.affinity == "SPHINX"
        || spec.features.iter().any(|x| x == "WING")
    {
        for i in 0..2 {
            let side = if i == 0 { -1.0 } else { 1.0 };
            let span = match spec.affinity.as_str() {
                "ALBATROSS" | "CONDOR" => 18.0,
                "EAGLE" | "VULTURE" | "BAT" => 13.0,
                "HUMMINGBIRD" => 5.5,
                "SIREN" | "HARPY" => 12.5,
                _ if avian => 10.5,
                _ => 8.0,
            };
            add(
                &mut nodes,
                format!("wing_{i}"),
                "WING",
                Some(torso),
                (if dragon { -5.0 } else { -3.0 }) * scale,
                (if dragon { -6.0 } else { -5.0 }) * scale,
                (if dragon { 11.0 } else { span }) * scale,
                (if dragon {
                    6.0
                } else if winged_animal {
                    span * 0.5
                } else {
                    4.5
                }) * scale,
                side * 0.6,
                if dragon || bat {
                    "MEMBRANE"
                } else if avian || matches!(spec.affinity.as_str(), "SIREN" | "HARPY") {
                    primary
                } else {
                    "FEATHER"
                },
                false,
            );
        }
    }
    if avian {
        for i in 0..3 {
            add(
                &mut nodes,
                format!("tail_feather_{i}"),
                "TAIL",
                Some(torso),
                (-13.0 - i as f32 * 1.2) * scale,
                (-3.0 + i as f32 * 1.5) * scale,
                3.3 * scale,
                1.0 * scale,
                -0.12 + i as f32 * 0.12,
                primary,
                false,
            );
        }
    }
    if [
        "SHOTGUN", "LASER", "RAILGUN", "CANNON", "ROCKET", "MISSILE", "SWORD", "CLUB", "AXE",
        "SPEAR", "BOW", "CHAINSAW",
    ]
    .contains(&spec.attack.concept.as_str())
        && !spec.features.iter().any(|x| x == "EYE_LASER")
    {
        let weapon_parent = if horizontal {
            nodes.iter().position(|n| n.id == "head_0").unwrap_or(torso)
        } else {
            nodes
                .iter()
                .position(|n| n.id == "hand_1")
                .or_else(|| nodes.iter().position(|n| n.id == "hand_0"))
                .unwrap_or(torso)
        };
        let weapon_x = nodes[weapon_parent].x + 3.3 * scale;
        let weapon_y = nodes[weapon_parent].y
            - if matches!(
                spec.attack.concept.as_str(),
                "SWORD" | "AXE" | "CLUB" | "SPEAR" | "CHAINSAW"
            ) {
                2.7 * scale
            } else {
                0.8 * scale
            };
        add(
            &mut nodes,
            "weapon".into(),
            "WEAPON",
            Some(weapon_parent),
            weapon_x,
            weapon_y,
            (if spec.attack.concept == "SPEAR" {
                7.0
            } else {
                5.0
            }) * scale,
            (if spec.attack.concept == "BOW" {
                5.5
            } else if matches!(
                spec.attack.concept.as_str(),
                "SWORD" | "AXE" | "CLUB" | "SPEAR" | "CHAINSAW"
            ) {
                2.0
            } else {
                1.5
            }) * scale,
            if matches!(
                spec.attack.concept.as_str(),
                "SWORD" | "CLUB" | "AXE" | "SPEAR" | "CHAINSAW"
            ) {
                -0.75
            } else {
                0.0
            },
            if spec.attack.concept == "CLUB" || spec.attack.concept == "BOW" {
                "WOOD"
            } else {
                "METAL"
            },
            true,
        );
    }
    if (spec.features.iter().any(|x| x == "TAIL" || x == "STINGER") || quadruped || dragon)
        && spec.body_plan != "SERPENT"
    {
        add(
            &mut nodes,
            "tail".into(),
            "TAIL",
            Some(torso),
            (if dragon || theropod || longneck {
                -15.0
            } else {
                -11.0
            }) * scale,
            (if dragon { 0.0 } else { 1.0 }) * scale,
            (if crocodile || dragon || theropod || longneck {
                10.0
            } else {
                6.0
            }) * scale,
            (if crocodile || dragon || theropod || longneck {
                2.8
            } else {
                2.0
            }) * scale,
            0.2,
            primary,
            false,
        );
    }
    if theropod && !nodes.iter().any(|node| node.id == "tail") {
        add(
            &mut nodes,
            "tail".into(),
            "TAIL",
            Some(torso),
            -15.0 * scale,
            2.0 * scale,
            11.0 * scale,
            3.0 * scale,
            0.1,
            primary,
            false,
        );
    }
    let has = |name: &str| spec.features.iter().any(|f| f == name);
    if has("ARMOR_PLATES") {
        add(
            &mut nodes,
            "armored_carapace".into(),
            "SHELL",
            Some(torso),
            -2.0 * scale,
            -5.0 * scale,
            13.0 * scale,
            4.0 * scale,
            0.0,
            primary,
            false,
        );
        for i in 0..4 {
            add(
                &mut nodes,
                format!("armor_spike_{i}"),
                "HORN",
                Some(torso),
                (-9.0 + i as f32 * 5.4) * scale,
                -8.0 * scale,
                1.3 * scale,
                2.2 * scale,
                0.0,
                "HORN",
                true,
            );
        }
    }
    if has("BACK_PLATES") {
        for i in 0..6 {
            add(
                &mut nodes,
                format!("dorsal_plate_{i}"),
                "FIN",
                Some(torso),
                (-11.0 + i as f32 * 4.2) * scale,
                -8.0 * scale,
                2.2 * scale,
                6.0 * scale,
                (i as f32 - 2.5) * 0.08,
                "HORN",
                true,
            );
        }
    }
    if has("TAIL_CLUB") {
        if let Some(tail) = nodes.iter().position(|n| n.id == "tail") {
            let (tx, ty) = (nodes[tail].x, nodes[tail].y);
            add(
                &mut nodes,
                "tail_club".into(),
                "SHELL",
                Some(tail),
                tx - 9.0 * scale,
                ty + 1.0 * scale,
                4.0 * scale,
                3.4 * scale,
                0.0,
                "HORN",
                true,
            );
        }
    }
    if has("FIN") {
        for i in 0..3 {
            add(
                &mut nodes,
                format!("fin_{i}"),
                "FIN",
                Some(torso),
                (-5.0 + i as f32 * 6.0) * scale,
                (if i == 1 { -8.0 } else { 3.0 }) * scale,
                (if i == 1 { 5.0 } else { 3.5 }) * scale,
                (if i == 1 { 7.0 } else { 4.0 }) * scale,
                (i as f32 - 1.0) * 0.35,
                primary,
                true,
            );
        }
    }
    if spec.affinity == "ORCA" {
        add(
            &mut nodes,
            "orca_belly".into(),
            "LOBE",
            Some(torso),
            1.5 * scale,
            3.2 * scale,
            8.5 * scale,
            2.3 * scale,
            0.0,
            "ORCA_WHITE",
            false,
        );
        if let Some(tail) = nodes.iter().position(|node| node.id == "tail_2") {
            let (tx, ty) = (nodes[tail].x, nodes[tail].y);
            add(
                &mut nodes,
                "orca_fluke".into(),
                "FIN",
                Some(tail),
                tx - 4.0 * scale,
                ty,
                4.5 * scale,
                2.6 * scale,
                0.0,
                primary,
                true,
            );
        }
    }
    if has("WHIRLWIND") {
        for i in 0..5 {
            add(
                &mut nodes,
                format!("vortex_{i}"),
                "VORTEX",
                Some(torso),
                (i as f32 * 1.8).sin() * 2.0 * scale,
                (9.0 - i as f32 * 5.0) * scale,
                (3.5 + i as f32 * 1.8) * scale,
                1.2 * scale,
                i as f32 * 0.25,
                "STORM",
                true,
            );
        }
    }
    if ["CEPHALOPOD", "KRAKEN"].contains(&spec.affinity.as_str()) {
        let count = if spec.affinity == "KRAKEN" { 8 } else { 6 };
        for i in 0..count {
            let spread = i as f32 - (count as f32 - 1.0) * 0.5;
            let root = add(
                &mut nodes,
                format!("sea_arm_{i}"),
                "TENTACLE",
                Some(torso),
                spread * 3.3 * scale,
                4.0 * scale,
                (if spec.affinity == "KRAKEN" { 2.3 } else { 1.6 }) * scale,
                5.0 * scale,
                spread * 0.12,
                primary,
                false,
            );
            add(
                &mut nodes,
                format!("sea_tip_{i}"),
                "TENTACLE",
                Some(root),
                spread * 5.2 * scale,
                (if spec.affinity == "KRAKEN" {
                    17.0
                } else {
                    12.0
                }) * scale,
                (if spec.affinity == "KRAKEN" { 1.4 } else { 0.9 }) * scale,
                4.5 * scale,
                -spread * 0.15,
                primary,
                false,
            );
        }
    }
    if let Some(head) = nodes.iter().position(|n| n.id == "head_0") {
        let hx = nodes[head].x;
        let hy = nodes[head].y;
        if has("MANY_EYES") && !arachnid {
            for i in 0..4 {
                add(
                    &mut nodes,
                    format!("extra_eye_{i}"),
                    "EYE",
                    Some(head),
                    hx + (i as f32 - 1.5) * 1.7 * scale,
                    hy - 1.8 * scale + (i % 2) as f32 * 2.2 * scale,
                    0.65 * scale,
                    0.65 * scale,
                    0.0,
                    "ENERGY",
                    true,
                );
            }
        }
        if has("FANG") || ["SPIDER", "COBRA", "SCORPION"].contains(&spec.affinity.as_str()) {
            for i in 0..2 {
                add(
                    &mut nodes,
                    format!("fang_{i}"),
                    "FANG",
                    Some(head),
                    hx + (3.0 + i as f32 * 1.4) * scale,
                    hy + 2.8 * scale,
                    0.9 * scale,
                    2.5 * scale,
                    0.2,
                    "HORN",
                    true,
                );
            }
        }
        if has("ANTLER") {
            for i in 0..2 {
                let side = if i == 0 { -1.0 } else { 1.0 };
                add(
                    &mut nodes,
                    format!("antler_{i}"),
                    "ANTLER",
                    Some(head),
                    hx + side * 3.0 * scale,
                    hy - 6.0 * scale,
                    1.5 * scale,
                    5.0 * scale,
                    side * 0.35,
                    "HORN",
                    true,
                );
            }
        }
        if has("TUSK") {
            for i in 0..2 {
                let mammoth = spec.affinity == "MAMMOTH";
                add(
                    &mut nodes,
                    format!("tusk_{i}"),
                    "TUSK",
                    Some(head),
                    hx + (3.0 + i as f32 * if mammoth { 2.6 } else { 1.8 }) * scale,
                    hy + (if mammoth { 5.0 } else { 2.8 }) * scale,
                    (if mammoth { 1.5 } else { 1.0 }) * scale,
                    (if mammoth { 6.0 } else { 3.0 }) * scale,
                    if mammoth { -0.55 } else { -0.35 },
                    "BONE",
                    true,
                );
            }
        }
        if has("SABER_FANGS") {
            for i in 0..2 {
                add(
                    &mut nodes,
                    format!("saber_fang_{i}"),
                    "FANG",
                    Some(head),
                    hx + (3.0 + i as f32 * 1.7) * scale,
                    hy + 4.5 * scale,
                    1.2 * scale,
                    6.0 * scale,
                    0.18,
                    "BONE",
                    true,
                );
            }
        }
        if has("TRIPLE_HORN") {
            for i in 0..3 {
                add(
                    &mut nodes,
                    format!("triceratops_horn_{i}"),
                    "HORN",
                    Some(head),
                    (hx + if i == 2 { 5.5 } else { -2.0 + i as f32 * 4.0 }) * scale,
                    (hy - if i == 2 { 1.0 } else { 4.5 }) * scale,
                    1.4 * scale,
                    (if i == 2 { 4.0 } else { 6.5 }) * scale,
                    -0.45,
                    "HORN",
                    true,
                );
            }
        }
        if has("FRILL") {
            add(
                &mut nodes,
                "head_frill".into(),
                "SHELL",
                Some(head),
                (hx - 5.0) * scale,
                (hy - 3.0) * scale,
                5.5 * scale,
                6.0 * scale,
                -0.25,
                primary,
                false,
            );
        }
    }
    if has("CLAW") {
        let terminals: Vec<usize> = nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "FOOT" || n.kind == "HAND")
            .map(|(i, _)| i)
            .collect();
        for (i, parent) in terminals.into_iter().enumerate() {
            let n = &nodes[parent];
            let (x, y) = (n.x, n.y);
            add(
                &mut nodes,
                format!("claw_{i}"),
                "CLAW",
                Some(parent),
                x + (if matches!(spec.affinity.as_str(), "SIREN" | "HARPY") {
                    3.0
                } else {
                    2.0
                }) * scale,
                y + 1.0 * scale,
                (if matches!(spec.affinity.as_str(), "SIREN" | "HARPY") {
                    1.8
                } else {
                    1.2
                }) * scale,
                (if matches!(spec.affinity.as_str(), "SIREN" | "HARPY") {
                    4.8
                } else {
                    2.3
                }) * scale,
                0.5,
                "HORN",
                true,
            );
        }
    }
    if has("TENTACLE") && spec.body_plan != "AMORPHOUS" {
        for i in 0..3 {
            let root = add(
                &mut nodes,
                format!("tendril_{i}"),
                "TENTACLE",
                Some(torso),
                (-6.0 + i as f32 * 6.0) * scale,
                4.0 * scale,
                1.5 * scale,
                4.0 * scale,
                0.2,
                primary,
                false,
            );
            add(
                &mut nodes,
                format!("tendril_tip_{i}"),
                "TENTACLE",
                Some(root),
                (-8.0 + i as f32 * 8.0) * scale,
                10.0 * scale,
                1.0 * scale,
                3.0 * scale,
                -0.3,
                primary,
                false,
            );
        }
    }
    if has("PUSTULE") || spec.materials.iter().any(|m| m == "TUMOR") {
        for i in 0..3 {
            add(
                &mut nodes,
                format!("growth_{i}"),
                "GROWTH",
                Some(torso),
                rng.random_range(-5.0..6.0) * scale,
                rng.random_range(-5.0..2.0) * scale,
                rng.random_range(1.4..3.2) * scale,
                rng.random_range(1.4..3.0) * scale,
                0.0,
                "TUMOR",
                false,
            );
        }
    }
    if has("SHELL") {
        add(
            &mut nodes,
            "shell".into(),
            "SHELL",
            Some(torso),
            (if gastropod { -4.0 } else { -2.0 }) * scale,
            (if gastropod { -2.0 } else { -5.0 }) * scale,
            (if gastropod { 8.0 } else { 9.0 }) * scale,
            (if gastropod { 8.0 } else { 4.0 }) * scale,
            -0.1,
            if gastropod { "HORN" } else { "CHITIN" },
            false,
        );
    }
    if has("SMOKE") || has("FLAME") {
        for i in 0..3 {
            add(
                &mut nodes,
                format!("aura_{i}"),
                "AURA",
                Some(torso),
                (-4.0 + i as f32 * 4.0) * scale,
                -10.0 * scale,
                1.5 * scale,
                (3.0 + i as f32) * scale,
                0.1 * i as f32,
                if has("FLAME") { "FIRE" } else { "SMOKE" },
                false,
            );
        }
    }
    if ["SCORPION", "MANTICORE", "HORNET"].contains(&spec.affinity.as_str()) {
        if let Some(tail) = nodes.iter().position(|n| n.id == "tail") {
            let n = &nodes[tail];
            let (x, y) = (n.x, n.y);
            add(
                &mut nodes,
                "stinger".into(),
                "STINGER",
                Some(tail),
                x - 6.0 * scale,
                y - 4.0 * scale,
                1.3 * scale,
                3.0 * scale,
                -0.5,
                "HORN",
                true,
            );
        }
    }
    if matches!(
        spec.affinity.as_str(),
        "ELEPHANT" | "GROOTSLANG" | "MAMMOTH"
    ) {
        if let Some(head) = nodes.iter().position(|n| n.id == "head_0") {
            let (hx, hy) = (nodes[head].x, nodes[head].y);
            add(
                &mut nodes,
                "trunk".into(),
                "TENTACLE",
                Some(head),
                hx + 5.0 * scale,
                hy + 5.5 * scale,
                1.7 * scale,
                6.0 * scale,
                0.3,
                primary,
                false,
            );
        }
    }
    if spec.affinity == "DRONE" || spec.secondary_affinities.iter().any(|a| a == "DRONE") {
        for i in 0usize..4 {
            let side = if i.is_multiple_of(2) { -1.0 } else { 1.0 };
            let (x, y) = ((5.0 + (i / 2) as f32 * 5.0) * side * scale, -6.0 * scale);
            add(
                &mut nodes,
                format!("rotor_{i}"),
                "ROTOR",
                Some(torso),
                x,
                y,
                3.0 * scale,
                0.6 * scale,
                0.0,
                "METAL",
                false,
            );
        }
    }
    if spec.affinity == "MUSHROOM" {
        for i in 0..4 {
            let dx = (i as f32 - 1.5) * 3.0 * scale;
            add(
                &mut nodes,
                format!("spore_sac_{i}"),
                "GROWTH",
                Some(torso),
                dx,
                -8.0 * scale,
                1.1 * scale,
                1.1 * scale,
                0.0,
                "ENERGY",
                true,
            );
        }
    }
    let mut hybrids = spec.secondary_affinities.clone();
    if hybrids.is_empty()
        && spec.mount.is_none()
        && ["HUMANOID", "QUADRUPED", "SERPENT", "WINGED"].contains(&spec.body_plan.as_str())
        && !["HITLER", "ANIME_GIRL", "ROBOT", "TANK"].contains(&spec.affinity.as_str())
        && rng.random_bool(0.10)
    {
        let options: &[&str] = match spec.body_plan.as_str() {
            "SERPENT" => &["LION", "FALCON", "WOLF"],
            "QUADRUPED" => &["GOAT", "RAVEN", "CROCODILE"],
            _ => &["WOLF", "GOAT", "RAVEN"],
        };
        hybrids.push(options[rng.random_range(0..options.len())].into());
    }
    for (index, affinity) in hybrids.iter().take(2).enumerate() {
        let side = if index == 0 { -1.0 } else { 1.0 };
        let (x, y) = if horizontal {
            (3.0 * scale, (-10.0 - index as f32 * 3.0) * scale)
        } else {
            ((-5.0 + index as f32 * 10.0) * scale, -13.0 * scale)
        };
        let head = add(
            &mut nodes,
            format!("hybrid_head_{index}"),
            "HEAD",
            Some(torso),
            x,
            y,
            3.7 * scale,
            3.5 * scale,
            side * 0.16,
            material_for_affinity(affinity),
            false,
        );
        add(
            &mut nodes,
            format!("hybrid_eye_{index}"),
            "EYE",
            Some(head),
            x + 1.5 * scale,
            y - 0.7 * scale,
            0.8 * scale,
            0.8 * scale,
            0.0,
            "ENERGY",
            true,
        );
        if ["GOAT", "RAM", "BULL", "BUFFALO", "MOOSE", "RHINO"].contains(&affinity.as_str()) {
            add(
                &mut nodes,
                format!("hybrid_horn_{index}"),
                "HORN",
                Some(head),
                x - 1.4 * scale,
                y - 4.0 * scale,
                1.4 * scale,
                4.0 * scale,
                side * 0.3,
                "HORN",
                true,
            );
        }
        if ["FALCON", "RAVEN", "HARPY", "GARUDA", "HORNET"].contains(&affinity.as_str()) {
            add(
                &mut nodes,
                format!("hybrid_wing_{index}"),
                "WING",
                Some(torso),
                (-5.0 + index as f32 * 2.0) * scale,
                -7.0 * scale,
                7.0 * scale,
                4.0 * scale,
                side * 0.5,
                "FEATHER",
                false,
            );
        }
    }
    if ["BEAR", "BUFFALO", "RHINO", "HIPPO", "ELEPHANT"].contains(&spec.affinity.as_str()) {
        nodes[torso].rx *= 1.18;
        nodes[torso].ry *= 1.15;
    }
    if spec.affinity == "MOOSE" || spec.affinity == "GIRAFFE" {
        for node in &mut nodes {
            if ["LIMB", "SHIN"].contains(&node.kind.as_str()) {
                node.ry *= 1.25;
            }
        }
    }
    if theropod {
        for node in &mut nodes {
            if matches!(node.kind.as_str(), "ARM" | "HAND" | "SHOULDER") {
                node.rx *= if spec.affinity == "TREX" { 0.52 } else { 0.72 };
                node.ry *= if spec.affinity == "TREX" { 0.58 } else { 0.8 };
                node.y -= 2.0 * scale;
            }
        }
    }
    if spec.affinity == "BIGFOOT" || spec.affinity == "YETI" {
        for node in &mut nodes {
            if node.kind == "FOOT" {
                node.rx *= 1.55;
            }
        }
    }
    if ["ANIME_GIRL", "HITLER"].contains(&spec.affinity.as_str()) {
        let anime = spec.affinity == "ANIME_GIRL";
        for node in &mut nodes {
            match node.kind.as_str() {
                "TORSO" => {
                    node.rx *= if anime { 0.68 } else { 0.76 };
                    node.material = if anime {
                        "CLOTH_IVORY"
                    } else {
                        "UNIFORM_BROWN"
                    }
                    .into();
                }
                "HEAD" => {
                    node.rx *= if anime { 1.36 } else { 1.19 };
                    node.ry *= if anime { 1.28 } else { 1.12 };
                    node.material = "SKIN_PALE".into();
                }
                "NECK" => node.material = "SKIN_PALE".into(),
                "EYE" | "MOUTH" => node.material = "EYE_DARK".into(),
                "SHOULDER" | "ARM" => {
                    node.rx *= 0.7;
                    node.x *= if anime { 0.76 } else { 0.83 };
                    node.material = if anime {
                        "CLOTH_IVORY"
                    } else {
                        "UNIFORM_BROWN"
                    }
                    .into();
                }
                "HAND" => {
                    node.x *= if anime { 0.76 } else { 0.83 };
                    node.material = "SKIN_PALE".into();
                }
                "LIMB" | "SHIN" | "HIP" => {
                    node.rx *= 0.78;
                    node.x *= if anime { 0.84 } else { 0.9 };
                    node.material = if anime {
                        "STOCKING_DARK"
                    } else {
                        "UNIFORM_BROWN"
                    }
                    .into();
                }
                "FOOT" => node.material = "LEATHER_BLACK".into(),
                _ => {}
            }
        }
        if anime {
            add(
                &mut nodes,
                "skirt".into(),
                "SKIRT",
                Some(torso),
                0.0,
                5.0 * scale,
                7.2 * scale,
                6.5 * scale,
                0.0,
                "CLOTH_NAVY",
                false,
            );
            if let Some(head) = nodes.iter().position(|node| node.id == "head_0") {
                let (hx, hy) = (nodes[head].x, nodes[head].y);
                add(
                    &mut nodes,
                    "hair_cap".into(),
                    "HAIR",
                    Some(head),
                    hx - 2.2 * scale,
                    hy - 2.3 * scale,
                    5.6 * scale,
                    4.7 * scale,
                    0.0,
                    "HAIR_DARK",
                    false,
                );
                add(
                    &mut nodes,
                    "hair_lock".into(),
                    "HAIR",
                    Some(head),
                    hx - 4.3 * scale,
                    hy + 4.0 * scale,
                    2.4 * scale,
                    5.8 * scale,
                    0.15,
                    "HAIR_DARK",
                    false,
                );
            }
        } else {
            add(
                &mut nodes,
                "belt".into(),
                "BELT",
                Some(torso),
                0.0,
                4.0 * scale,
                6.5 * scale,
                1.1 * scale,
                0.0,
                "LEATHER_BLACK",
                false,
            );
            add(
                &mut nodes,
                "collar".into(),
                "COLLAR",
                Some(torso),
                0.0,
                -8.0 * scale,
                4.0 * scale,
                1.3 * scale,
                0.0,
                "CLOTH_IVORY",
                true,
            );
        }
    }
    apply_part_scales(spec, &mut nodes);
    // Choose one coherent body palette per specimen, keeping explicit material intent.
    let variants: &[&str] = match primary.as_str() {
        "FLESH" => &["FLESH", "FLESH_ASH", "FLESH_OCHRE", "FLESH_RUST"],
        "CHITIN" => &["CHITIN", "CHITIN_UMBER", "CHITIN_JADE", "CHITIN_BLUE"],
        "SCALES" => &["SCALES", "SCALES_RUST", "SCALES_BLUE", "SCALES_OCHRE"],
        "FUR" => &["FUR", "FUR_DARK", "FUR_GOLD"],
        "SLIME" => &["SLIME", "SLIME_AMBER", "SLIME_BLUE"],
        _ => &[],
    };
    if !variants.is_empty() {
        let chosen = variants[rng.random_range(0..variants.len())];
        for node in &mut nodes {
            if node.material == *primary {
                node.material = chosen.into();
            }
        }
    }
    let mut mass = 0.0;
    let mut weighted = (0.0, 0.0);
    for n in &nodes {
        let m = std::f32::consts::PI * n.rx * n.ry * n.density;
        mass += m;
        weighted.0 += m * n.x;
        weighted.1 += m * n.y;
    }
    let com = (weighted.0 / mass, weighted.1 / mass);
    let mut repairs = Vec::new();
    let support = if [
        "FLOATING",
        "WINGED",
        "SERPENT",
        "GASTROPOD",
        "AMORPHOUS",
        "FUNGUS",
        "TANK",
    ]
    .contains(&spec.body_plan.as_str())
    {
        1.0
    } else {
        (spec.limb_count as f32 / 4.0).clamp(0.0, 2.0)
    };
    let mut gravity = 1.0;
    if spec.body_plan == "FLOATING" {
        gravity = 0.0;
        repairs.push("LEVITATION".into());
    } else if spec.features.iter().any(|feature| feature == "FIN") {
        gravity = 0.25;
        repairs.push("MAGICAL_BUOYANCY".into());
    } else if mass > 1800.0 && support < 1.2 {
        gravity = 0.65;
        repairs.push("GRAVITY_REDUCTION".into());
    }
    if com.0.abs() > 6.0 && support < 1.5 {
        repairs.push("TELEKINETIC_BALANCE".into());
    }
    if spec.body_plan == "WINGED" && mass > 850.0 {
        repairs.push("MAGICAL_LIFT".into());
    }
    if spec.materials.iter().any(|m| m == "SPECTRAL") {
        repairs.push("SPECTRAL_COLLISION".into());
    }
    if spec.features.iter().any(|f| f == "CLIMB") && mass > 900.0 {
        repairs.push("ADHESIVE_FEET".into());
    }
    if ["SHOTGUN", "RAILGUN", "ROCKET", "CANNON"].contains(&spec.attack.concept.as_str())
        && mass < 1200.0
    {
        repairs.push("RECOIL_DAMPING".into());
    }
    Body {
        nodes,
        mass,
        com,
        gravity,
        repairs,
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn scoped_weapon_size_changes_weapon_and_mass_without_enlarging_torso() {
        let base = build(
            &crate::parser::parse_vocabulary("orc with an axe"),
            &mut ChaCha8Rng::seed_from_u64(7),
        );
        let huge = build(
            &crate::parser::parse_vocabulary("orc with a huge axe"),
            &mut ChaCha8Rng::seed_from_u64(7),
        );
        let radius =
            |body: &Body, id: &str| body.nodes.iter().find(|node| node.id == id).unwrap().rx;
        assert_eq!(radius(&base, "torso"), radius(&huge, "torso"));
        assert!(radius(&huge, "weapon") > radius(&base, "weapon") * 1.7);
        assert!(huge.mass > base.mass);
    }

    #[test]
    fn named_humanoids_have_distinct_clothing_and_faces() {
        let anime = build(
            &crate::parser::parse_vocabulary("anime girl"),
            &mut ChaCha8Rng::seed_from_u64(7),
        );
        let historic = build(
            &crate::parser::parse_vocabulary("adolph hitler"),
            &mut ChaCha8Rng::seed_from_u64(7),
        );
        assert!(anime
            .nodes
            .iter()
            .any(|node| node.kind == "SKIRT" && node.material == "CLOTH_NAVY"));
        assert!(anime.nodes.iter().any(|node| node.kind == "HAIR"));
        assert!(historic.nodes.iter().any(|node| node.kind == "BELT"));
        assert!(historic
            .nodes
            .iter()
            .any(|node| node.material == "UNIFORM_BROWN"));
        assert!(anime
            .nodes
            .iter()
            .any(|node| node.kind == "HEAD" && node.material == "SKIN_PALE"));
    }
}
