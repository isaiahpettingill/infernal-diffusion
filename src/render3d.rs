//! Deterministic, headless 3D mesh baking for directional sprite sheets.
//! Meshes are authored assets; positions and dimensions still come from anatomy.

use crate::Instant;
use crate::{
    anatomy::{Body, Node},
    animation::Pose,
    parser::MonsterSpec,
    proto, render,
};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

#[path = "v2/face_meshes.rs"]
mod v2_faces;
#[path = "v2/skeleton_meshes.rs"]
mod v2_skeleton;

pub const ANGLES_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];
pub const PIXELS_PER_UNIT: f32 = 96.0 / 62.0;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct V3 {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) z: f32,
}
impl V3 {
    pub(crate) fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    fn dot(self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }
    fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }
    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn normalized(self) -> Self {
        self * (1.0 / self.length().max(1e-6))
    }
}
impl std::ops::Add for V3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}
impl std::ops::Sub for V3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}
impl std::ops::Mul<f32> for V3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

#[derive(Deserialize)]
struct MeshAsset {
    vertices: Vec<[f32; 3]>,
    faces: Vec<(usize, usize, usize, String)>,
}

const MESHES: &[(&str, &str)] = &[
    (
        "skull_insect",
        include_str!("../assets/meshes/skull_insect.json"),
    ),
    (
        "skull_spider",
        include_str!("../assets/meshes/skull_spider.json"),
    ),
    (
        "part_wing_insect",
        include_str!("../assets/meshes/part_wing_insect.json"),
    ),
    (
        "skull_canid",
        include_str!("../assets/meshes/skull_canid.json"),
    ),
    (
        "skull_reptile",
        include_str!("../assets/meshes/skull_reptile.json"),
    ),
    (
        "skull_worm",
        include_str!("../assets/meshes/skull_worm.json"),
    ),
    (
        "skull_feline",
        include_str!("../assets/meshes/skull_feline.json"),
    ),
    (
        "skull_bovine",
        include_str!("../assets/meshes/skull_bovine.json"),
    ),
    (
        "skull_humanoid",
        include_str!("../assets/meshes/skull_humanoid.json"),
    ),
    (
        "skull_cyclops",
        include_str!("../assets/meshes/skull_cyclops.json"),
    ),
    (
        "skull_arthropod",
        include_str!("../assets/meshes/skull_arthropod.json"),
    ),
    (
        "skull_avian",
        include_str!("../assets/meshes/skull_avian.json"),
    ),
    (
        "skull_generic",
        include_str!("../assets/meshes/skull_generic.json"),
    ),
    (
        "skull_bear",
        include_str!("../assets/meshes/skull_bear.json"),
    ),
    (
        "skull_moose",
        include_str!("../assets/meshes/skull_moose.json"),
    ),
    (
        "skull_rhino",
        include_str!("../assets/meshes/skull_rhino.json"),
    ),
    (
        "skull_elephant",
        include_str!("../assets/meshes/skull_elephant.json"),
    ),
    (
        "skull_alien",
        include_str!("../assets/meshes/skull_alien.json"),
    ),
    (
        "skull_goblin",
        include_str!("../assets/meshes/skull_goblin.json"),
    ),
    (
        "skull_robot",
        include_str!("../assets/meshes/skull_robot.json"),
    ),
    (
        "skull_mushroom",
        include_str!("../assets/meshes/skull_mushroom.json"),
    ),
    (
        "skull_clown",
        include_str!("../assets/meshes/skull_clown.json"),
    ),
    (
        "skull_anime",
        include_str!("../assets/meshes/skull_anime.json"),
    ),
    (
        "skull_hitler",
        include_str!("../assets/meshes/skull_hitler.json"),
    ),
    (
        "skull_tank",
        include_str!("../assets/meshes/skull_tank.json"),
    ),
    ("skull_stormtrooper", include_str!("../assets/meshes/skull_stormtrooper.json")),
    ("part_horn", include_str!("../assets/meshes/part_horn.json")),
    ("part_claw", include_str!("../assets/meshes/part_claw.json")),
    ("part_fang", include_str!("../assets/meshes/part_fang.json")),
    ("part_wing", include_str!("../assets/meshes/part_wing.json")),
    (
        "part_wing_feather",
        include_str!("../assets/meshes/part_wing_feather.json"),
    ),
    (
        "part_shell",
        include_str!("../assets/meshes/part_shell.json"),
    ),
    ("part_hoof", include_str!("../assets/meshes/part_hoof.json")),
    ("part_paw", include_str!("../assets/meshes/part_paw.json")),
    (
        "part_tread",
        include_str!("../assets/meshes/part_tread.json"),
    ),
    (
        "part_tank_hull",
        include_str!("../assets/meshes/part_tank_hull.json"),
    ),
    (
        "part_rotor",
        include_str!("../assets/meshes/part_rotor.json"),
    ),
    (
        "part_weapon",
        include_str!("../assets/meshes/part_weapon.json"),
    ),
    (
        "part_weapon_sword",
        include_str!("../assets/meshes/part_weapon_sword.json"),
    ),
    (
        "part_weapon_club",
        include_str!("../assets/meshes/part_weapon_club.json"),
    ),
    (
        "part_weapon_axe",
        include_str!("../assets/meshes/part_weapon_axe.json"),
    ),
    (
        "part_weapon_spear",
        include_str!("../assets/meshes/part_weapon_spear.json"),
    ),
    (
        "part_weapon_bow",
        include_str!("../assets/meshes/part_weapon_bow.json"),
    ),
    (
        "part_weapon_chainsaw",
        include_str!("../assets/meshes/part_weapon_chainsaw.json"),
    ),
];

fn asset(name: &str) -> &'static MeshAsset {
    static PARSED: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    let parsed = PARSED.get_or_init(|| {
        MESHES
            .iter()
            .map(|(_, raw)| serde_json::from_str(raw).expect("checked-in mesh asset is valid"))
            .collect()
    });
    let index = MESHES
        .iter()
        .position(|(id, _)| *id == name)
        .expect("known mesh recipe ID");
    &parsed[index]
}

pub fn recipe_fingerprint() -> String {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let mut hash = Sha256::new();
        for (id, raw) in MESHES {
            hash.update(id.as_bytes());
            hash.update(raw.as_bytes());
        }
        hash.finalize()[..8]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    })
    .clone()
}

// Follow the existing composite node namespace to its owning specimen.
fn v2_surface_spec<'a>(spec: &'a MonsterSpec, id: &str) -> &'a MonsterSpec {
    let mut current = spec;
    let mut path = id;
    while let Some(rest) = path.strip_prefix("mount_") {
        let Some(child) = current.mount.as_ref().and_then(|m| m.spec.as_deref()) else {
            return current;
        };
        current = child;
        path = rest;
    }
    if path.starts_with("rider_") {
        return current;
    }
    while let Some(child) = current.mount.as_ref().and_then(|m| m.spec.as_deref()) {
        current = child;
    }
    current
}
fn v2_has_nested_ghost(spec: &MonsterSpec) -> bool {
    spec.mount
        .as_ref()
        .and_then(|m| m.spec.as_deref())
        .is_some_and(|child| {
            child.features.iter().any(|f| f == "GHOST") || v2_has_nested_ghost(child)
        })
}
fn v2_local_surface(triangles: &mut [Triangle], spec: &MonsterSpec, node: &Node, seed: u64) {
    if !visual_v2(spec) || spec.mount.is_none() {
        return;
    }
    let local = v2_surface_spec(spec, &node.id);
    let gains = v2_gains(local, seed);
    let friendly = local.features.iter().any(|f| f == "FRIENDLY_LURE");
    for triangle in triangles {
        triangle.color = v2_surface_color(local, triangle.color, gains, friendly);
    }
}

fn node_identity<'a>(spec: &'a MonsterSpec, id: &str) -> (&'a str, &'a str) {
    if visual_v2(spec) {
        if let Some(identity) = crate::v2::hybrid::node_identity(spec, id) {
            return identity;
        }
    }
    let mut current = spec;
    let mut path = id;
    while let Some(rest) = path.strip_prefix("mount_") {
        let Some(mount) = &current.mount else { break };
        let Some(child) = mount.spec.as_deref() else {
            return (&mount.body_plan, &mount.affinity);
        };
        current = child;
        path = rest;
    }
    if path.starts_with("rider_") {
        return (&current.body_plan, &current.affinity);
    }
    while let Some(mount) = &current.mount {
        let Some(child) = mount.spec.as_deref() else {
            return (&mount.body_plan, &mount.affinity);
        };
        current = child;
    }
    (&current.body_plan, &current.affinity)
}

