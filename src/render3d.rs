//! Deterministic, headless 3D mesh baking for directional sprite sheets.
//! Meshes are authored assets; positions and dimensions still come from anatomy.

use crate::{
    anatomy::{Body, Node},
    animation::Pose,
    parser::MonsterSpec,
    proto, render,
};
use image::{Rgba, RgbaImage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub const ANGLES_DEG: [f32; 4] = [0.0, 90.0, 180.0, 270.0];
pub const PIXELS_PER_UNIT: f32 = 96.0 / 62.0;

#[derive(Clone, Copy, Debug, Default)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}
impl V3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
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

fn node_identity<'a>(spec: &'a MonsterSpec, id: &str) -> (&'a str, &'a str) {
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
            "ARACHNID" | "INSECT" => "skull_arthropod",
            "HUMANOID" | "FLOATING" => "skull_humanoid",
            "WINGED" => "skull_avian",
            "SERPENT" => "skull_reptile",
            _ => "skull_generic",
        },
    }
}

#[derive(Clone, Copy)]
struct Triangle {
    a: V3,
    b: V3,
    c: V3,
    color: [u8; 4],
    local: [V3; 3],
    texture: TextureKind,
    seed: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TextureKind {
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
}

fn texture_kind(material: &str, role: &str) -> TextureKind {
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
        TextureKind::None => 0.0,
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

fn node_pose(body: &Body, index: usize, pose: &Pose) -> (V3, f32) {
    let node = &body.nodes[index];
    if node.kind == "WEAPON"
        && !matches!(
            pose.state.as_str(),
            "IMPACT_LIGHT" | "IMPACT_HEAVY" | "KNOCKBACK" | "STUN" | "DEATH"
        )
    {
        if let Some(parent) = node.parent {
            let anchor = &body.nodes[parent];
            let (px, py, pa) = render::transform(anchor, parent, pose, body.gravity);
            return (
                V3::new(
                    px + node.x - anchor.x,
                    -py - node.y + anchor.y,
                    depth_for(anchor) + 0.4,
                ),
                pa + node.angle - anchor.angle,
            );
        }
    }
    let (x, y, angle) = render::transform(node, index, pose, body.gravity);
    let mut z = depth_for(node);
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

fn scene(spec: &MonsterSpec, body: &Body, pose: &Pose, seed: u64) -> Vec<Triangle> {
    let positions: Vec<(V3, f32)> = body
        .nodes
        .iter()
        .enumerate()
        .map(|(i, _)| node_pose(body, i, pose))
        .collect();
    let mut triangles = Vec::with_capacity(body.nodes.len() * 100);
    let mut texture_rng = ChaCha8Rng::seed_from_u64(seed);
    for (i, node) in body.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            if !(node.kind == "TORSO" && body.nodes[parent].kind == "TORSO")
                && !["EYE", "MOUTH", "HORN", "FANG", "CLAW", "WEAPON", "WING"]
                    .contains(&node.kind.as_str())
            {
                let radius = node.rx.min(node.ry).min(body.nodes[parent].rx) * 0.6;
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
    for (i, node) in body.nodes.iter().enumerate() {
        let node_seed = texture_rng.random::<u32>();
        if node.kind == "MOUTH" || node.kind == "EYE" {
            continue;
        }
        let (position, angle) = positions[i];
        let (node_plan, node_affinity) = node_identity(spec, &node.id);
        let radius_z = if node.kind == "TORSO" {
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
        };
        let (mesh, scale, material) = match node.kind.as_str() {
            "HEAD" => (
                asset(skull_recipe(spec, node)),
                V3::new(node.rx, node.ry, node.ry * 0.88),
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
        let mut scale = scale;
        if spec.is_blob() && matches!(node.kind.as_str(), "TORSO" | "LOBE") {
            let pulse = (pose.phase * std::f32::consts::TAU + i as f32 * 0.7).sin();
            scale.x *= 1.0 + pulse * 0.08;
            scale.y *= 1.0 - pulse * 0.1;
        }
        if spec.is_gastropod() && matches!(node.kind.as_str(), "TORSO" | "SOLE") {
            let stretch = (pose.phase * std::f32::consts::TAU).sin();
            let amount = if spec.affinity == "SLUG" { 0.18 } else { 0.10 };
            scale.x *= 1.0 + stretch * amount;
            scale.y *= 1.0 - stretch * amount * 0.7;
        }
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
                irregularity: if matches!(node.kind.as_str(), "TORSO" | "LOBE" | "TAIL" | "SHELL") {
                    0.16
                } else if matches!(node.kind.as_str(), "LIMB" | "ARM" | "LEG") {
                    0.06
                } else {
                    0.0
                },
            },
        );
    }
    triangles
}

#[derive(Clone, Copy)]
struct ScreenPoint {
    x: f32,
    y: f32,
    depth: f32,
}

fn project(point: V3, angle: f32, resolution: u32, supersample: f32) -> ScreenPoint {
    let (s, c) = angle.sin_cos();
    let front = point.x * s + point.z * c;
    let right = point.x * c - point.z * s;
    let elevation = 0.28_f32;
    let scale = PIXELS_PER_UNIT * supersample;
    ScreenPoint {
        x: resolution as f32 * 0.5 + right * scale,
        y: resolution as f32 * 0.57
            + (-point.y * elevation.cos() + front * elevation.sin()) * scale,
        depth: front * elevation.cos() + point.y * elevation.sin(),
    }
}

pub fn collider_views(node: &Node, size: u32) -> Vec<proto::ColliderView> {
    let position = V3::new(node.x, -node.y, depth_for(node));
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

fn draw_triangle(image: &mut RgbaImage, depths: &mut [f32], triangle: &Triangle, angle: f32) {
    let size = image.width();
    let a = project(triangle.a, angle, size, 2.0);
    let b = project(triangle.b, angle, size, 2.0);
    let c = project(triangle.c, angle, size, 2.0);
    let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if area.abs() < 0.001 {
        return;
    }
    let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
    let max_x = a.x.max(b.x).max(c.x).ceil().min((size - 1) as f32) as u32;
    let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
    let max_y = a.y.max(b.y).max(c.y).ceil().min((size - 1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let normal = (triangle.b - triangle.a)
        .cross(triangle.c - triangle.a)
        .normalized();
    let light = V3::new(-0.42, 0.79, 0.45).normalized();
    let shade = (0.49 + 0.51 * normal.dot(light).abs()).clamp(0.0, 1.0);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) / area;
            let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) / area;
            let w2 = 1.0 - w0 - w1;
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

fn frame(
    spec: &MonsterSpec,
    body: &Body,
    pose: &Pose,
    size: u32,
    angle: f32,
    palette: &[[u8; 4]],
    seed: u64,
) -> RgbaImage {
    let high_size = size * 2;
    let mut high = RgbaImage::new(high_size, high_size);
    let mut depths = vec![f32::NEG_INFINITY; (high_size * high_size) as usize];
    for triangle in &scene(spec, body, pose, seed) {
        draw_triangle(&mut high, &mut depths, triangle, angle);
    }
    let mut low = RgbaImage::new(size, size);
    for y in 0..size {
        for x in 0..size {
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
    render::quantize(&mut low, palette);
    if pose.state == "DEATH" {
        for pixel in low.pixels_mut() {
            pixel[3] = render::quantized_alpha((pixel[3] as f32 * (1.0 - pose.phase * 0.2)) as u8);
        }
    }
    low
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

pub fn sheet(
    spec: &MonsterSpec,
    body: &Body,
    poses: &[Pose],
    size: u32,
    seed: u64,
) -> (RgbaImage, u32, u32) {
    let columns = 8;
    let total = poses.len() as u32 * ANGLES_DEG.len() as u32;
    let rows = total.div_ceil(columns);
    let palette = palette(body);
    let mut output = RgbaImage::new(columns * size, rows * size);
    for (direction, degrees) in ANGLES_DEG.iter().enumerate() {
        let angle = degrees.to_radians();
        for (index, pose) in poses.iter().enumerate() {
            let frame = frame(spec, body, pose, size, angle, &palette, seed);
            let frame_id = direction as u32 * poses.len() as u32 + index as u32;
            let ox = frame_id % columns * size;
            let oy = frame_id / columns * size;
            for y in 0..size {
                for x in 0..size {
                    output.put_pixel(ox + x, oy + y, *frame.get_pixel(x, y));
                }
            }
        }
    }
    let color_count = render::color_count(&output) as u32;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;
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
        let (first_weapon, _) = node_pose(&body, weapon_index, movement[1]);
        let (last_weapon, _) = node_pose(&body, weapon_index, movement[4]);
        let (first_hand, _) = node_pose(&body, hand_index, movement[1]);
        let (last_hand, _) = node_pose(&body, hand_index, movement[4]);
        assert!((first_weapon.x - last_weapon.x).abs() > 0.1);
        assert!(((first_weapon.x - last_weapon.x) - (first_hand.x - last_hand.x)).abs() < 0.001);
        assert!(((first_weapon.y - last_weapon.y) - (first_hand.y - last_hand.y)).abs() < 0.001);
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
        let sprite = frame(&spec, &body, idle, 128, 0.0, &colors, 7);
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
            let first = frame(&spec, &body, moving[0], 96, 0.0, &colors, 31);
            let later = frame(&spec, &body, moving[2], 96, 0.0, &colors, 31);
            let changed = first
                .pixels()
                .zip(later.pixels())
                .filter(|(a, b)| (a[3] > 0) != (b[3] > 0))
                .count();
            assert!(changed > 100, "{prompt} has only {changed} moving pixels");
        }
    }
}
