//! Shared V1/V2 world-size policy, independent from texture resolution and poses.
use crate::{
    anatomy::{Body, Node},
    parser::MonsterSpec,
    proto::SizeInfo,
    render3d,
};
use serde::{Deserialize, Serialize};

pub const RECIPE_ID: &str = "scale.player_relative@1";
pub const REFERENCE_PLAYER_HEIGHT_PX: f32 = 96.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SizeModifier {
    #[default]
    Normal,
    Giant,
    Large,
    Small,
    Mini,
}
impl SizeModifier {
    pub fn is_normal(&self) -> bool {
        *self == Self::Normal
    }
    pub fn from_word(word: &str) -> Self {
        match word {
            "giant" | "huge" | "colossal" | "massive" => Self::Giant,
            "large" | "big" => Self::Large,
            "small" | "little" => Self::Small,
            "mini" | "miniature" | "tiny" => Self::Mini,
            _ => Self::Normal,
        }
    }
    /// A dedicated seeded stream: independent of profile, parser, anatomy RNG
    /// consumption and animation frame. No random draw at display time.
    pub fn factor(self, seed: u64) -> f32 {
        let mut value = seed.wrapping_add(0x7369_7a65_5f76_3031);
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        let fraction = ((value >> 40) as u32) as f32 / 16_777_215.0;
        let (low, high) = match self {
            Self::Normal => (1.0, 1.0),
            Self::Giant => (1.65, 2.50),
            Self::Large => (1.15, 1.50),
            Self::Small => (0.55, 0.80),
            Self::Mini => (0.25, 0.45),
        };
        low + (high - low) * fraction
    }
}

/// Gameplay-readable scale, rather than a zoological simulator. Heights are
/// neutral body stature: small wildlife < player < large beasts. Long horizontal
/// creatures keep their authored aspect ratio; wings/tails are not height rulers.
pub fn species_height(spec: &MonsterSpec) -> f32 {
    if spec.features.iter().any(|f| f == "CHIBI") {
        return 64.0;
    }
    // Animal-headed humanoids keep humanoid stature.
    if spec.body_plan == "HUMANOID" {
        return match spec.affinity.as_str() {
            "GOBLIN" | "TOKOLOSHE" => 56.0,
            "ORC" | "GORILLA" | "MINOTAUR" | "DEMONIC" | "ASURA" | "RAKSHASA" => 112.0,
            "TROLL" | "BIGFOOT" | "YETI" | "CYCLOPEAN" | "FOMORIAN" => 152.0,
            "GIANT" => 96.0, // The noun already carries the giant text modifier.
            "JOTUNN" | "BALOR" | "TYPHON" => 192.0,
            "JAR_JAR" => 112.0,
            _ => 96.0,
        };
    }
    match spec.affinity.as_str() {
        "HUMMINGBIRD" => 16.0,
        "SPARROW" => 20.0,
        "INSECT" | "BEETLE" | "SCARAB" | "HORNET" | "CENTIPEDE" => 24.0,
        "SPIDER" | "SCORPION" | "CRAB" | "LOBSTER" | "FROG" | "SNAIL" | "SLUG" => 28.0,
        "RABBIT" | "KILLER_RABBIT" | "BAT" | "PIGEON" | "BIRD" | "FISH" | "EEL" => 32.0,
        "CROW" | "RAVEN" | "FALCON" | "HAWK" | "OWL" | "JELLYFISH" | "MUSHROOM" => 40.0,
        "WOLF" | "JACKAL" | "DOG" | "PIG" | "BOAR" | "WARTHOG" | "GOAT" | "RAM" | "HYENA"
        | "SLIME" | "BLOB" | "OOZE" | "DRONE" => 56.0,
        "LION" | "TIGER" | "LEOPARD" | "PANTHER" | "CHEETAH" | "SABERTOOTH_CAT" | "GAZELLE"
        | "ANTELOPE" | "EAGLE" | "VULTURE" | "HERON" | "IBIS" | "PELICAN" | "CROCODILE"
        | "SERPENT" | "SNAKE" | "COBRA" | "CEPHALOPOD" => 64.0,
        "CONDOR" | "ALBATROSS" | "STRIX" | "SHARK" => 80.0,
        "HORSE" | "ZEBRA" | "BULL" | "BUFFALO" | "KELPIE" | "SLEIPNIR" | "PEGASUS" | "HIPPO"
        | "RHINO" | "RAPTOR" | "SIREN" | "HARPY" | "VALKYRIE" | "BENNU" => 96.0,
        "BEAR" | "MOOSE" | "OSTRICH" | "CERBERUS" | "GRIFFIN" | "SPHINX" | "CENTAUR"
        | "MANTICORE" | "CHIMERIC" | "TANK" | "ANKYLOSAUR" => 128.0,
        "ELEPHANT" | "TRICERATOPS" | "STEGO" | "DRAGON" | "WYVERN" => 160.0,
        "MAMMOTH" | "GIRAFFE" | "ORCA" | "WHALE" | "TREE_MONSTER" | "SEA_DRAGON"
        | "SEA_SERPENT" | "SAND_WORM" | "HYDRA" | "GROOTSLANG" | "MOKELE_MBEMBE" => 192.0,
        "TREX" | "KRAKEN" => 224.0,
        "LONGNECK_DINO" | "JORMUNGANDR" | "APOPHIS" => 288.0,
        _ => match spec.body_plan.as_str() {
            "INSECT" | "ARACHNID" | "GASTROPOD" => 28.0,
            "QUADRUPED" | "SERPENT" | "AMORPHOUS" => 64.0,
            "WINGED" => 64.0,
            "THEROPOD" | "TANK" => 128.0,
            _ => 96.0,
        },
    }
}