fn skull_recipe(spec: &MonsterSpec, node: &Node) -> &'static str {
    let (plan, identity) = node_identity(spec, &node.id);
    let affinity = if let Some(index) = node
        .id
        .strip_prefix("hybrid_head_")
        .and_then(|s| s.parse::<usize>().ok())
    {
        spec.secondary_affinities
            .get(index)
            .map(String::as_str)
            .unwrap_or(spec.affinity.as_str())
    } else {
        identity
    };
    match affinity {
        "HORNET" | "BEETLE" | "SCARAB" | "CENTIPEDE" => "skull_insect",
        "SPIDER" | "SCORPION" => "skull_spider",
        "WOLF" | "JACKAL" | "DOG" | "CERBERUS" | "FENRIR" => "skull_canid",
        "CHUPACABRA" | "XOLOTL" | "HYENA" => "skull_canid",
        "CROCODILE" | "COBRA" | "DRAGON" | "WYVERN" | "HYDRA" | "SERPENT" | "SNAKE" | "SHARK"
        | "SHARKNADO" | "FISH" | "EEL" | "SEA_DRAGON" | "SEA_SERPENT" | "TREX" | "RAPTOR"
        | "LONGNECK_DINO" | "STEGO" | "TRICERATOPS" | "ANKYLOSAUR" => "skull_reptile",
        "SAND_WORM" => "skull_worm",
        "ORCA" | "WHALE" => "skull_generic",
        "QUETZALCOATL" | "APOPHIS" | "JORMUNGANDR" | "CIPACTLI" => "skull_reptile",
        "LION" | "CAT" | "SPHINX" | "MANTICORE" | "GRIFFIN" | "SABERTOOTH_CAT" => "skull_feline",
        "TIGER" | "LEOPARD" | "CHEETAH" | "PANTHER" => "skull_feline",
        "MINOTAUR" | "BULL" | "GOAT" | "RAM" | "SATYR" | "BUFFALO" => "skull_bovine",
        "BEAR" | "BIGFOOT" | "YETI" => "skull_bear",
        "MOOSE" | "GIRAFFE" | "ANTELOPE" | "GAZELLE" => "skull_moose",
        "RHINO" => "skull_rhino",
        "ELEPHANT" | "GROOTSLANG" | "MAMMOTH" => "skull_elephant",
        "ALIEN" => "skull_alien",
        "ORC" | "GOBLIN" | "TROLL" | "TOKOLOSHE" => "skull_goblin",
        "STORMTROOPER" => "skull_stormtrooper",
        "ROBOT" | "DRONE" => "skull_robot",
        "MUSHROOM" => "skull_mushroom",
        "KRAKEN" => "skull_alien",
        "OWL" => "skull_generic",
        "CLOWN" => "skull_clown",
        "ANIME_GIRL" => "skull_anime",
        "HITLER" => "skull_hitler",
        "TANK" => "skull_tank",
        "CYCLOPEAN" => "skull_cyclops",
        "BAT" => "skull_canid",
        "FALCON" | "IBIS" | "VULTURE" | "RAVEN" | "HAWK" | "EAGLE" | "CROW" | "CONDOR"
        | "HERON" | "PELICAN" | "ALBATROSS" | "HUMMINGBIRD" | "BENNU" | "STRIX" | "GARUDA"
        | "BIRD" | "PIGEON" | "SPARROW" => "skull_avian",
        "SIREN" | "HARPY" => "skull_humanoid",
        _ => match plan {
            "ARACHNID" => "skull_spider",
            "INSECT" => "skull_insect",
            "HUMANOID" | "FLOATING" => "skull_humanoid",
            "WINGED" => "skull_avian",
            "SERPENT" => "skull_reptile",
            _ => "skull_generic",
        },
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Triangle {
    pub(crate) a: V3,
    pub(crate) b: V3,
    pub(crate) c: V3,
    pub(crate) color: [u8; 4],
    pub(crate) local: [V3; 3],
    pub(crate) texture: TextureKind,
    pub(crate) seed: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextureKind {
    None,
    Flesh,
    Fur,
    Scales,
    Chitin,
    Bone,
    Stone,
    Slime,
    Spectral,
    Other,
    Bark,
    OrganicScales,
    OrganicFur,
}

pub(crate) fn texture_kind(material: &str, role: &str) -> TextureKind {
    if !matches!(
        role,
        "base" | "shell" | "muzzle" | "ear" | "membrane" | "cap" | "gill" | "tread"
    ) {
        return TextureKind::None;
    }
    match material {
        "FLESH" | "FLESH_ASH" | "FLESH_OCHRE" | "FLESH_RUST" | "SKIN_PALE" | "TUMOR" | "ROTTEN"
        | "ALIEN_SKIN" | "GUNGAN_SKIN" | "MAKEUP" => TextureKind::Flesh,
        "FUR" | "FUR_GOLD" | "FUR_WHITE" | "FUR_DARK" | "FUR_BROWN" | "HAIR_DARK" => {
            TextureKind::Fur
        }
        material if material.starts_with("FEATHER") => TextureKind::Fur,
        "SCALES" | "SCALES_RUST" | "SCALES_BLUE" | "SCALES_OCHRE" | "SAND_HIDE" | "SHARK_HIDE"
        | "ORCA_HIDE" | "MEMBRANE" => TextureKind::Scales,
        "CHITIN" | "CHITIN_UMBER" | "CHITIN_JADE" | "CHITIN_BLUE" => TextureKind::Chitin,
        "BONE" | "HORN" => TextureKind::Bone,
        "STONE" => TextureKind::Stone,
        "SLIME" | "SLIME_AMBER" | "SLIME_BLUE" | "DEEP_SEA" | "FUNGUS" => TextureKind::Slime,
        "SPECTRAL" | "SMOKE" | "STORM" => TextureKind::Spectral,
        "METAL" | "WOOD" | "LEAF" | "WRAPPING" => TextureKind::Other,
        _ => TextureKind::None,
    }
}

fn hash_lattice(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x9e37_79b1)
        ^ (y as u32).wrapping_mul(0x85eb_ca77)
        ^ (z as u32).wrapping_mul(0xc2b2_ae3d);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h as f32 / u32::MAX as f32
}

fn noise(p: V3, seed: u32) -> f32 {
    let x = p.x.floor() as i32;
    let y = p.y.floor() as i32;
    let z = p.z.floor() as i32;
    let smooth = |v: f32| v * v * (3.0 - 2.0 * v);
    let fx = smooth(p.x - x as f32);
    let fy = smooth(p.y - y as f32);
    let fz = smooth(p.z - z as f32);
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let plane = |zi| {
        let a = mix(
            hash_lattice(x, y, zi, seed),
            hash_lattice(x + 1, y, zi, seed),
            fx,
        );
        let b = mix(
            hash_lattice(x, y + 1, zi, seed),
            hash_lattice(x + 1, y + 1, zi, seed),
            fx,
        );
        mix(a, b, fy)
    };
    mix(plane(z), plane(z + 1), fz)
}

fn texture_value(kind: TextureKind, local: V3, seed: u32) -> f32 {
    if kind == TextureKind::None {
        return 0.0;
    }
    if kind == TextureKind::OrganicFur {
        let broad = noise(local * 2.4, seed) - 0.5;
        let strands = noise(V3::new(local.x * 15.0, local.y * 3.0, local.z * 15.0), seed) - 0.5;
        return broad * 0.25 + strands * 0.10;
    }
    if kind == TextureKind::Bark {
        let grain = (local.x * 39.0 + noise(local * 3.0, seed) * 5.0).sin();
        return grain * 0.19
            + (noise(V3::new(local.x * 12.0, local.y * 2.0, local.z * 12.0), seed) - 0.5) * 0.35;
    }
    if kind == TextureKind::OrganicScales {
        let row = (local.y * 12.0).floor();
        let x = (local.x * 11.0 + row * 0.5).fract().abs();
        let y = (local.y * 12.0).fract().abs();
        return if (x - 0.5).powi(2) + (y - 0.3).powi(2) > 0.32 {
            -0.22
        } else {
            0.06
        };
    }
    let p = match kind {
        TextureKind::Fur => V3::new(local.x * 12.0, local.y * 4.0, local.z * 12.0),
        TextureKind::Scales | TextureKind::Chitin => local * 9.0,
        TextureKind::Bone => local * 7.0,
        TextureKind::Stone => local * 10.0,
        TextureKind::Slime => local * 5.0,
        _ => local * 6.0,
    };
    let broad = noise(p, seed) - 0.5;
    let fine = noise(p * 2.7, seed ^ 0xa341_316c) - 0.5;
    let amount = match kind {
        TextureKind::None => unreachable!(),
        TextureKind::Fur | TextureKind::Stone | TextureKind::Slime => 0.95,
        TextureKind::Scales | TextureKind::Chitin => 0.8,
        TextureKind::Spectral => 0.7,
        _ => 0.6,
    };
    (broad * 0.8 + fine * 0.35) * amount
}

fn role_color(role: &str, material: &str) -> [u8; 4] {
    let (dark, mid, high) = render::colors(material);
    match role {
        "hair_ridge" => [72, 58, 51, 255],
        "robot_metal" => [143, 157, 165, 255],
        "robot_eye" => [89, 177, 202, 255],
        "face_skin" => mid,
        "face_tusk" => [238, 234, 215, 255],
        "helmet_white" => [220, 225, 230, 255],
        "helmet_black" => [16, 20, 25, 255],
        "skin_soft" => [231, 180, 151, 255],
        "heart" => [242, 79, 127, 255],
        "eye_white" => [238, 234, 215, 255],
        "eye_dark" => [24, 24, 31, 255],
        "base" | "ear" | "muzzle" => mid,
        "jaw" | "brow" | "ridge" => high,
        "shell" => render::colors(material).1,
        "socket" | "nose" | "mouth" => [24, 24, 31, 255],
        "eye" if material == "SKIN_PALE" => render::colors("EYE_DARK").0,
        "eye" => render::colors("ENERGY").2,
        "tooth" => render::colors("BONE").2,
        "horn" | "mandible" | "beak" | "hoof" => render::colors("HORN").1,
        "rib" => render::colors("HORN").0,
        "membrane" if material == "MEMBRANE" => render::colors("MEMBRANE").1,
        "membrane" => mid,
        "metal" if material == "UNDERSUIT_BLACK" => mid,
        "metal" => render::colors("METAL").1,
        "wood" => render::colors("WOOD").1,
        "cap" => render::colors("FUNGUS").1,
        "gill" => render::colors("FUNGUS").0,
        "spot" | "makeup" => render::colors("MAKEUP").2,
        "red" => render::colors("RED_CLOTH").2,
        "hair" => render::colors("HAIR_DARK").0,
        "moustache" => [28, 25, 27, 255],
        "tread" => render::colors("METAL").0,
        _ => dark,
    }
}

fn depth_for(node: &Node) -> f32 {
    if let Some(z) = node.z {
        return z;
    }
    if node.id.starts_with("saber_fang_") || node.id.starts_with("tusk_") {
        return 5.0;
    }
    let number = node
        .id
        .rsplit('_')
        .next()
        .and_then(|v| v.parse::<usize>().ok());
    match node.kind.as_str() {
        "HAIR" => -1.5,
        "HEAD" | "NECK" | "MOUTH" | "HORN" | "FANG" | "EYE" => {
            let group = node
                .id
                .split('_')
                .nth(1)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let offset = match group {
                0 => 0.0,
                1 => -5.0,
                2 => 5.0,
                3 => -9.0,
                _ => 9.0,
            };
            offset + if node.kind == "EYE" { 0.9 } else { 0.0 }
        }
        "LIMB" | "HIP" | "FOOT" | "ARM" | "HAND" | "SHOULDER" => {
            let i = number.unwrap_or(0);
            let side = if i.is_multiple_of(2) { 1.0 } else { -1.0 };
            side * (3.3 + (i / 2) as f32 * 0.35)
        }
        "SHIN" => {
            let i = number.unwrap_or(0);
            if i.is_multiple_of(2) {
                3.3 + (i / 2) as f32 * 0.35
            } else {
                -3.3 - (i / 2) as f32 * 0.35
            }
        }
        "TAIL" => (number.unwrap_or(0) as f32 * 1.15).sin() * 2.2,
        "TREAD" => {
            if number.unwrap_or(0).is_multiple_of(2) {
                4.0
            } else {
                -4.0
            }
        }
        "WING" => {
            if number.unwrap_or(0).is_multiple_of(2) {
                3.0
            } else {
                -3.0
            }
        }
        _ => 0.0,
    }
}

fn resolved_depth(body: &Body, index: usize) -> f32 {
    let node = &body.nodes[index];
    if node.kind == "WEAPON" {
        node.z.unwrap_or_else(|| {
            node.parent
                .map(|p| depth_for(&body.nodes[p]) + 0.4)
                .unwrap_or_else(|| depth_for(node))
        })
    } else if node.kind == "CLAW" && node.z.is_none() {
        // A claw is attached to its foot/hand, not to the creature centerline.
        node.parent
            .map(|p| depth_for(&body.nodes[p]))
            .unwrap_or_else(|| depth_for(node))
    } else {
        depth_for(node)
    }
}

fn base_node_pose(body: &Body, index: usize, pose: &Pose) -> (V3, f32) {
    let node = &body.nodes[index];
    let (x, y, angle) = render::transform(node, index, pose, body.gravity);
    let mut z = resolved_depth(body, index);
    if ["SERPENT", "SAND_WORM"].contains(&pose.motion_archetype.as_str())
        && matches!(pose.state.as_str(), "MOVE" | "FAST_MOVE")
        && matches!(node.kind.as_str(), "TAIL" | "TORSO" | "NECK" | "HEAD")
    {
        let segment = node
            .id
            .rsplit('_')
            .next()
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(0.0);
        let amplitude = if node.kind == "TAIL" { 4.5 } else { 1.7 };
        z += (pose.phase * std::f32::consts::TAU - segment * 0.72).sin() * amplitude;
    }
    (V3::new(x, -y, z), angle)
}

fn aquatic_render_power(pose: &Pose) -> f32 {
    match pose.state.as_str() {
        "MOVE" | "FAST_MOVE" | "WATER_RISE" | "WATER_DIVE" => 1.0,
        "WATER_HOVER" => 0.35,
        "WATER_TURN" => (std::f32::consts::PI * pose.phase).sin().powi(2),
        "ATTACK" if pose.clip_id == "attack_water" => {
            (std::f32::consts::PI * pose.phase).sin().powi(2)
        }
        _ => 0.0,
    }
}

// Fish propel laterally, unlike cetaceans' vertical fluking. Rotate each
// animated tail edge instead of adding independent z offsets: joint lengths
// stay fixed and descendants (especially tail fins) inherit the displacement.
// This aquatic correction intentionally also applies to the V1 swim profile.
fn node_pose(body: &Body, index: usize, pose: &Pose) -> (V3, f32) {
    let base = base_node_pose(body, index, pose);
    if !matches!(
        pose.motion_archetype.as_str(),
        "AQUATIC" | "AQUATIC_MERMAID"
    ) || aquatic_render_power(pose) == 0.0
    {
        return base;
    }
    let node = &body.nodes[index];
    let Some(parent) = node.parent else {
        return base;
    };
    let parent_base = base_node_pose(body, parent, pose).0;
    let parent_animated = node_pose(body, parent, pose).0;
    let mut edge = base.0 - parent_base;
    if node.kind == "TAIL" {
        let segment = node
            .id
            .rsplit('_')
            .next()
            .and_then(|part| part.parse::<f32>().ok())
            .unwrap_or(index as f32);
        let amplitude = if pose.state == "FAST_MOVE" {
            0.62
        } else {
            0.42
        };
        let yaw = (pose.phase * std::f32::consts::TAU - segment * 0.65).sin()
            * amplitude
            * aquatic_render_power(pose);
        let (sine, cosine) = yaw.sin_cos();
        edge = V3::new(
            edge.x * cosine - edge.z * sine,
            edge.y,
            edge.x * sine + edge.z * cosine,
        );
    }
    (parent_animated + edge, base.1)
}

struct AssetOptions {
    flip_z: bool,
    seed: u32,
    irregularity: f32,
}

fn add_asset(
    triangles: &mut Vec<Triangle>,
    mesh: &MeshAsset,
    position: V3,
    scale: V3,
    angle: f32,
    material: &str,
    options: AssetOptions,
) {
    let ca = angle.cos();
    let sa = angle.sin();
    let locals: Vec<V3> = mesh
        .vertices
        .iter()
        .map(|v| V3::new(v[0], v[1], v[2]))
        .collect();
    let points: Vec<V3> = locals
        .iter()
        .map(|v| {
            let wobble = 1.0 + options.irregularity * (noise(*v * 5.0, options.seed) - 0.5);
            let x = v.x * scale.x * wobble;
            let y = v.y * scale.y * wobble;
            let z = v.z * scale.z * wobble * if options.flip_z { -1.0 } else { 1.0 };
            position + V3::new(x * ca - y * sa, x * sa + y * ca, z)
        })
        .collect();
    for (a, b, c, role) in &mesh.faces {
        triangles.push(Triangle {
            a: points[*a],
            b: points[*b],
            c: points[*c],
            color: role_color(role, material),
            local: [locals[*a], locals[*b], locals[*c]],
            texture: texture_kind(material, role),
            seed: options.seed,
        });
    }
}

fn sphere() -> &'static MeshAsset {
    static SPHERE: OnceLock<MeshAsset> = OnceLock::new();
    SPHERE.get_or_init(|| {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let rings = 5;
        let sides = 10;
        for i in 0..=rings {
            let latitude =
                -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / rings as f32;
            for j in 0..sides {
                let longitude = std::f32::consts::TAU * j as f32 / sides as f32;
                vertices.push([
                    latitude.cos() * longitude.cos(),
                    latitude.sin(),
                    latitude.cos() * longitude.sin(),
                ]);
            }
        }
        for i in 0..rings {
            for j in 0..sides {
                let a = i * sides + j;
                let b = i * sides + (j + 1) % sides;
                let c = (i + 1) * sides + j;
                let d = (i + 1) * sides + (j + 1) % sides;
                faces.push((a, b, c, "base".into()));
                faces.push((b, d, c, "base".into()));
            }
        }
        MeshAsset { vertices, faces }
    })
}

fn vortex_ring() -> &'static MeshAsset {
    static RING: OnceLock<MeshAsset> = OnceLock::new();
    RING.get_or_init(|| {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let sides = 18;
        for i in 0..sides {
            let angle = std::f32::consts::TAU * i as f32 / sides as f32;
            for (radius, y) in [(0.68, -0.32), (1.0, 0.15), (0.68, 0.32)] {
                vertices.push([
                    radius * angle.cos(),
                    y + i as f32 * 0.018,
                    radius * angle.sin(),
                ]);
            }
        }
        for i in 0..sides {
            let next = (i + 1) % sides;
            for band in 0..2 {
                let a = i * 3 + band;
                let b = next * 3 + band;
                let c = i * 3 + band + 1;
                let d = next * 3 + band + 1;
                faces.push((a, b, c, "base".into()));
                faces.push((b, d, c, "base".into()));
            }
        }
        MeshAsset { vertices, faces }
    })
}

fn flared_skirt() -> &'static MeshAsset {
    static SKIRT: OnceLock<MeshAsset> = OnceLock::new();
    SKIRT.get_or_init(|| {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let sides = 12;
        for (y, radius) in [(1.0, 0.55), (0.2, 0.78), (-1.0, 1.0)] {
            for i in 0..sides {
                let angle = std::f32::consts::TAU * i as f32 / sides as f32;
                vertices.push([radius * angle.cos(), y, radius * 0.65 * angle.sin()]);
            }
        }
        for band in 0..2 {
            for i in 0..sides {
                let a = band * sides + i;
                let b = band * sides + (i + 1) % sides;
                let c = (band + 1) * sides + i;
                let d = (band + 1) * sides + (i + 1) % sides;
                faces.push((a, b, c, "base".into()));
                faces.push((b, d, c, "base".into()));
            }
        }
        MeshAsset { vertices, faces }
    })
}

fn upright_torso() -> &'static MeshAsset {
    static TORSO: OnceLock<MeshAsset> = OnceLock::new();
    TORSO.get_or_init(|| {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let sections = [
            (-1.0, 0.18, 0.22),
            (-0.76, 0.58, 0.56),
            (-0.43, 0.67, 0.61),
            (-0.12, 0.58, 0.52),
            (0.27, 0.95, 0.77),
            (0.6, 1.0, 0.81),
            (0.88, 0.41, 0.43),
            (1.0, 0.19, 0.22),
        ];
        let sides = 12;
        for (y, width, depth) in sections {
            for side in 0..sides {
                let angle = std::f32::consts::TAU * side as f32 / sides as f32;
                vertices.push([width * angle.cos(), y, depth * angle.sin()]);
            }
        }
        for ring in 0..sections.len() - 1 {
            for side in 0..sides {
                let a = ring * sides + side;
                let b = ring * sides + (side + 1) % sides;
                let c = (ring + 1) * sides + side;
                let d = (ring + 1) * sides + (side + 1) % sides;
                faces.push((a, b, c, "base".into()));
                faces.push((b, d, c, "base".into()));
            }
        }
        MeshAsset { vertices, faces }
    })
}

fn add_connector(triangles: &mut Vec<Triangle>, a: V3, b: V3, radius: f32, color: [u8; 4]) {
    let direction = b - a;
    if direction.length() < 0.2 {
        return;
    }
    let axis = direction.normalized();
    let reference = if axis.z.abs() < 0.85 {
        V3::new(0.0, 0.0, 1.0)
    } else {
        V3::new(0.0, 1.0, 0.0)
    };
    let side = axis.cross(reference).normalized();
    let up = axis.cross(side).normalized();
    for i in 0..8 {
        let t0 = std::f32::consts::TAU * i as f32 / 8.0;
        let t1 = std::f32::consts::TAU * (i + 1) as f32 / 8.0;
        let offset0 = (side * t0.cos() + up * t0.sin()) * radius;
        let offset1 = (side * t1.cos() + up * t1.sin()) * radius;
        triangles.push(Triangle {
            a: a + offset0,
            b: a + offset1,
            c: b + offset0,
            color,
            local: [V3::default(); 3],
            texture: TextureKind::None,
            seed: 0,
        });
        triangles.push(Triangle {
            a: a + offset1,
            b: b + offset1,
            c: b + offset0,
            color,
            local: [V3::default(); 3],
            texture: TextureKind::None,
            seed: 0,
        });
    }
}

fn v2_shade_tube(triangles: &mut Vec<Triangle>, a: V3, b: V3, start_radius: f32, end_radius: f32) {
    let axis = (b - a).normalized();
    let reference = if axis.z.abs() < 0.85 {
        V3::new(0.0, 0.0, 1.0)
    } else {
        V3::new(0.0, 1.0, 0.0)
    };
    let side = axis.cross(reference).normalized();
    let up = axis.cross(side).normalized();
    for i in 0..12 {
        let t0 = std::f32::consts::TAU * i as f32 / 12.0;
        let t1 = std::f32::consts::TAU * (i + 1) as f32 / 12.0;
        let o0 = side * t0.cos() + up * t0.sin();
        let o1 = side * t1.cos() + up * t1.sin();
        for [a, b, c] in [
            [
                a + o0 * start_radius,
                a + o1 * start_radius,
                b + o0 * end_radius,
            ],
            [
                a + o1 * start_radius,
                b + o1 * end_radius,
                b + o0 * end_radius,
            ],
        ] {
            triangles.push(Triangle {
                a,
                b,
                c,
                color: [0, 0, 0, 255],
                local: [V3::default(); 3],
                texture: TextureKind::None,
                seed: 0,
            });
        }
    }
    // Joint caps share the tube radius; they close bend seams without bead bulges.
    let first = triangles.len();
    add_asset(
        triangles,
        sphere(),
        b,
        V3::new(end_radius, end_radius, end_radius),
        0.0,
        "FLESH",
        AssetOptions {
            flip_z: false,
            seed: 0,
            irregularity: 0.0,
        },
    );
    for t in &mut triangles[first..] {
        t.color = [0, 0, 0, 255];
        t.texture = TextureKind::None;
    }
}

fn v2_add_connector(
    triangles: &mut Vec<Triangle>,
    a: V3,
    b: V3,
    radius: f32,
    material: &str,
    seed: u32,
) {
    let direction = b - a;
    if direction.length() < 0.2 {
        return;
    }
    let axis = direction.normalized();
    let reference = if axis.z.abs() < 0.85 {
        V3::new(0.0, 0.0, 1.0)
    } else {
        V3::new(0.0, 1.0, 0.0)
    };
    let side = axis.cross(reference).normalized();
    let up = axis.cross(side).normalized();
    let profiles = [(0.0, 0.8), (0.28, 1.12), (0.66, 0.96), (1.0, 0.65)];
    let mut points = Vec::new();
    let mut locals = Vec::new();
    for (distance, girth) in profiles {
        for i in 0..10 {
            let theta = std::f32::consts::TAU * i as f32 / 10.0;
            let local = V3::new(
                theta.cos() * girth,
                distance * 2.0 - 1.0,
                theta.sin() * girth,
            );
            points.push(a + direction * distance + (side * local.x + up * local.z) * radius);
            locals.push(local);
        }
    }
    for band in 0..3 {
        for side in 0..10 {
            let a = band * 10 + side;
            let b = band * 10 + (side + 1) % 10;
            for [a, b, c] in [[a, b, a + 10], [b, b + 10, a + 10]] {
                triangles.push(Triangle {
                    a: points[a],
                    b: points[b],
                    c: points[c],
                    local: [locals[a], locals[b], locals[c]],
                    color: render::colors(material).1,
                    texture: if material == "WOOD" {
                        TextureKind::Bark
                    } else {
                        if texture_kind(material, "base") == TextureKind::Fur {
                            TextureKind::OrganicFur
                        } else {
                            texture_kind(material, "base")
                        }
                    },
                    seed,
                });
            }
        }
    }
}

// V2 meshes are deliberately separate from MESHES: V1 asset bytes and recipe
// fingerprints remain stable. These surfaces live in node-local coordinates,
// so details move with their parent rather than drifting in animation frames.
fn visual_v2(spec: &MonsterSpec) -> bool {
    spec.features.iter().any(|feature| feature == "PROFILE_V2")
}

fn v2_profile(sections: &[(f32, f32, f32)], flutes: f32) -> MeshAsset {
    let sides = 16;
    let mut mesh = MeshAsset {
        vertices: Vec::new(),
        faces: Vec::new(),
    };
    for &(y, width, depth) in sections {
        for side in 0..sides {
            let theta = std::f32::consts::TAU * side as f32 / sides as f32;
            let ridge = 1.0 + flutes * (theta * 8.0 + y * 0.45).cos();
            mesh.vertices
                .push([width * theta.cos() * ridge, y, depth * theta.sin() * ridge]);
        }
    }
    for band in 0..sections.len() - 1 {
        for side in 0..sides {
            let a = band * sides + side;
            let b = band * sides + (side + 1) % sides;
            let c = a + sides;
            let d = b + sides;
            let role = if flutes > 0.1 && side % 2 == 0 {
                "ridge"
            } else if band == 1 {
                "jaw"
            } else {
                "base"
            };
            mesh.faces.push((a, b, c, role.into()));
            mesh.faces.push((b, d, c, role.into()));
        }
    }
    // Close both ends, including profiles whose terminals have nonzero width.
    for (ring, y, reverse) in [
        (0, sections[0].0, true),
        (sections.len() - 1, sections[sections.len() - 1].0, false),
    ] {
        let center = mesh.vertices.len();
        mesh.vertices.push([0.0, y, 0.0]);
        for side in 0..sides {
            let a = ring * sides + side;
            let b = ring * sides + (side + 1) % sides;
            mesh.faces.push(if reverse {
                (center, b, a, "base".into())
            } else {
                (center, a, b, "base".into())
            });
        }
    }
    mesh
}

fn v2_append_blob(mesh: &mut MeshAsset, center: V3, scale: V3, role: &str) {
    let offset = mesh.vertices.len();
    mesh.vertices.extend(sphere().vertices.iter().map(|v| {
        [
            center.x + v[0] * scale.x,
            center.y + v[1] * scale.y,
            center.z + v[2] * scale.z,
        ]
    }));
    mesh.faces.extend(
        sphere()
            .faces
            .iter()
            .map(|(a, b, c, _)| (a + offset, b + offset, c + offset, role.into())),
    );
}

fn v2_mesh(kind: &str) -> &'static MeshAsset {
    static MESH: OnceLock<Vec<MeshAsset>> = OnceLock::new();
    let meshes = MESH.get_or_init(|| {
        let muscle = v2_profile(
            &[
                (-1.0, 0.22, 0.25),
                (-0.72, 0.55, 0.57),
                (-0.25, 0.9, 0.85),
                (0.3, 1.0, 0.9),
                (0.72, 0.65, 0.65),
                (1.0, 0.28, 0.3),
            ],
            0.035,
        );
        let trunk = v2_profile(
            &[
                (-1.0, 1.0, 0.92),
                (-0.7, 0.79, 0.8),
                (-0.25, 0.62, 0.68),
                (0.35, 0.77, 0.74),
                (0.8, 0.61, 0.57),
                (1.0, 0.4, 0.4),
            ],
            0.19,
        );
        let tail = v2_profile(
            &[
                (-1.0, 0.18, 0.2),
                (-0.65, 0.44, 0.48),
                (-0.15, 0.72, 0.78),
                (0.45, 0.94, 0.92),
                (1.0, 0.8, 0.78),
            ],
            0.045,
        );
        let mut barrel = v2_profile(
            &[
                (-1.0, 0.36, 0.45),
                (-0.72, 0.74, 0.72),
                (-0.25, 0.88, 0.85),
                (0.3, 0.94, 0.92),
                (0.64, 1.0, 0.83),
                (0.9, 0.7, 0.64),
                (1.0, 0.35, 0.36),
            ],
            0.035,
        );
        for v in &mut barrel.vertices {
            let old_y = v[1];
            v[1] = v[0] + 0.10 * old_y;
            v[0] = old_y;
        }
        let mut frog = MeshAsset {
            vertices: Vec::new(),
            faces: Vec::new(),
        };
        v2_append_blob(
            &mut frog,
            V3::new(0.0, -0.05, 0.0),
            V3::new(1.05, 0.62, 1.0),
            "base",
        );
        v2_append_blob(
            &mut frog,
            V3::new(0.35, -0.36, 0.0),
            V3::new(0.8, 0.22, 0.92),
            "jaw",
        );
        for side in [-1.0, 1.0] {
            v2_append_blob(
                &mut frog,
                V3::new(0.1, 0.49, side * 0.64),
                V3::new(0.36, 0.4, 0.36),
                "base",
            );
            v2_append_blob(
                &mut frog,
                V3::new(0.3, 0.56, side * 0.84),
                V3::new(0.21, 0.25, 0.16),
                "eye_white",
            );
            v2_append_blob(
                &mut frog,
                V3::new(0.39, 0.57, side * 0.94),
                V3::new(0.1, 0.18, 0.06),
                "eye_dark",
            );
            v2_append_blob(
                &mut frog,
                V3::new(0.42, 0.65, side * 0.98),
                V3::new(0.045, 0.065, 0.025),
                "eye_white",
            );
        }
        // Ribbed fan, with a recessed central notch and two broad flukes.
        let mut fan = MeshAsset {
            vertices: vec![[0.0, 0.85, 0.0]],
            faces: Vec::new(),
        };
        for i in 0..=12 {
            let t = i as f32 / 12.0;
            let x = (t * 2.0 - 1.0) * 1.2;
            let y = -0.95 + 0.8 * (1.0 - x.abs()).max(0.0).powi(3);
            fan.vertices
                .push([x, y, if i % 2 == 0 { 0.08 } else { -0.08 }]);
            if i > 0 {
                fan.faces.push((
                    0,
                    i,
                    i + 1,
                    if i % 2 == 0 { "ridge" } else { "membrane" }.into(),
                ));
            }
        }
        let mut web = MeshAsset {
            vertices: vec![[-0.7, 0.0, 0.0]],
            faces: Vec::new(),
        };
        for (x, z) in [
            (0.4, -1.0),
            (1.15, -0.8),
            (0.7, -0.35),
            (1.4, 0.0),
            (0.7, 0.35),
            (1.15, 0.8),
            (0.4, 1.0),
        ] {
            web.vertices.push([x, -0.12, z]);
        }
        for i in 1..7 {
            web.faces.push((0, i, i + 1, "membrane".into()));
        }
        let crown = v2_profile(
            &[
                (-1.0, 0.1, 0.1),
                (-0.6, 0.8, 0.7),
                (-0.1, 1.0, 0.95),
                (0.4, 0.85, 0.9),
                (1.0, 0.08, 0.08),
            ],
            0.22,
        );
        let mut tree_face = v2_profile(
            &[
                (-1.0, 0.5, 0.5),
                (-0.5, 0.8, 0.8),
                (0.3, 0.88, 0.88),
                (1.0, 0.6, 0.6),
            ],
            0.16,
        );
        for side in [-1.0, 1.0] {
            v2_append_blob(
                &mut tree_face,
                V3::new(0.5, 0.28, side * 0.7),
                V3::new(0.25, 0.2, 0.16),
                "eye_white",
            );
            v2_append_blob(
                &mut tree_face,
                V3::new(0.62, 0.28, side * 0.81),
                V3::new(0.11, 0.14, 0.05),
                "eye_dark",
            );
        }
        v2_append_blob(
            &mut tree_face,
            V3::new(0.77, -0.38, 0.0),
            V3::new(0.13, 0.09, 0.45),
            "mouth",
        );
        vec![
            muscle, trunk, tail, frog, fan, web, crown, tree_face, barrel,
        ]
    });
    &meshes[match kind {
        "trunk" => 1,
        "tail" => 2,
        "frog" => 3,
        "fan" => 4,
        "web" => 5,
        "crown" => 6,
        "tree_face" => 7,
        "barrel" => 8,
        _ => 0,
    }]
}

fn v2_skeletal_node(spec: &MonsterSpec, node: &Node) -> bool {
    visual_v2(spec)
        && (node_identity(spec, &node.id).1 == "SKELETON"
            || node.material == "BONE"
            || spec
                .features
                .iter()
                .any(|f| matches!(f.as_str(), "SKELETON" | "SKELETAL")))
}

fn v2_face_recipe(spec: &MonsterSpec, node: &Node) -> Option<&'static str> {
    if !visual_v2(spec) || node.kind != "HEAD" {
        return None;
    }
    let (_, primary) = node_identity(spec, &node.id);
    let affinity = node
        .id
        .strip_prefix("hybrid_head_")
        .and_then(|index| index.parse::<usize>().ok())
        .and_then(|index| spec.secondary_affinities.get(index))
        .map(String::as_str)
        .unwrap_or(primary);
    match affinity {
        "SKELETON" => return Some("skeleton"),
        "JELLYFISH" | "TORNADO" => return None,
        "MERMAID" => return Some("mermaid"),
        "CENTAUR" | "SATYR" | "FAUN" | "HARPY" | "SIREN" | "MERMAN" | "FISHMAN" | "SPHINX"
        | "VALKYRIE" | "NAGA" | "LAMIA" | "ECHIDNA" => return Some("human"),
        "BIGFOOT" | "YETI" | "GORILLA" | "VANARA" => return Some("ape"),
        "ROBOT" => return Some("robot"),
        "ALIEN" => return Some("alien"),
        _ => {}
    }
    match skull_recipe(spec, node) {
        "skull_stormtrooper" => Some("helmet"),
        "skull_goblin" => Some("orc"),
        "skull_cyclops" => Some("cyclops"),
        "skull_humanoid" | "skull_anime" | "skull_clown" | "skull_hitler" => Some("human"),
        _ => None,
    }
}