pub fn target_height(spec: &MonsterSpec, seed: u64) -> f32 {
    if let Some(height) = spec.world_height_px {
        return height;
    }
    let height = species_height(spec) * spec.world_size_modifier.factor(seed);
    if let Some(mount) = &spec.mount {
        // One composite scalar; rider and mount clause modifiers survive recipe selection.
        let mount_spec = crate::parser::mount_spec(spec, mount);
        height + target_height(&mount_spec, seed ^ 0x004d_4f55_4e54) * 0.65
    } else {
        height
    }
}

pub(crate) fn reference_node(spec: &MonsterSpec, node: &Node) -> bool {
    if node.kind == "TAIL"
        && matches!(
            spec.affinity.as_str(),
            "MERMAID" | "MERMAN" | "NAGA" | "LAMIA"
        )
    {
        return true; // Tail is the lower body, not an accessory.
    }
    !matches!(
        node.kind.as_str(),
        "ARM"
            | "HAND"
            | "WEAPON"
            | "WING"
            | "TAIL"
            | "FIN"
            | "HORN"
            | "ANTLER"
            | "FANG"
            | "TUSK"
            | "CLAW"
            | "STINGER"
            | "SMOKE"
            | "FLAME"
            | "VORTEX"
    ) && !node.id.starts_with("v2_shade_tendril_")
}

pub fn size_info(spec: &MonsterSpec, body: &Body, seed: u64) -> SizeInfo {
    let reference_height_px = render3d::reference_body_height(spec, body, seed);
    let target_height_px = target_height(spec, seed);
    SizeInfo {
        world_scale: target_height_px / reference_height_px,
        reference_height_px,
        target_height_px,
        reference_player_height_px: REFERENCE_PLAYER_HEIGHT_PX,
        ..Default::default()
    }
}

pub fn valid_size_info(size: &SizeInfo) -> bool {
    let fields = [
        size.world_scale,
        size.reference_height_px,
        size.target_height_px,
        size.reference_player_height_px,
    ];
    if fields.iter().all(|v| *v == 0.0) {
        return true;
    } // legacy V1/V2 package
    fields.iter().all(|v| v.is_finite() && *v > 0.0)
        && (size.reference_height_px * size.world_scale - size.target_height_px).abs()
            <= size.target_height_px * 0.0001
}