fn v2_mesh_override(spec: &MonsterSpec, node: &Node) -> Option<&'static MeshAsset> {
    if !visual_v2(spec) {
        return None;
    }
    if node.kind == "BONE_SPUR" {
        return Some(v2_skeleton::mesh("long_bone"));
    }
    if v2_skeletal_node(spec, node) {
        let part = match node.kind.as_str() {
            "HEAD" => Some("skull"),
            "TORSO" => Some("ribs"),
            "HAND" => Some("hand"),
            "FOOT" => Some("foot"),
            "LOBE" if node.id.contains("pelvis") => Some("pelvis"),
            "LIMB" | "ARM" | "SHIN" | "NECK" | "SHOULDER" | "HIP" | "TAIL" => Some("long_bone"),
            _ => None,
        };
        if let Some(part) = part {
            return Some(v2_skeleton::mesh(part));
        }
    }
    if node.id == "v2_hybrid_chest" {
        return Some(upright_torso());
    }
    if let Some(face) = v2_face_recipe(spec, node) {
        return Some(v2_faces::mesh(face));
    }
    let (plan, affinity) = node_identity(spec, &node.id);
    match (affinity, node.kind.as_str()) {
        ("FISH", "HEAD") => Some(v2_faces::mesh("fish")),
        ("CENTAUR", "FOOT") => Some(asset("part_hoof")),
        ("FROG" | "TOAD", "HEAD") => Some(v2_mesh("frog")),
        ("FROG" | "TOAD", "FOOT" | "HAND") => Some(v2_mesh("web")),
        ("TREE_MONSTER", "HEAD") => Some(v2_mesh("tree_face")),
        ("TREE_MONSTER", "TORSO" | "BRANCH" | "ROOT" | "LIMB" | "ARM" | "SHIN") => {
            Some(v2_mesh("trunk"))
        }
        ("TREE_MONSTER", "LOBE") | (_, "CANOPY") => Some(v2_mesh("crown")),
        ("MERMAID" | "MERMAN", "TAIL") => Some(v2_mesh("tail")),
        ("MERMAID" | "MERMAN", "FIN" | "FLUKE") => Some(v2_mesh("fan")),
        (_, "TORSO") if matches!(plan, "QUADRUPED" | "THEROPOD" | "INSECT" | "ARACHNID") => {
            Some(v2_mesh("barrel"))
        }
        (_, "LIMB" | "ARM" | "SHIN" | "HIP" | "SHOULDER" | "ABDOMEN" | "BODY_SEGMENT") => {
            Some(v2_mesh("muscle"))
        }
        _ => None,
    }
}

fn v2_lure_heart() -> &'static MeshAsset {
    static HEART: OnceLock<MeshAsset> = OnceLock::new();
    HEART.get_or_init(|| {
        let mut mesh = MeshAsset {
            vertices: Vec::new(),
            faces: Vec::new(),
        };
        // Crossed cards keep the harmless emote readable in all four views.
        for sideways in [false, true] {
            let center = mesh.vertices.len();
            mesh.vertices.push([0.0, 0.0, 0.0]);
            for i in 0..24 {
                let t = std::f32::consts::TAU * i as f32 / 24.0;
                let x = t.sin().powi(3);
                let y = (13.0 * t.cos()
                    - 5.0 * (2.0 * t).cos()
                    - 2.0 * (3.0 * t).cos()
                    - (4.0 * t).cos())
                    / 17.0;
                mesh.vertices
                    .push(if sideways { [0.0, y, x] } else { [x, y, 0.0] });
            }
            for i in 0..24 {
                mesh.faces.push((
                    center,
                    center + 1 + i,
                    center + 1 + (i + 1) % 24,
                    "heart".into(),
                ));
            }
        }
        mesh
    })
}

fn v2_friendly_pose(spec: &MonsterSpec, pose: &Pose) -> bool {
    visual_v2(spec)
        && spec.features.iter().any(|f| f == "FRIENDLY_LURE")
        && match pose.state.as_str() {
            "ATTACK" | "PHASE" | "HURT" | "DEATH" => false,
            "REVEAL" => pose.phase < 0.45,
            _ => true,
        }
}

fn v2_friendly_head(mesh: &MeshAsset) -> MeshAsset {
    MeshAsset {
        vertices: mesh.vertices.clone(),
        faces: mesh
            .faces
            .iter()
            .filter(|(_, _, _, role)| !matches!(role.as_str(), "tooth" | "horn" | "mandible"))
            .map(|(a, b, c, role)| {
                (
                    *a,
                    *b,
                    *c,
                    match role.as_str() {
                        "socket" => "eye_white",
                        "eye" => "eye_dark",
                        "brow" => "base",
                        other => other,
                    }
                    .into(),
                )
            })
            .collect(),
    }
}

// One coherent seed transform is shared by every material and its quantization
// palette. This preserves contrast and authored material relationships.
fn v2_gains(spec: &MonsterSpec, seed: u64) -> [f32; 3] {
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ 0xe173_a5c8_42bf_900d);
    if matches!(
        spec.affinity.as_str(),
        "FROG" | "TOAD" | "ALIEN" | "SLIME" | "MUSHROOM"
    ) {
        return [
            rng.random_range(0.82_f32..1.16),
            rng.random_range(0.82_f32..1.16),
            rng.random_range(0.82_f32..1.16),
        ];
    }
    // Natural coats vary in value and warmth, not arbitrary channel rotation.
    // This keeps wolves brown/gray, mammoths umber and bark earthy.
    let value = rng.random_range(0.88_f32..1.12);
    let warmth = rng.random_range(-0.055_f32..0.055);
    [value * (1.0 + warmth), value, value * (1.0 - warmth)]
}

fn v2_tint(color: [u8; 4], gains: [f32; 3], friendly: bool) -> [u8; 4] {
    let mut out = color;
    for channel in 0..3 {
        let value = color[channel] as f32 * gains[channel];
        out[channel] = (if friendly { value * 0.84 + 31.0 } else { value })
            .clamp(0.0, 255.0)
            .round() as u8;
    }
    out
}

fn v2_surface_color(
    spec: &MonsterSpec,
    color: [u8; 4],
    gains: [f32; 3],
    friendly: bool,
) -> [u8; 4] {
    let mut color = v2_tint(color, gains, friendly);
    let has = |feature: &str| spec.features.iter().any(|f| f == feature);
    let ghost = has("GHOST");
    if has("SHADE") {
        color = [0, 0, 0, 255];
    } else if ghost || has("GHOUL") || spec.affinity == "GHOUL" {
        let luma =
            (color[0] as f32 * 0.30 + color[1] as f32 * 0.59 + color[2] as f32 * 0.11).min(255.0);
        if luma < 60.0 {
            color = [25, 40, 42, 255];
        } else if ghost {
            color = [
                (95.0 + luma * 0.30) as u8,
                (135.0 + luma * 0.30) as u8,
                (160.0 + luma * 0.30) as u8,
                255,
            ];
        } else {
            color = [
                (65.0 + luma * 0.42) as u8,
                (87.0 + luma * 0.44) as u8,
                (78.0 + luma * 0.40) as u8,
                255,
            ];
        }
    }
    // Transparency is an actual RGBA value, independent of combat immunity.
    // A ghoul alone remains opaque; GHOST is the explicit translucent modifier.
    if ghost && !has("SHADE") {
        color[3] = 128;
    }
    color
}

fn v2_finish_alpha(spec: &MonsterSpec, pose: &Pose, image: &mut RgbaImage) {
    if !visual_v2(spec) {
        return;
    }
    let has = |tag: &str| spec.features.iter().any(|f| f == tag);
    if has("SHADE") {
        return;
    }
    if spec.mount.is_some() && (has("GHOST") || v2_has_nested_ghost(spec)) {
        for pixel in image.pixels_mut() {
            if pose.state != "DEATH" && pixel[3] == 128 {
                pixel[3] = 97;
            } else if pose.state == "DEATH" && pixel[3] == 64 {
                pixel[3] = 48;
            }
        }
        return;
    }
    let phase = pose.state == "PHASE" && (has("GHOST") || has("GHOUL") || spec.affinity == "GHOUL");
    let ghost = has("GHOST");
    if !phase && !ghost {
        return;
    }
    for pixel in image.pixels_mut().filter(|p| p[3] > 0) {
        pixel[3] = if phase {
            64
        } else {
            ((pixel[3] as u16 * 97) / 128).min(97) as u8
        };
    }
}

fn v2_node_material<'a>(spec: &MonsterSpec, node: &'a Node, seed: u64) -> &'a str {
    if v2_skeletal_node(spec, node) {
        return "BONE";
    }
    if !visual_v2(spec) || !matches!(node_identity(spec, &node.id).1, "MERMAID" | "MERMAN") {
        return &node.material;
    }
    let ordinary = node.material.starts_with("SCALES")
        || node.material.starts_with("FLESH")
        || matches!(node.material.as_str(), "MEMBRANE" | "SKIN_PALE");
    if !ordinary {
        return &node.material;
    }
    if matches!(
        node.kind.as_str(),
        "HEAD" | "NECK" | "TORSO" | "ARM" | "SHOULDER" | "HAND"
    ) {
        "SKIN_PALE"
    } else if matches!(node.kind.as_str(), "TAIL" | "FIN" | "FLUKE") || node.id.contains("pelvis") {
        if seed.is_multiple_of(2) {
            "SCALES_BLUE"
        } else {
            "SCALES"
        }
    } else {
        &node.material
    }
}

fn scene_palette(spec: &MonsterSpec, body: &Body, seed: u64) -> Vec<[u8; 4]> {
    let mut colors = palette(body);
    if visual_v2(spec) {
        let friendly = spec.features.iter().any(|f| f == "FRIENDLY_LURE");
        // Facial/emote colors precede generic materials in the 48-color budget.
        let original = colors;
        colors = vec![[238, 234, 215, 255], [24, 24, 31, 255]];
        // Normal humanoid faces also need their own skin and helmet ramps.
        if body
            .nodes
            .iter()
            .any(|node| v2_face_recipe(spec, node).is_some())
        {
            colors.extend([
                [231, 180, 151, 255],
                [145, 94, 78, 255],
                [255, 220, 190, 255],
                [220, 225, 230, 255],
                [132, 144, 155, 255],
                [16, 20, 25, 255],
                [72, 58, 51, 255],
                [143, 157, 165, 255],
                [89, 177, 202, 255],
            ]);
        }
        if friendly {
            colors.extend([
                [231, 180, 151, 255],
                [145, 94, 78, 255],
                [255, 220, 190, 255],
                [242, 79, 127, 255],
                [155, 40, 79, 255],
                [255, 146, 179, 255],
            ]);
        }
        for node in &body.nodes {
            let material = v2_node_material(spec, node, seed);
            if material != node.material {
                let (dark, mid, high) = render::colors(material);
                for c in [dark, mid, high] {
                    render::add_palette_color(&mut colors, c);
                }
            }
        }
        for color in original {
            render::add_palette_color(&mut colors, color);
        }
        let base_colors = colors.clone();
        let gains = v2_gains(spec, seed);
        for color in &mut colors {
            *color = v2_surface_color(spec, *color, gains, friendly);
        }
        if spec.mount.is_some() {
            for node in &body.nodes {
                let local = v2_surface_spec(spec, &node.id);
                let gains = v2_gains(local, seed);
                let friendly = local.features.iter().any(|f| f == "FRIENDLY_LURE");
                colors.extend(
                    base_colors
                        .iter()
                        .map(|c| v2_surface_color(local, *c, gains, friendly)),
                );
            }
        }
        colors.sort_unstable();
        colors.dedup();
        colors.truncate(255);
    }
    colors
}

#[path = "v2/shade_mesh.rs"]
mod v2_shade_mesh;

fn scene(spec: &MonsterSpec, body: &Body, pose: &Pose, seed: u64) -> Vec<Triangle> {
    let positions: Vec<(V3, f32)> = body
        .nodes
        .iter()
        .enumerate()
        .map(|(i, _)| node_pose(body, i, pose))
        .collect();
    let mut triangles = Vec::with_capacity(body.nodes.len() * 100);
    let mut texture_rng = ChaCha8Rng::seed_from_u64(seed);
    let mut shade_strands = Vec::new();
    for (i, node) in body.nodes.iter().enumerate() {
        let connector_start = triangles.len();
        if let Some(parent) = node.parent {
            if visual_v2(spec) && node.id.starts_with("v2_shade_tendril_") {
                let start = if body.nodes[parent].id.starts_with("v2_shade_tendril_") {
                    body.nodes[parent].rx
                } else {
                    node.rx * 1.6
                };
                let end = node.rx * if node.id.ends_with("_4") { 0.35 } else { 1.0 };
                v2_shade_tube(
                    &mut shade_strands,
                    positions[parent].0,
                    positions[i].0,
                    start,
                    end,
                );
                continue;
            }
            if !(node.kind == "TORSO" && body.nodes[parent].kind == "TORSO")
                && !["EYE", "MOUTH", "HORN", "FANG", "CLAW", "WEAPON", "WING"]
                    .contains(&node.kind.as_str())
            {
                let radius = node.rx.min(node.ry).min(body.nodes[parent].rx) * 0.6;
                if v2_skeletal_node(spec, node) {
                    add_connector(
                        &mut triangles,
                        positions[parent].0,
                        positions[i].0,
                        (radius * 0.22).max(0.15),
                        [238, 234, 215, 255],
                    );
                } else if visual_v2(spec) {
                    v2_add_connector(
                        &mut triangles,
                        positions[parent].0,
                        positions[i].0,
                        radius.max(0.25),
                        v2_node_material(spec, node, seed),
                        (seed as u32).wrapping_add(i as u32),
                    );
                } else {
                    add_connector(
                        &mut triangles,
                        positions[parent].0,
                        positions[i].0,
                        radius.max(0.25),
                        render::colors(&node.material).0,
                    );
                }
            }
        }
        v2_local_surface(&mut triangles[connector_start..], spec, node, seed);
    }
    for (i, node) in body.nodes.iter().enumerate() {
        let node_seed = texture_rng.random::<u32>();
        if visual_v2(spec)
            && (node.id.starts_with("v2_shade_tendril_")
                || (node.kind == "WEAPON" && spec.features.iter().any(|f| f == "SHADE")))
        {
            continue;
        }
        if v2_friendly_pose(spec, pose) && matches!(node.kind.as_str(), "FANG" | "CLAW" | "STINGER")
        {
            continue;
        }
        if node.kind == "MOUTH" || node.kind == "EYE" {
            continue;
        }
        let (position, angle) = positions[i];
        let (node_plan, node_affinity) = node_identity(spec, &node.id);
        let radius_z = node.rz.unwrap_or_else(|| {
            if node.kind == "TORSO" {
                if node_plan == "HUMANOID"
                    || (node_plan == "WINGED"
                        && !matches!(node_affinity, "DRAGON" | "WYVERN")
                        && (!node.material.starts_with("FEATHER")
                            || matches!(node_affinity, "SIREN" | "HARPY"))
                        && node_affinity != "BAT")
                {
                    node.rx * 0.57
                } else {
                    node.ry * 0.85
                }
            } else {
                node.rx.min(node.ry) * 0.85
            }
        });
        let (mesh, scale, material) = match node.kind.as_str() {
            "HEAD" => (
                asset(skull_recipe(spec, node)),
                V3::new(node.rx, node.ry, node.rz.unwrap_or(node.ry * 0.88)),
                node.material.as_str(),
            ),
            "HORN" | "ANTLER" => (
                asset("part_horn"),
                V3::new(node.rx * 0.9, node.ry * 0.85, node.rx),
                node.material.as_str(),
            ),
            "CLAW" | "STINGER" => (
                asset("part_claw"),
                V3::new(node.rx, node.ry, node.rx),
                node.material.as_str(),
            ),
            "FANG" | "TUSK" => (
                asset("part_fang"),
                V3::new(node.rx, node.ry, node.rx),
                node.material.as_str(),
            ),
            "VORTEX" => (
                vortex_ring(),
                V3::new(node.rx, node.ry, node.rx),
                node.material.as_str(),
            ),
            "FIN" => (
                asset("part_wing"),
                V3::new(node.rx, node.ry, node.rx * 0.4),
                node.material.as_str(),
            ),
            "SKIRT" => (
                flared_skirt(),
                V3::new(node.rx, node.ry, node.rx * 0.65),
                node.material.as_str(),
            ),
            "WING" if node_plan == "INSECT" || node_affinity == "HORNET" => (
                asset("part_wing_insect"),
                V3::new(node.rx, node.ry, node.rz.unwrap_or(node.rx)),
                "MEMBRANE",
            ),
            "ELYTRON" => (
                sphere(),
                V3::new(node.rx, node.ry, radius_z),
                node.material.as_str(),
            ),
            "WING" => (
                asset(if node.material == "MEMBRANE" {
                    "part_wing"
                } else {
                    "part_wing_feather"
                }),
                V3::new(node.rx, node.ry, node.rx * 0.55),
                node.material.as_str(),
            ),
            "SHELL" => (
                asset("part_shell"),
                V3::new(node.rx, node.ry, node.rx * 0.75),
                node.material.as_str(),
            ),
            "WEAPON" => (
                asset(match spec.attack.concept.as_str() {
                    "SWORD" => "part_weapon_sword",
                    "CLUB" => "part_weapon_club",
                    "AXE" => "part_weapon_axe",
                    "SPEAR" => "part_weapon_spear",
                    "BOW" => "part_weapon_bow",
                    "CHAINSAW" => "part_weapon_chainsaw",
                    _ => "part_weapon",
                }),
                V3::new(node.rx, node.ry * 1.2, node.ry * 1.2),
                node.material.as_str(),
            ),
            "TREAD" => (
                asset("part_tread"),
                V3::new(node.rx, node.ry, node.rx * 0.38),
                node.material.as_str(),
            ),
            "ROTOR" => (
                asset("part_rotor"),
                V3::new(node.rx, node.ry, node.rx),
                node.material.as_str(),
            ),
            "TORSO" if node_plan == "TANK" => (
                asset("part_tank_hull"),
                V3::new(node.rx, node.ry, radius_z),
                node.material.as_str(),
            ),
            "FOOT" if node_plan == "QUADRUPED" || matches!(node_affinity, "DRAGON" | "WYVERN") => (
                asset(
                    if [
                        "HORSE", "BULL", "GOAT", "RAM", "BOAR", "MOOSE", "BUFFALO", "RHINO",
                        "ELEPHANT", "MAMMOTH", "GIRAFFE", "ZEBRA", "ANTELOPE", "GAZELLE",
                    ]
                    .contains(&node_affinity)
                    {
                        "part_hoof"
                    } else {
                        "part_paw"
                    },
                ),
                V3::new(node.rx, node.ry, node.rx),
                node.material.as_str(),
            ),
            "TORSO"
                if node_plan == "HUMANOID"
                    || (node_plan == "WINGED"
                        && !matches!(node_affinity, "DRAGON" | "WYVERN")
                        && (!node.material.starts_with("FEATHER")
                            || matches!(node_affinity, "SIREN" | "HARPY"))
                        && node_affinity != "BAT") =>
            {
                (
                    upright_torso(),
                    V3::new(node.rx * 0.95, node.ry * 1.05, radius_z),
                    node.material.as_str(),
                )
            }
            _ => (
                sphere(),
                V3::new(node.rx, node.ry, radius_z),
                node.material.as_str(),
            ),
        };
        let selected_material = v2_node_material(spec, node, seed);
        let material = if selected_material != node.material {
            selected_material
        } else {
            material
        };
        let mesh = v2_mesh_override(spec, node).unwrap_or(mesh);
        let friendly_head = if node.kind == "HEAD" && v2_friendly_pose(spec, pose) {
            Some(v2_friendly_head(mesh))
        } else {
            None
        };
        let mesh = friendly_head.as_ref().unwrap_or(mesh);
        let mut scale = scale;
        if pose.motion_archetype == "AQUATIC_JELLY"
            && aquatic_render_power(pose) > 0.0
            && matches!(node.kind.as_str(), "HEAD" | "TORSO" | "LOBE")
        {
            let expansion = 1.0 + (pose.phase * std::f32::consts::TAU).sin() * 0.13;
            scale.x *= expansion;
            scale.z *= expansion;
            scale.y /= expansion * expansion;
        }
        if v2_skeletal_node(spec, node)
            && matches!(
                node.kind.as_str(),
                "LIMB" | "ARM" | "SHIN" | "HAND" | "FOOT"
            )
        {
            let bone_shape = 0.86 + hash_lattice(i as i32, 13, 7, seed as u32) * 0.28;
            scale.x *= bone_shape;
            scale.z /= bone_shape;
        }
        if visual_v2(spec) {
            if node.kind == "FIN" || node.kind == "FLUKE" {
                scale.z = node.rz.unwrap_or(node.rx * 0.25);
            }
            // Modest anatomically local allometry, stable throughout the atlas.
            let variation = 0.94 + hash_lattice(i as i32, 0, 0, seed as u32) * 0.12;
            if node.kind != "WEAPON" {
                scale.x *= variation;
                scale.z /= variation;
            }
        }
        if pose.state != "DEATH" && spec.is_blob() && matches!(node.kind.as_str(), "TORSO" | "LOBE")
        {
            let pulse = (pose.phase * std::f32::consts::TAU + i as f32 * 0.7).sin();
            scale.x *= 1.0 + pulse * 0.08;
            scale.y *= 1.0 - pulse * 0.1;
        }
        if pose.state != "DEATH"
            && spec.is_gastropod()
            && matches!(node.kind.as_str(), "TORSO" | "SOLE")
        {
            let stretch = (pose.phase * std::f32::consts::TAU).sin();
            let amount = if spec.affinity == "SLUG" { 0.18 } else { 0.10 };
            scale.x *= 1.0 + stretch * amount;
            scale.y *= 1.0 - stretch * amount * 0.7;
        }
        let triangle_start = triangles.len();
        add_asset(
            &mut triangles,
            mesh,
            position,
            scale,
            -angle,
            material,
            AssetOptions {
                flip_z: node.kind == "WING" && depth_for(node) < 0.0,
                seed: node_seed,
                irregularity: if matches!(
                    node.kind.as_str(),
                    "TORSO" | "LOBE" | "TAIL" | "SHELL" | "ABDOMEN" | "BODY_SEGMENT"
                ) {
                    0.08
                } else if matches!(node.kind.as_str(), "LIMB" | "ARM" | "LEG") {
                    0.06
                } else {
                    0.0
                },
            },
        );
        v2_local_surface(&mut triangles[triangle_start..], spec, node, seed);
        if visual_v2(spec) {
            for triangle in &mut triangles[triangle_start..] {
                if triangle.texture != TextureKind::None {
                    if material == "SKIN_PALE" && matches!(node_affinity, "MERMAID" | "MERMAN") {
                        triangle.texture = TextureKind::None;
                    } else if material == "WOOD" {
                        triangle.texture = TextureKind::Bark;
                    } else if material.starts_with("SCALES") {
                        triangle.texture = TextureKind::OrganicScales;
                    } else if triangle.texture == TextureKind::Fur {
                        triangle.texture = TextureKind::OrganicFur;
                    }
                }
            }
        }
    }
    if visual_v2(spec) && spec.features.iter().any(|f| f == "SHADE") {
        v2_shade_mesh::unwind(&mut triangles, body, pose, &positions);
    }
    triangles.extend(shade_strands);
    if visual_v2(spec) {
        let friendly = spec.features.iter().any(|f| f == "FRIENDLY_LURE");
        if friendly && pose.state == "LURE" && pose.phase >= 0.25 {
            if let Some(head) = body.nodes.iter().position(|node| node.kind == "HEAD") {
                let rise = (pose.phase - 0.25) * 3.0;
                let (origin, _) = positions[head];
                // Canopies and raised waving hands may sit above the head. Keep
                // the harmless emote outside the complete posed silhouette.
                let top = triangles
                    .iter()
                    .flat_map(|triangle| [triangle.a.y, triangle.b.y, triangle.c.y])
                    .fold(origin.y + body.nodes[head].ry, f32::max);
                let offset = V3::new(0.0, top - origin.y + 4.2 + rise, 0.0);
                add_asset(
                    &mut triangles,
                    v2_lure_heart(),
                    origin + offset,
                    V3::new(3.3, 3.3, 3.3),
                    0.0,
                    "RED_CLOTH",
                    AssetOptions {
                        flip_z: false,
                        seed: seed as u32,
                        irregularity: 0.0,
                    },
                );
            }
        }
        let gains = v2_gains(spec, seed);
        for triangle in &mut triangles {
            if spec.mount.is_none() || spec.features.iter().any(|f| f == "SHADE") {
                triangle.color = v2_surface_color(spec, triangle.color, gains, friendly);
            }
            if pose.state == "PHASE"
                && !spec.features.iter().any(|f| f == "SHADE")
                && (spec
                    .features
                    .iter()
                    .any(|f| matches!(f.as_str(), "GHOST" | "GHOUL"))
                    || spec.affinity == "GHOUL")
            {
                triangle.color[3] = 64;
            }
        }
    }
    triangles
}

#[derive(Clone, Copy)]
pub(crate) struct ScreenPoint {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) depth: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct Camera {
    pub(crate) sine: f32,
    pub(crate) cosine: f32,
    pub(crate) elevation_sine: f32,
    pub(crate) elevation_cosine: f32,
    pub(crate) scale: f32,
    pub(crate) origin_x: f32,
    pub(crate) origin_y: f32,
}

impl Camera {
    fn new(angle: f32, resolution: u32, supersample: f32) -> Self {
        let (sine, cosine) = angle.sin_cos();
        let (elevation_sine, elevation_cosine) = 0.28_f32.sin_cos();
        Self {
            sine,
            cosine,
            elevation_sine,
            elevation_cosine,
            scale: PIXELS_PER_UNIT * supersample,
            origin_x: resolution as f32 * 0.5,
            origin_y: resolution as f32 * 0.57,
        }
    }
}

pub(crate) fn project_with_camera(point: V3, camera: Camera) -> ScreenPoint {
    let front = point.x * camera.sine + point.z * camera.cosine;
    let right = point.x * camera.cosine - point.z * camera.sine;
    ScreenPoint {
        x: camera.origin_x + right * camera.scale,
        y: camera.origin_y
            + (-point.y * camera.elevation_cosine + front * camera.elevation_sine) * camera.scale,
        depth: front * camera.elevation_cosine + point.y * camera.elevation_sine,
    }
}

fn project(point: V3, angle: f32, resolution: u32, supersample: f32) -> ScreenPoint {
    project_with_camera(point, Camera::new(angle, resolution, supersample))
}

/// Expand framing to contain every posed triangle from every view, without
/// changing the shared pixel scale. Long tails, raised weapons and flying wings
/// must never be silently cropped by a species-name canvas heuristic.
pub fn required_frame_size(
    spec: &MonsterSpec,
    body: &Body,
    poses: &[Pose],
    minimum: u32,
    seed: u64,
) -> u32 {
    let cameras: Vec<_> = ANGLES_DEG
        .iter()
        .map(|a| Camera::new(a.to_radians(), 0, 1.0))
        .collect();
    let margin = 3.0;
    let mut required = minimum as f32;
    for pose in poses {
        for triangle in scene(spec, body, pose, seed) {
            for point in [triangle.a, triangle.b, triangle.c] {
                for camera in &cameras {
                    let p = project_with_camera(point, *camera);
                    required = required.max((p.x.abs() + margin) * 2.0);
                    required = required.max(if p.y < 0.0 {
                        (-p.y + margin) / 0.57
                    } else {
                        (p.y + margin) / 0.43
                    });
                }
            }
        }
    }
    // Whole tiles keep GPU/atlas allocation predictable while preserving scale.
    ((required.ceil() as u32).saturating_add(7) / 8) * 8
}

/// Node-only projection retained for callers supplying explicit lateral depth.
pub fn collider_views(node: &Node, size: u32) -> Vec<proto::ColliderView> {
    collider_views_at_depth(node, size, depth_for(node))
}

/// Neutral collider views use exactly the same attachment depth as mesh baking.
pub fn collider_views_for_body(body: &Body, index: usize, size: u32) -> Vec<proto::ColliderView> {
    collider_views_at_depth(&body.nodes[index], size, resolved_depth(body, index))
}

/// Exact per-frame attack origins, including physical joints, global root
/// translation/rotation and aquatic lateral motion. Legacy rest views are intact.
pub fn collider_views_for_pose(
    body: &Body,
    index: usize,
    pose: &Pose,
    size: u32,
) -> Vec<proto::ColliderView> {
    let position = node_pose(body, index, pose).0;
    let node = &body.nodes[index];
    let radius = (node.rx.min(node.ry) * PIXELS_PER_UNIT).max(0.5);
    ANGLES_DEG
        .iter()
        .enumerate()
        .map(|(direction_index, angle)| {
            let p = project(position, angle.to_radians(), size, 1.0);
            proto::ColliderView {
                direction_index: direction_index as u32,
                x: p.x,
                y: p.y,
                radius,
            }
        })
        .collect()
}

/// Approximate dynamic damage-receiver circles for an unwinding shade.
/// Centers and sampled ellipsoid surfaces use the exact rendering deformation.
/// Tendril nodes stay on their already animated contact graph.
pub fn collider_views_for_shade_receiver(
    body: &Body,
    index: usize,
    pose: &Pose,
    size: u32,
) -> Vec<proto::ColliderView> {
    if body.nodes[index].id.starts_with("v2_shade_tendril_") {
        return collider_views_for_pose(body, index, pose, size);
    }
    let positions: Vec<_> = (0..body.nodes.len())
        .map(|i| node_pose(body, i, pose))
        .collect();
    let Some(field) = v2_shade_mesh::Field::new(body, pose, &positions) else {
        return collider_views_for_pose(body, index, pose, size);
    };
    let node = &body.nodes[index];
    let center = positions[index].0;
    let mapped = field.map(center);
    let offsets = [
        V3::new(node.rx, 0.0, 0.0),
        V3::new(0.0, node.ry, 0.0),
        V3::new(0.0, 0.0, node.rz.unwrap_or(node.rx)),
    ];
    ANGLES_DEG
        .iter()
        .enumerate()
        .map(|(direction_index, angle)| {
            let p = project(mapped, angle.to_radians(), size, 1.0);
            let mut radius = 0.5_f32;
            for offset in offsets {
                for sign in [-1.0, 1.0] {
                    let q = project(
                        field.map(center + offset * sign),
                        angle.to_radians(),
                        size,
                        1.0,
                    );
                    radius = radius.max((q.x - p.x).hypot(q.y - p.y));
                }
            }
            proto::ColliderView {
                direction_index: direction_index as u32,
                x: p.x,
                y: p.y,
                radius,
            }
        })
        .collect()
}

fn collider_views_at_depth(node: &Node, size: u32, depth: f32) -> Vec<proto::ColliderView> {
    let position = V3::new(node.x, -node.y, depth);
    let radius = (node.rx.min(node.ry) * PIXELS_PER_UNIT).max(0.5);
    ANGLES_DEG
        .iter()
        .enumerate()
        .map(|(direction_index, angle)| {
            let projected = project(position, angle.to_radians(), size, 1.0);
            proto::ColliderView {
                direction_index: direction_index as u32,
                x: projected.x,
                y: projected.y,
                radius,
            }
        })
        .collect()
}

#[inline(always)]
fn draw_triangle<const LEGACY_SAMPLE_STEP: bool>(
    image: &mut RgbaImage,
    depths: &mut [f32],
    triangle: &Triangle,
    camera: Camera,
) {
    let size = image.width();
    let a = project_with_camera(triangle.a, camera);
    let b = project_with_camera(triangle.b, camera);
    let c = project_with_camera(triangle.c, camera);
    let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area.abs() < 0.001 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min((size - 1) as f32) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y =
        a.y.max(b.y)
            .max(c.y)
            .ceil()
            .min((image.height() - 1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let inverse_area = 1.0 / area;
    let w0_step = (b.y - c.y) * inverse_area;
    let w1_step = (c.y - a.y) * inverse_area;
    let normal = if LEGACY_SAMPLE_STEP {
        (triangle.b - triangle.a)
            .cross(triangle.c - triangle.a)
            .normalized()
    } else {
        // Imported meshes span meters, centimeters and arbitrary source units.
        // Normalize in f64 without the legacy small-vector clamp or overflow.
        let u = triangle.b - triangle.a;
        let v = triangle.c - triangle.a;
        let n = [
            f64::from(u.y) * f64::from(v.z) - f64::from(u.z) * f64::from(v.y),
            f64::from(u.z) * f64::from(v.x) - f64::from(u.x) * f64::from(v.z),
            f64::from(u.x) * f64::from(v.y) - f64::from(u.y) * f64::from(v.x),
        ];
        let length = n.iter().map(|x| x * x).sum::<f64>().sqrt();
        if length == 0.0 {
            return;
        }
        V3::new(
            (n[0] / length) as f32,
            (n[1] / length) as f32,
            (n[2] / length) as f32,
        )
    };
    const LIGHT: V3 = V3 {
        x: -0.420021,
        y: 0.790039,
        z: 0.450023,
    };
    let shade = (0.49 + 0.51 * normal.dot(LIGHT).abs()).clamp(0.0, 1.0);
    for y in min_y..=max_y {
        let py = y as f32 + 0.5;
        let px = min_x as f32 + 0.5;
        let mut w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) * inverse_area;
        let mut w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) * inverse_area;
        for x in min_x..=max_x {
            let w2 = 1.0 - w0 - w1;
            let sample_w0 = w0;
            let sample_w1 = w1;
            w0 += w0_step;
            w1 += w1_step;
            // Existing monster pixels retain their historical sample stepping.
            // Generic meshes use all three weights at the same pixel center;
            // mixing adjacent samples causes false depth seams on coplanar faces.
            let w0 = if LEGACY_SAMPLE_STEP { w0 } else { sample_w0 };
            let w1 = if LEGACY_SAMPLE_STEP { w1 } else { sample_w1 };
            if w0 < -0.0001 || w1 < -0.0001 || w2 < -0.0001 {
                continue;
            }
            let depth = w0 * a.depth + w1 * b.depth + w2 * c.depth;
            let offset = (y * size + x) as usize;
            if depth <= depths[offset] {
                continue;
            }
            depths[offset] = depth;
            let local = triangle.local[0] * w0 + triangle.local[1] * w1 + triangle.local[2] * w2;
            let variation = texture_value(triangle.texture, local, triangle.seed);
            let mut shaded = triangle.color;
            for channel in shaded.iter_mut().take(3) {
                *channel = (*channel as f32 * (shade + variation).clamp(0.25, 1.35)) as u8;
            }
            image.put_pixel(x, y, Rgba(shaded));
        }
    }
}

#[inline(always)]
pub(crate) fn rasterize(
    triangles: &[Triangle],
    image: &mut RgbaImage,
    depths: &mut [f32],
    camera: Camera,
) {
    for triangle in triangles {
        draw_triangle::<true>(image, depths, triangle, camera);
    }
}

/// Generic mesh path with coherent pixel-center coverage/depth interpolation.
pub(crate) fn rasterize_mesh(
    triangles: &[Triangle],
    image: &mut RgbaImage,
    depths: &mut [f32],
    camera: Camera,
) {
    for triangle in triangles {
        draw_triangle::<false>(image, depths, triangle, camera);
    }
}

type Rasterizer = unsafe fn(&[Triangle], &mut RgbaImage, &mut [f32], Camera);

unsafe fn rasterize_baseline(
    triangles: &[Triangle],
    image: &mut RgbaImage,
    depths: &mut [f32],
    camera: Camera,
) {
    rasterize(triangles, image, depths, camera);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "sse4.2,popcnt")]
unsafe fn rasterize_x64_v2(
    triangles: &[Triangle],
    image: &mut RgbaImage,
    depths: &mut [f32],
    camera: Camera,
) {
    rasterize(triangles, image, depths, camera);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2,fma,bmi1,bmi2,f16c,lzcnt")]
unsafe fn rasterize_x64_v3(
    triangles: &[Triangle],
    image: &mut RgbaImage,
    depths: &mut [f32],
    camera: Camera,
) {
    rasterize(triangles, image, depths, camera);
}

fn select_rasterizer() -> (Rasterizer, &'static str) {
    #[cfg(target_arch = "x86_64")]
    {
        // An override can only lower the selected tier. It is useful for
        // comparing kernels and cannot execute unsupported instructions.
        let ceiling = std::env::var("INFERNAL_CPU_KERNEL").unwrap_or_default();
        use std::arch::x86_64::__cpuid;
        let basic = __cpuid(1);
        let extended = __cpuid(0x8000_0001);
        let v2 = basic.ecx & (1 << 13) != 0
            && extended.ecx & 1 != 0
            && std::is_x86_feature_detected!("sse3")
            && std::is_x86_feature_detected!("ssse3")
            && std::is_x86_feature_detected!("sse4.1")
            && std::is_x86_feature_detected!("sse4.2")
            && std::is_x86_feature_detected!("popcnt");
        let v3 = v2
            && basic.ecx & (1 << 22) != 0
            && extended.ecx & (1 << 5) != 0
            && std::is_x86_feature_detected!("avx")
            && std::is_x86_feature_detected!("avx2")
            && std::is_x86_feature_detected!("bmi1")
            && std::is_x86_feature_detected!("bmi2")
            && std::is_x86_feature_detected!("f16c")
            && std::is_x86_feature_detected!("fma");
        if v3 && ceiling != "x64" && ceiling != "x64_v2" {
            return (rasterize_x64_v3, "x64_v3");
        }
        if v2 && ceiling != "x64" {
            return (rasterize_x64_v2, "x64_v2");
        }
        (rasterize_baseline, "x64")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        (
            rasterize_baseline,
            if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x86"
            },
        )
    }
}

fn rasterizer() -> (Rasterizer, &'static str) {
    static SELECTED: std::sync::OnceLock<(Rasterizer, &'static str)> = std::sync::OnceLock::new();
    *SELECTED.get_or_init(select_rasterizer)
}

/// Selected CPU kernel. The x64 library contains all three implementations.
pub fn cpu_kernel_name() -> &'static str {
    rasterizer().1
}

/// Shared 2x coverage reduction; keeps the established monster alpha contract.
pub(crate) fn downsample(high: &RgbaImage) -> RgbaImage {
    let width = high.width() / 2;
    let height = high.height() / 2;
    let mut low = RgbaImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let samples = [
                high.get_pixel(2 * x, 2 * y),
                high.get_pixel(2 * x + 1, 2 * y),
                high.get_pixel(2 * x, 2 * y + 1),
                high.get_pixel(2 * x + 1, 2 * y + 1),
            ];
            let count = samples.iter().filter(|p| p[3] > 0).count();
            if count < 2 {
                continue;
            }
            let mut rgba = [0u8; 4];
            for channel in 0..4 {
                rgba[channel] = (samples
                    .iter()
                    .filter(|p| p[3] > 0)
                    .map(|p| p[channel] as u32)
                    .sum::<u32>()
                    / count as u32) as u8;
            }
            low.put_pixel(x, y, Rgba(rgba));
        }
    }
    low
}

fn frame_with_stats_mode(
    triangles: &[Triangle],
    pose: &Pose,
    size: u32,
    angle: f32,
    palette: &[[u8; 4]],
    nearest: &mut std::collections::HashMap<[u8; 3], [u8; 3]>,
    coherent: bool,
) -> (RgbaImage, [std::time::Duration; 3]) {
    let phase_started = Instant::now();
    let high_size = size * 2;
    let mut high = RgbaImage::new(high_size, high_size);
    let mut depths = vec![f32::NEG_INFINITY; (high_size * high_size) as usize];
    let camera = Camera::new(angle, high_size, 2.0);
    if coherent {
        rasterize_mesh(triangles, &mut high, &mut depths, camera);
    } else {
        let kernel = rasterizer().0;
        unsafe {
            kernel(triangles, &mut high, &mut depths, camera);
        }
    }
    let raster_time = phase_started.elapsed();
    let phase_started = Instant::now();
    let mut low = downsample(&high);
    let downsample_time = phase_started.elapsed();
    let phase_started = Instant::now();
    render::quantize_cached(&mut low, palette, nearest);
    if pose.state == "DEATH" {
        for pixel in low.pixels_mut() {
            pixel[3] =
                render::quantized_alpha((pixel[3] as f32 * render::death_opacity(pose)) as u8);
        }
    }
    (low, [raster_time, downsample_time, phase_started.elapsed()])
}

#[cfg(test)]
fn frame(
    triangles: &[Triangle],
    pose: &Pose,
    size: u32,
    angle: f32,
    palette: &[[u8; 4]],
    nearest: &mut std::collections::HashMap<[u8; 3], [u8; 3]>,
) -> RgbaImage {
    frame_with_stats_mode(triangles, pose, size, angle, palette, nearest, false).0
}

fn palette(body: &Body) -> Vec<[u8; 4]> {
    let mut colors = Vec::new();
    for material in body.nodes.iter().map(|n| n.material.as_str()) {
        let (dark, mid, high) = render::colors(material);
        for color in [dark, mid, high] {
            render::add_palette_color(&mut colors, color);
        }
    }
    for material in [
        "BONE",
        "HORN",
        "CHITIN",
        "METAL",
        "WOOD",
        "MEMBRANE",
        "ENERGY",
        "MAKEUP",
        "RED_CLOTH",
        "FUR_DARK",
        "FUNGUS",
    ] {
        let (dark, mid, high) = render::colors(material);
        for color in [dark, mid, high] {
            render::add_palette_color(&mut colors, color);
        }
    }
    render::add_palette_color(&mut colors, [24, 24, 31, 255]);
    render::add_palette_color(&mut colors, [28, 25, 27, 255]);
    colors
}

#[cfg(not(target_arch = "wasm32"))]
fn render_pool() -> Option<&'static rayon::ThreadPool> {
    static POOL: std::sync::OnceLock<Option<rayon::ThreadPool>> = std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        let logical_cores = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(2);
        // Leave at least two logical cores for Godot and the OS. The generator
        // itself already runs on a background job thread.
        if logical_cores < 4 {
            return None;
        }
        rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .thread_name(|index| format!("infernal-render-{index}"))
            .build()
            .ok()
    })
    .as_ref()
}

pub fn sheet(
    spec: &MonsterSpec,
    body: &Body,
    poses: &[Pose],
    size: u32,
    seed: u64,
) -> (RgbaImage, u32, u32) {
    let profile = std::env::var_os("INFERNAL_PROFILE").is_some();
    let started = Instant::now();
    let mut scene_time = std::time::Duration::ZERO;
    let mut frame_time = std::time::Duration::ZERO;
    let mut stage_times = [std::time::Duration::ZERO; 3];
    let total = poses.len() as u32 * ANGLES_DEG.len() as u32;
    // A near-square atlas avoids >8K/16K-tall textures after expanding animation
    // framing. Consumers already receive columns; frame IDs remain unchanged.
    let columns = ((total as f32).sqrt().ceil() as u32).max(1);
    let rows = total.div_ceil(columns);
    let palette = scene_palette(spec, body, seed);
    let mut output = RgbaImage::new(columns * size, rows * size);
    for (index, pose) in poses.iter().enumerate() {
        let phase_started = Instant::now();
        let triangles = scene(spec, body, pose, seed);
        scene_time += phase_started.elapsed();
        let make_frame = |(direction, degrees): (usize, &f32)| {
            let phase_started = Instant::now();
            let mut nearest = std::collections::HashMap::new();
            let (mut frame, stages) = frame_with_stats_mode(
                &triangles,
                pose,
                size,
                degrees.to_radians(),
                &palette,
                &mut nearest,
                visual_v2(spec),
            );
            v2_finish_alpha(spec, pose, &mut frame);
            (direction, frame, stages, phase_started.elapsed())
        };
        #[cfg(target_arch = "wasm32")]
        let frames: Vec<_> = ANGLES_DEG.par_iter().enumerate().map(make_frame).collect();
        #[cfg(not(target_arch = "wasm32"))]
        let frames: Vec<_> = if let Some(pool) = render_pool() {
            pool.install(|| ANGLES_DEG.par_iter().enumerate().map(make_frame).collect())
        } else {
            ANGLES_DEG.iter().enumerate().map(make_frame).collect()
        };
        for (direction, frame, stages, duration) in frames {
            for (total, stage) in stage_times.iter_mut().zip(stages) {
                *total += stage;
            }
            frame_time += duration;
            let frame_id = direction as u32 * poses.len() as u32 + index as u32;
            let ox = frame_id % columns * size;
            let oy = frame_id / columns * size;
            let source = frame.as_raw();
            let target = output.as_mut();
            let row_len = (size * 4) as usize;
            let stride = (columns * size * 4) as usize;
            for y in 0..size as usize {
                let destination = ((oy as usize + y) * stride) + ox as usize * 4;
                target[destination..destination + row_len]
                    .copy_from_slice(&source[y * row_len..(y + 1) * row_len]);
            }
        }
    }
    let color_count = render::color_count(&output) as u32;
    if profile {
        eprintln!("infernal profile: sprite_size={size} poses={} scene={:.1}ms raster={:.1}ms downsample={:.1}ms quantize={:.1}ms frame={:.1}ms sheet_total={:.1}ms", poses.len(), scene_time.as_secs_f64() * 1000.0, stage_times[0].as_secs_f64() * 1000.0, stage_times[1].as_secs_f64() * 1000.0, stage_times[2].as_secs_f64() * 1000.0, frame_time.as_secs_f64() * 1000.0, started.elapsed().as_secs_f64() * 1000.0);
    }
    (output, columns, color_count)
}

pub fn visible_extent(sheet: &RgbaImage, size: u32, columns: u32, stride: u32) -> (u32, u32) {
    let mut width = 0;
    let mut height = 0;
    for direction in 0..ANGLES_DEG.len() as u32 {
        let frame = direction * stride;
        let sx = frame % columns * size;
        let sy = frame / columns * size;
        let mut bounds = (size, size, 0, 0);
        for y in 0..size {
            for x in 0..size {
                if sheet.get_pixel(sx + x, sy + y)[3] > 0 {
                    bounds.0 = bounds.0.min(x);
                    bounds.1 = bounds.1.min(y);
                    bounds.2 = bounds.2.max(x + 1);
                    bounds.3 = bounds.3.max(y + 1);
                }
            }
        }
        width = width.max(bounds.2.saturating_sub(bounds.0));
        height = height.max(bounds.3.saturating_sub(bounds.1));
    }
    (width, height)
}

/// Enlarged first pose from each camera direction for inspecting a bake.
pub fn preview(sheet: &RgbaImage, size: u32, columns: u32, stride: u32) -> RgbaImage {
    let zoom = 4;
    let mut output = RgbaImage::new(size * zoom * ANGLES_DEG.len() as u32, size * zoom);
    for direction in 0..ANGLES_DEG.len() as u32 {
        let frame = direction * stride;
        let sx = frame % columns * size;
        let sy = frame / columns * size;
        for y in 0..size {
            for x in 0..size {
                let pixel = *sheet.get_pixel(sx + x, sy + y);
                for dy in 0..zoom {
                    for dx in 0..zoom {
                        output.put_pixel((direction * size + x) * zoom + dx, y * zoom + dy, pixel);
                    }
                }
            }
        }
    }
    output
}

/// Export the exact triangulated intermediate scene used by the sprite baker.
/// Coordinates are X forward, Y up, Z lateral; colors are per-face RGBA.
/// Intended for deterministic offline inspection in Blender, not runtime loading.
pub fn debug_scene_json(
    spec: &MonsterSpec,
    body: &Body,
    pose: &Pose,
    seed: u64,
) -> serde_json::Value {
    let triangles = scene(spec, body, pose, seed);
    serde_json::json!({
        "coordinates": "X forward, Y up, Z lateral",
        "affinity": spec.affinity, "body_plan": spec.body_plan,
        "seed": seed, "clip": pose.clip_id, "phase": pose.phase,
        "nodes": body.nodes.iter().enumerate().map(|(i,n)| {
            let (p,a) = node_pose(body,i,pose);
            serde_json::json!({"id":n.id,"kind":n.kind,"parent":n.parent,
                "position":[p.x,p.y,p.z],"radii":[n.rx,n.ry],"angle":a})
        }).collect::<Vec<_>>(),
        "triangles": triangles.iter().map(|t| serde_json::json!({
            "vertices":[[t.a.x,t.a.y,t.a.z],[t.b.x,t.b.y,t.b.z],[t.c.x,t.c.y,t.c.z]],
            "color": t.color
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
    fn visual_signature(triangles: &[Triangle]) -> Vec<u32> {
        triangles
            .iter()
            .flat_map(|t| {
                [
                    t.a.x.to_bits(),
                    t.a.y.to_bits(),
                    t.a.z.to_bits(),
                    t.b.x.to_bits(),
                    t.b.y.to_bits(),
                    t.b.z.to_bits(),
                    t.c.x.to_bits(),
                    t.c.y.to_bits(),
                    t.c.z.to_bits(),
                    u32::from_le_bytes(t.color),
                    t.seed,
                ]
            })
            .collect()
    }

    #[test]
    fn aquatic_tail_yaw_preserves_every_attachment_length() {
        let spec = crate::parser::parse_vocabulary("shark");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        let mut pose = poses[0].clone();
        pose.motion_archetype = "AQUATIC".into();
        pose.state = "MOVE".into();
        pose.phase = 0.25;
        let mut displaced = false;
        for (index, node) in body.nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                let before =
                    base_node_pose(&body, index, &pose).0 - base_node_pose(&body, parent, &pose).0;
                let after = node_pose(&body, index, &pose).0 - node_pose(&body, parent, &pose).0;
                assert!(
                    (before.length() - after.length()).abs() < 0.0001,
                    "{}",
                    node.id
                );
                if node.kind == "TAIL" && (before.z - after.z).abs() > 0.01 {
                    displaced = true;
                }
            }
        }
        assert!(displaced, "fish tail must propel laterally");
    }

    #[test]
    fn posed_attack_origin_views_follow_exact_root_and_joint_motion() {
        let spec = crate::parser::parse_vocabulary("orc with sword");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        let mut pose = poses[0].clone();
        pose.root_x += 12.0;
        pose.root_y -= 7.0;
        pose.root_angle = 0.3;
        for index in 0..body.nodes.len() {
            for view in collider_views_for_pose(&body, index, &pose, 128) {
                let exact = project(
                    node_pose(&body, index, &pose).0,
                    ANGLES_DEG[view.direction_index as usize].to_radians(),
                    128,
                    1.0,
                );
                assert!((exact.x - view.x).abs() < 0.0001 && (exact.y - view.y).abs() < 0.0001);
            }
        }
    }

    #[test]
    fn v2_ghost_lion_keeps_orc_rider_and_huge_axe_opaque() {
        let mut spec = crate::v2::prompt_normalization::normalize_mount("Ghost lion with orc rider with huge axe").unwrap();
        crate::v2::prepare(&mut spec);
        let mut body = crate::anatomy::build(&spec,&mut ChaCha8Rng::seed_from_u64(0));
        crate::v2::anatomy::refine(&spec,&mut body,0);
        assert!(body.nodes.iter().any(|n|n.id.starts_with("rider_")&&n.kind=="WEAPON"));
        for node in &body.nodes {
            let local = v2_surface_spec(&spec,&node.id);
            assert_eq!(local.features.iter().any(|f|f=="GHOST"), !node.id.starts_with("rider_"),"{}",node.id);
        }
        let (_,_,_,poses) = crate::animation::make(&spec,&body);
        let triangles = scene(&spec,&body,&poses[0],0);
        assert!(triangles.iter().any(|t|t.color[3]==128));
        assert!(triangles.iter().any(|t|t.color[3]==255));
        let (image,_,colors)=sheet(&spec,&body,&poses[..1],256,0);
        assert!(image.pixels().any(|p|p[3]==97));
        assert!(image.pixels().any(|p|p[3]==255));
        assert!(colors<=255);
    }

    #[test]
    fn v2_skeletal_merfolk_override_skin_hair_and_scales() {
        let mut spec = crate::parser::parse_vocabulary("mermaid");
        spec.features
            .extend(["PROFILE_V2".into(), "SKELETAL".into()]);
        crate::v2::prepare(&mut spec);
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(0));
        crate::v2::anatomy::refine(&spec, &mut body, 0);
        for node in &body.nodes {
            assert_eq!(v2_node_material(&spec, node, 0), "BONE");
            if node.kind == "HEAD" {
                assert!(std::ptr::eq(
                    v2_mesh_override(&spec, node).unwrap(),
                    v2_skeleton::mesh("skull")
                ));
            }
            if node.kind == "TORSO" {
                assert!(std::ptr::eq(
                    v2_mesh_override(&spec, node).unwrap(),
                    v2_skeleton::mesh("ribs")
                ));
            }
            if node.kind == "TAIL" {
                assert!(std::ptr::eq(
                    v2_mesh_override(&spec, node).unwrap(),
                    v2_skeleton::mesh("long_bone")
                ));
            }
        }
    }

    #[test]
    fn v2_shade_unwinds_the_whole_body_and_restores_exact_endpoint() {
        for prompt in [
            "shade humanoid",
            "shade wolf",
            "shade tree monster",
            "shade mermaid",
        ] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            spec.features.push("SHADE".into());
            crate::v2::prepare(&mut spec);
            let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(0));
            crate::v2::anatomy::refine(&spec, &mut body, 0);
            let (_, _, _, poses) = crate::animation::make(&spec, &body);
            let original = poses[0].clone();
            let mut start = original.clone();
            start.state = "SHADE_UNWIND".into();
            start.phase = 0.0;
            assert_eq!(
                visual_signature(&scene(&spec, &body, &original, 0)),
                visual_signature(&scene(&spec, &body, &start, 0))
            );
            let mut peak = start.clone();
            peak.phase = 1.0;
            let palette = scene_palette(&spec, &body, 0);
            let render = |p: &Pose| {
                frame_with_stats_mode(
                    &scene(&spec, &body, p, 0),
                    p,
                    256,
                    0.0,
                    &palette,
                    &mut std::collections::HashMap::new(),
                    true,
                )
                .0
            };
            let before = render(&start);
            let after = render(&peak);
            let mut strands = Vec::new();
            for (i, node) in body
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.id.starts_with("v2_shade_tendril_"))
            {
                let parent = node.parent.unwrap();
                let radius = if body.nodes[parent].id.starts_with("v2_shade_tendril_") {
                    body.nodes[parent].rx
                } else {
                    node.rx * 1.6
                };
                v2_shade_tube(
                    &mut strands,
                    node_pose(&body, parent, &peak).0,
                    node_pose(&body, i, &peak).0,
                    radius,
                    node.rx * if node.id.ends_with("_4") { 0.35 } else { 1.0 },
                );
            }
            let strand_image = frame_with_stats_mode(
                &strands,
                &peak,
                256,
                0.0,
                &palette,
                &mut std::collections::HashMap::new(),
                true,
            )
            .0;
            let extra = after
                .pixels()
                .zip(strand_image.pixels())
                .filter(|(a, b)| a[3] > 0 && b[3] == 0)
                .count();
            let strand_pixels = strand_image.pixels().filter(|p| p[3] > 0).count();
            assert!(
                extra < strand_pixels / 10 + 4,
                "peak retained a web outside strands: {prompt}, {extra}/{strand_pixels}"
            );

            let occupied = before.pixels().filter(|p| p[3] > 0).count();
            let vanished = before
                .pixels()
                .zip(after.pixels())
                .filter(|(a, b)| a[3] > 0 && b[3] == 0)
                .count();
            assert!(
                vanished as f32 / occupied as f32 > 0.65,
                "unchanged body survived peak: {prompt}, {vanished}/{occupied}"
            );
            assert!(after.pixels().all(|p| p[3] == 0 || p.0 == [0, 0, 0, 255]));
            assert_eq!(
                before,
                render(&start),
                "reverse endpoint must be byte exact"
            );
            let head = body.nodes.iter().position(|n| n.kind == "HEAD").unwrap();
            let old = collider_views_for_pose(&body, head, &peak, 256);
            let morphed = collider_views_for_shade_receiver(&body, head, &peak, 256);
            assert!(old
                .iter()
                .zip(&morphed)
                .any(|(a, b)| (a.x - b.x).hypot(a.y - b.y) > 2.0));
            spec.features.retain(|f| f != "PROFILE_V2");
            assert_eq!(
                visual_signature(&scene(&spec, &body, &start, 0)),
                visual_signature(&scene(&spec, &body, &peak, 0)),
                "V1 must not morph"
            );
        }
    }

    #[test]
    fn v2_coherent_raster_is_gated_and_v1_keeps_legacy_pixels() {
        let mut spec = crate::parser::parse_vocabulary("orc");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        for coherent in [false, true] {
            if coherent {
                spec.features.push("PROFILE_V2".into());
            }
            let triangles = scene(&spec, &body, &poses[0], 42);
            let palette = scene_palette(&spec, &body, 42);
            let expected = frame_with_stats_mode(
                &triangles,
                &poses[0],
                128,
                0.0,
                &palette,
                &mut std::collections::HashMap::new(),
                coherent,
            )
            .0;
            let (atlas, _, _) = sheet(&spec, &body, &poses[..1], 128, 42);
            assert_eq!(
                image::imageops::crop_imm(&atlas, 0, 0, 128, 128).to_image(),
                expected
            );
        }
    }

    #[test]
    fn v2_spectral_modifiers_preserve_real_alpha_and_opaque_ghouls() {
        let mut spec = crate::parser::parse_vocabulary("humanoid");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        spec.features.extend(["PROFILE_V2".into(), "GHOST".into()]);
        let ghost = scene(&spec, &body, &poses[0], 42);
        assert!(ghost.iter().all(|t| t.color[3] == 128));
        let image = frame(
            &ghost,
            &poses[0],
            128,
            0.0,
            &scene_palette(&spec, &body, 42),
            &mut std::collections::HashMap::new(),
        );
        assert!(image.pixels().any(|p| p[3] == 128));
        assert!(image.pixels().all(|p| p[3] == 0 || p[3] == 128));
        let (ghost_sheet, _, _) = sheet(&spec, &body, &poses[..1], 128, 42);
        assert!(ghost_sheet.pixels().any(|p| p[3] == 97));
        assert!(ghost_sheet.pixels().all(|p| p[3] == 0 || p[3] == 97));
        assert!(render::color_count(&ghost_sheet) <= 255);
        spec.features.retain(|f| f != "GHOST");
        spec.features.push("GHOUL".into());
        assert!(scene(&spec, &body, &poses[0], 42)
            .iter()
            .all(|t| t.color[3] == 255));
        let (ghoul_sheet, _, _) = sheet(&spec, &body, &poses[..1], 128, 42);
        assert!(ghoul_sheet
            .pixels()
            .filter(|p| p[3] > 0)
            .all(|p| p[3] == 255));
        let mut phase = poses[0].clone();
        phase.state = "PHASE".into();
        let (phase_sheet, _, _) = sheet(&spec, &body, &[phase], 128, 42);
        assert!(phase_sheet
            .pixels()
            .filter(|p| p[3] > 0)
            .all(|p| p[3] == 64));
        spec.features.push("SHADE".into());
        assert!(scene(&spec, &body, &poses[0], 42)
            .iter()
            .all(|t| t.color == [0, 0, 0, 255]));
        let (shade_sheet, _, _) = sheet(&spec, &body, &poses[..1], 128, 42);
        assert!(shade_sheet
            .pixels()
            .filter(|p| p[3] > 0)
            .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0));
        let mut shade_phase = poses[0].clone();
        shade_phase.state = "PHASE".into();
        let (shade_phase_sheet, _, _) = sheet(&spec, &body, &[shade_phase], 128, 42);
        assert!(shade_phase_sheet
            .pixels()
            .filter(|p| p[3] > 0)
            .all(|p| p.0 == [0, 0, 0, 255]));
        spec.features.retain(|f| f != "PROFILE_V2");
        assert!(scene(&spec, &body, &poses[0], 42)
            .iter()
            .any(|t| t.color[0] > 0));
    }

    #[test]
    fn v2_explicit_hybrid_head_and_upper_identity_are_local() {
        let mut spec = crate::parser::parse_vocabulary("mermaid");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let head = body.nodes.iter().find(|n| n.kind == "HEAD").unwrap();
        spec.features.extend(
            [
                "HYBRID_VERSION_1",
                "HYBRID_UPPER_STORMTROOPER",
                "HYBRID_LOWER_MERMAID",
                "HYBRID_HEAD_STORMTROOPER",
                "HYBRID_LEGS_0",
                "HYBRID_ARMS_2",
            ]
            .map(str::to_owned),
        );
        assert_ne!(skull_recipe(&spec, head), "skull_stormtrooper");
        spec.features.push("PROFILE_V2".into());
        assert_eq!(v2_face_recipe(&spec, head), Some("helmet"));
        spec.features.retain(|f| f != "HYBRID_HEAD_STORMTROOPER");
        spec.features.push("HYBRID_HEAD_FISH".into());
        assert_eq!(v2_face_recipe(&spec, head), None);
        assert_eq!(skull_recipe(&spec, head), "skull_reptile");
        let mut tail = head.clone();
        tail.kind = "TAIL".into();
        tail.id = "v2_fish_tail_0".into();
        assert_eq!(node_identity(&spec, &tail.id).1, "MERMAID");
        spec.features.retain(|f| f != "HYBRID_HEAD_FISH");
        spec.features.push("HYBRID_HEAD_BIRD".into());
        assert_eq!(skull_recipe(&spec, head), "skull_avian");
    }

    #[test]
    fn v2_humanoid_coverage_manifest_matches_every_enumerated_recipe() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../assets/v2/humanoid_face_coverage.json")).unwrap();
        let base = crate::parser::parse_vocabulary("humanoid");
        let body = crate::anatomy::build(&base, &mut ChaCha8Rng::seed_from_u64(42));
        let head = body.nodes.iter().find(|n| n.kind == "HEAD").unwrap();
        let entries = manifest["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 66);
        for entry in entries {
            let mut spec = crate::parser::parse_vocabulary(entry["prompt"].as_str().unwrap());
            assert_eq!(spec.affinity, entry["affinity"].as_str().unwrap());
            assert_eq!(v2_face_recipe(&spec, head), None);
            spec.features.push("PROFILE_V2".into());
            assert_eq!(
                v2_face_recipe(&spec, head),
                entry["v2_face"].as_str(),
                "{}",
                spec.affinity
            );
        }
    }

    #[test]
    fn v2_humanoid_faces_are_species_specific_and_never_replace_beaks() {
        for (prompt, expected) in [
            ("orc", "orc"),
            ("stormtrooper", "helmet"),
            ("mermaid", "mermaid"),
            ("humanoid", "human"),
            ("harpy", "human"),
            ("siren", "human"),
            ("centaur", "human"),
            ("satyr", "human"),
            ("merman", "human"),
            ("robot", "robot"),
            ("alien", "alien"),
            ("gorilla", "ape"),
        ] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
            let head = body.nodes.iter().find(|node| node.kind == "HEAD").unwrap();
            assert_eq!(v2_face_recipe(&spec, head), None);
            spec.features.push("PROFILE_V2".into());
            assert_eq!(v2_face_recipe(&spec, head), Some(expected), "{prompt}");
            let mesh = v2_faces::mesh(expected);
            assert!(mesh
                .vertices
                .iter()
                .flatten()
                .all(|value| value.is_finite()));
            assert!(mesh
                .faces
                .iter()
                .all(|(a, b, c, _)| *a < mesh.vertices.len()
                    && *b < mesh.vertices.len()
                    && *c < mesh.vertices.len()));
            assert!(
                mesh.vertices.iter().all(|v| v[0] < 0.9),
                "{prompt}: no projecting muzzle"
            );
            assert!(mesh.faces.iter().any(|(_, _, _, role)| role
                == if expected == "helmet" || expected == "robot" {
                    "helmet_black"
                } else {
                    "eye_dark"
                }));
        }
        for prompt in [
            "falcon",
            "wolf",
            "minotaur",
            "dragon",
            "jellyfish",
            "tornado",
        ] {
            let mut spec = crate::parser::parse_vocabulary(prompt);
            let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
            spec.features.push("PROFILE_V2".into());
            for head in body.nodes.iter().filter(|node| node.kind == "HEAD") {
                assert_eq!(
                    v2_face_recipe(&spec, head),
                    None,
                    "preserve {prompt} beak/muzzle"
                );
            }
        }
        let mut hybrid = crate::parser::parse_vocabulary("mermaid");
        let body = crate::anatomy::build(&hybrid, &mut ChaCha8Rng::seed_from_u64(42));
        let mut secondary = body
            .nodes
            .iter()
            .find(|n| n.kind == "HEAD")
            .unwrap()
            .clone();
        secondary.id = "hybrid_head_0".into();
        hybrid.features.push("PROFILE_V2".into());
        hybrid.secondary_affinities = vec!["FALCON".into()];
        assert_eq!(v2_face_recipe(&hybrid, &secondary), None);
        assert!(
            v2_faces::mesh("mermaid")
                .vertices
                .iter()
                .any(|v| v[1] < -1.5),
            "flowing hair extends beyond cap"
        );
        assert_ne!(
            v2_faces::mesh("orc").vertices,
            v2_faces::mesh("human").vertices
        );
        assert_ne!(
            v2_faces::mesh("helmet").vertices,
            v2_faces::mesh("human").vertices
        );
    }

    #[test]
    fn v2_visuals_require_explicit_profile_gate() {
        let original = crate::parser::parse_vocabulary("wolf");
        let body = crate::anatomy::build(&original, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&original, &body);
        let mut flagged = original.clone();
        flagged.features.push("FRIENDLY_LURE".into());
        assert!(!visual_v2(&flagged));
        assert_eq!(
            visual_signature(&scene(&original, &body, &poses[0], 42)),
            visual_signature(&scene(&flagged, &body, &poses[0], 42))
        );
        assert_eq!(palette(&body), scene_palette(&flagged, &body, 42));
        assert!(body
            .nodes
            .iter()
            .all(|node| v2_mesh_override(&flagged, node).is_none()));
    }

    #[test]
    fn v2_seed_palette_and_geometry_are_repeatable_and_varied() {
        let mut spec = crate::parser::parse_vocabulary("wolf");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        spec.features.push("PROFILE_V2".into());
        let first = visual_signature(&scene(&spec, &body, &poses[0], 42));
        assert_eq!(first, visual_signature(&scene(&spec, &body, &poses[0], 42)));
        assert_ne!(first, visual_signature(&scene(&spec, &body, &poses[0], 43)));
        assert_ne!(
            scene_palette(&spec, &body, 42),
            scene_palette(&spec, &body, 43)
        );
        assert!(scene_palette(&spec, &body, 42).len() <= 255);
        for kind in [
            "muscle",
            "trunk",
            "tail",
            "frog",
            "fan",
            "web",
            "crown",
            "tree_face",
        ] {
            let mesh = v2_mesh(kind);
            assert!(mesh.vertices.iter().flatten().all(|v| v.is_finite()));
            assert!(mesh
                .faces
                .iter()
                .all(|(a, b, c, _)| *a < mesh.vertices.len()
                    && *b < mesh.vertices.len()
                    && *c < mesh.vertices.len()));
        }
    }

    #[test]
    fn v2_friendly_lure_has_semantic_reveal_boundary() {
        let mut spec = crate::parser::parse_vocabulary("wolf");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        spec.features
            .extend(["PROFILE_V2".into(), "FRIENDLY_LURE".into()]);
        let mut pose = poses[0].clone();
        pose.state = "LURE".into();
        assert!(v2_friendly_pose(&spec, &pose));
        pose.phase = 0.0;
        let before_hearts = scene(&spec, &body, &pose, 42).len();
        pose.phase = 0.5;
        assert!(scene(&spec, &body, &pose, 42).len() > before_hearts);
        let friendly = visual_signature(&scene(&spec, &body, &pose, 42));
        pose.state = "ATTACK".into();
        assert!(!v2_friendly_pose(&spec, &pose));
        assert_ne!(friendly, visual_signature(&scene(&spec, &body, &pose, 42)));
        pose.state = "REVEAL".into();
        pose.phase = 0.0;
        assert!(v2_friendly_pose(&spec, &pose));
        pose.phase = 1.0;
        assert!(!v2_friendly_pose(&spec, &pose));
    }

    #[test]
    fn checked_in_meshes_have_valid_faces() {
        for (name, _) in MESHES {
            let mesh = asset(name);
            assert!(!mesh.vertices.is_empty());
            assert!(!mesh.faces.is_empty());
            for (a, b, c, _) in &mesh.faces {
                assert!(*a < mesh.vertices.len());
                assert!(*b < mesh.vertices.len());
                assert!(*c < mesh.vertices.len());
            }
        }
    }

    #[test]
    fn held_weapon_follows_hand_during_walk() {
        let spec = crate::parser::parse_vocabulary("cyclops with a sword");
        let mut rng = ChaCha8Rng::seed_from_u64(9);
        let body = crate::anatomy::build(&spec, &mut rng);
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        let weapon_index = body
            .nodes
            .iter()
            .position(|node| node.id == "weapon")
            .unwrap();
        let hand_index = body.nodes[weapon_index].parent.unwrap();
        let movement: Vec<_> = poses.iter().filter(|pose| pose.clip_id == "move").collect();
        let weapon = &body.nodes[weapon_index];
        let hand = &body.nodes[hand_index];
        for pose in movement {
            let (w, _) = node_pose(&body, weapon_index, pose);
            let (h, a) = node_pose(&body, hand_index, pose);
            let delta = a - hand.angle;
            let dx = weapon.x - hand.x;
            let dy = weapon.y - hand.y;
            assert!((w.x - h.x - (dx * delta.cos() - dy * delta.sin())).abs() < 0.001);
            assert!((w.y - h.y + (dx * delta.sin() + dy * delta.cos())).abs() < 0.001);
            assert!((w.z - h.z - 0.4).abs() < 0.001);
        }
    }

    #[test]
    fn claws_share_their_parent_lateral_attachment() {
        let spec = crate::parser::parse_vocabulary("bear");
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        for (i, n) in body
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.kind == "CLAW")
        {
            let parent = n.parent.unwrap();
            for pose in &poses {
                assert!(
                    (node_pose(&body, i, pose).0.z - node_pose(&body, parent, pose).0.z).abs()
                        < 0.001
                );
            }
        }
    }

    #[test]
    fn neutral_collider_views_match_mesh_attachment_centers() {
        for prompt in ["bear", "cyclops with sword", "beetle with wings"] {
            let spec = crate::parser::parse_vocabulary(prompt);
            let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
            let (_, _, _, poses) = crate::animation::make(&spec, &body);
            let mut neutral = poses[0].clone();
            neutral.physical_positions = body.nodes.iter().map(|n| (n.x, n.y, n.angle)).collect();
            for i in 0..body.nodes.len() {
                for view in collider_views_for_body(&body, i, 128) {
                    let p = project(
                        node_pose(&body, i, &neutral).0,
                        ANGLES_DEG[view.direction_index as usize].to_radians(),
                        128,
                        1.0,
                    );
                    assert!(
                        (p.x - view.x).abs() < 0.001 && (p.y - view.y).abs() < 0.001,
                        "{prompt} {}",
                        body.nodes[i].id
                    );
                }
            }
        }
    }

    #[test]
    fn fitted_canvas_contains_every_pose_and_direction() {
        for prompt in [
            "dragon",
            "cobra",
            "orc with a huge sword",
            "giant spider",
            "ghost",
            "hornet",
        ] {
            let spec = crate::parser::parse_vocabulary(prompt);
            let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
            let (_, _, _, poses) = crate::animation::make(&spec, &body);
            let size = required_frame_size(&spec, &body, &poses, 72, 42);
            assert!((72..=512).contains(&size) && size.is_multiple_of(8));
            for pose in &poses {
                for triangle in scene(&spec, &body, pose, 42) {
                    for point in [triangle.a, triangle.b, triangle.c] {
                        for angle in ANGLES_DEG {
                            let p = project(point, angle.to_radians(), size, 1.0);
                            assert!(
                                p.x >= 2.0
                                    && p.x <= size as f32 - 2.0
                                    && p.y >= 2.0
                                    && p.y <= size as f32 - 2.0,
                                "{prompt} {}: {size}, {},{}",
                                pose.clip_id,
                                p.x,
                                p.y
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn projection_keeps_one_pixel_scale_across_canvas_sizes() {
        let point = V3::new(12.0, 5.0, 3.0);
        let small = project(point, 0.0, 72, 1.0);
        let large = project(point, 0.0, 128, 1.0);
        assert!(((small.x - 36.0) - (large.x - 64.0)).abs() < 0.001);
        assert!(((small.y - 72.0 * 0.57) - (large.y - 128.0 * 0.57)).abs() < 0.001);
    }

    #[test]
    fn mounted_rider_fits_frame_without_torso_bridge() {
        let spec = crate::parser::parse_vocabulary(
            "orc with huge axe riding giant spider that shoots fire",
        );
        let body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(7));
        let (_, _, _, poses) = crate::animation::make(&spec, &body);
        let idle = poses.iter().find(|pose| pose.clip_id == "idle").unwrap();
        let colors = palette(&body);
        let sprite = frame(
            &scene(&spec, &body, idle, 7),
            idle,
            128,
            0.0,
            &colors,
            &mut std::collections::HashMap::new(),
        );
        assert!(sprite.rows().next().unwrap().all(|pixel| pixel[3] == 0));
        let rider = body
            .nodes
            .iter()
            .position(|node| node.id == "rider_torso")
            .unwrap();
        let mut disconnected = body.clone();
        disconnected.nodes[rider].parent = None;
        assert_eq!(
            scene(&spec, &body, idle, 7).len(),
            scene(&spec, &disconnected, idle, 7).len()
        );
    }

    #[test]
    fn seeded_material_noise_is_stable_and_changes_with_seed() {
        let position = V3::new(0.47, -0.18, 0.36);
        let first = texture_value(TextureKind::Fur, position, 7);
        assert_eq!(first, texture_value(TextureKind::Fur, position, 7));
        assert_ne!(first, texture_value(TextureKind::Fur, position, 8));
        assert_eq!(texture_value(TextureKind::None, position, 7), 0.0);
    }

    #[test]
    fn locomotion_changes_the_rendered_silhouette() {
        for prompt in ["wolf", "orc with a sword", "cobra", "slug", "spider"] {
            let spec = crate::parser::parse_vocabulary(prompt);
            let mut rng = ChaCha8Rng::seed_from_u64(31);
            let body = crate::anatomy::build(&spec, &mut rng);
            let (_, _, _, poses) = crate::animation::make(&spec, &body);
            let moving: Vec<_> = poses.iter().filter(|pose| pose.clip_id == "move").collect();
            assert_eq!(moving.len(), 8, "{prompt}");
            let colors = palette(&body);
            let first = frame(
                &scene(&spec, &body, moving[0], 31),
                moving[0],
                96,
                0.0,
                &colors,
                &mut std::collections::HashMap::new(),
            );
            let later = frame(
                &scene(&spec, &body, moving[2], 31),
                moving[2],
                96,
                0.0,
                &colors,
                &mut std::collections::HashMap::new(),
            );
            let changed = first
                .pixels()
                .zip(later.pixels())
                .filter(|(a, b)| (a[3] > 0) != (b[3] > 0))
                .count();
            assert!(changed > 100, "{prompt} has only {changed} moving pixels");
        }
    }
}
