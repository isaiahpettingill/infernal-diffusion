use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MonsterSpec {
    pub body_plan: String,
    pub affinity: String,
    pub size: f32,
    pub bulk: f32,
    pub limb_count: u8,
    pub arm_count: u8,
    pub heads: u8,
    pub materials: Vec<String>,
    pub features: Vec<String>,
    #[serde(default)]
    pub part_scales: BTreeMap<String, f32>,
    pub attack: AttackIntent,
    #[serde(default)]
    pub secondary_affinities: Vec<String>,
    #[serde(default)]
    pub spawn: Option<SpawnIntent>,
    #[serde(default)]
    pub mount: Option<MountIntent>,
    pub confidence: f32,
    pub parser: String,
    #[serde(default)]
    pub confidences: BTreeMap<String, f32>,
    #[serde(default)]
    pub token_evidence: Vec<TokenEvidence>,
    #[serde(default)]
    pub description_synonyms: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpawnIntent {
    pub target: String,
    pub count: u8,
    pub max_active: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MountIntent {
    pub body_plan: String,
    pub affinity: String,
    /// RIDER or MOUNT survives the composite creature's defeat.
    pub survivor: String,
    #[serde(default)]
    pub spec: Option<Box<MonsterSpec>>,
}

pub fn mount_spec(parent: &MonsterSpec, mount: &MountIntent) -> MonsterSpec {
    let mut result = mount.spec.as_deref().cloned().unwrap_or_else(|| {
        let mut fallback =
            parse_vocabulary_clause(&mount.affinity.to_lowercase().replace('_', " "));
        fallback.body_plan = mount.body_plan.clone();
        fallback.affinity = mount.affinity.clone();
        fallback
    });
    result.size = result.size.max(parent.size.min(0.75));
    result.spawn = None;
    result
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenEvidence {
    pub label: String,
    pub start: usize,
    pub end: usize,
    pub confidence: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttackIntent {
    pub delivery: String,
    pub element: String,
    pub concept: String,
}

impl MonsterSpec {
    pub fn is_bird(&self) -> bool {
        matches!(
            self.affinity.as_str(),
            "FALCON"
                | "BIRD"
                | "PIGEON"
                | "SPARROW"
                | "IBIS"
                | "VULTURE"
                | "RAVEN"
                | "HAWK"
                | "OWL"
                | "EAGLE"
                | "CROW"
                | "CONDOR"
                | "HERON"
                | "PELICAN"
                | "ALBATROSS"
                | "HUMMINGBIRD"
                | "BENNU"
                | "STRIX"
        )
    }

    pub fn is_dragon(&self) -> bool {
        matches!(self.affinity.as_str(), "DRAGON" | "WYVERN")
    }

    pub fn is_gastropod(&self) -> bool {
        self.body_plan == "GASTROPOD"
    }

    pub fn is_blob(&self) -> bool {
        self.body_plan == "AMORPHOUS" && self.affinity != "CEPHALOPOD"
    }
}

#[cfg(test)]
mod vocabulary_tests {
    use super::*;

    #[derive(Deserialize)]
    struct Example {
        prompt: String,
        body_plan: String,
        affinity: String,
        secondary: Option<String>,
        spawn: Option<String>,
        mount: Option<String>,
    }

    #[test]
    fn curated_semantic_examples_parse() {
        for line in include_str!("../data/semantic_examples.jsonl").lines() {
            let example: Example = serde_json::from_str(line).unwrap();
            let spec = parse_vocabulary(&example.prompt);
            assert_eq!(spec.body_plan, example.body_plan, "{}", example.prompt);
            assert_eq!(spec.affinity, example.affinity, "{}", example.prompt);
            if let Some(secondary) = example.secondary {
                assert!(
                    spec.secondary_affinities.contains(&secondary),
                    "{}",
                    example.prompt
                );
            }
            if let Some(spawn) = example.spawn {
                assert_eq!(
                    spec.spawn.as_ref().map(|s| &s.target),
                    Some(&spawn),
                    "{}",
                    example.prompt
                );
            }
            if let Some(mount) = example.mount {
                assert_eq!(
                    spec.mount.as_ref().map(|m| &m.affinity),
                    Some(&mount),
                    "{}",
                    example.prompt
                );
            }
        }
    }

    #[test]
    fn nearby_size_words_apply_to_their_parts() {
        let sword = parse_vocabulary("orc with a huge sword");
        assert_eq!(sword.size, 0.55);
        assert_eq!(sword.part_scales.get("WEAPON"), Some(&1.8));
        let mixed = parse_vocabulary("huge orc with a tiny sword");
        assert_eq!(mixed.size, 0.9);
        assert_eq!(mixed.part_scales.get("WEAPON"), Some(&0.65));
        let wings = parse_vocabulary("small dragon with huge wings");
        assert_eq!(wings.size, 0.25);
        assert_eq!(wings.part_scales.get("WING"), Some(&1.8));
        let head = parse_vocabulary("troll with a huge head and a huge arm");
        assert_eq!(head.size, 0.55);
        assert_eq!(head.part_scales.get("HEAD_0"), Some(&1.8));
        assert_eq!(head.part_scales.get("ARM_0"), Some(&1.8));
    }

    #[test]
    fn riding_clauses_keep_creature_traits_and_weapons_local() {
        use rand::SeedableRng;
        use rand_chacha::ChaCha8Rng;
        let stack = parse_vocabulary(
            "anime girl with axe riding a two headed ghost pig riding a slime pig",
        );
        assert_eq!(stack.affinity, "ANIME_GIRL");
        assert_eq!(stack.attack.concept, "AXE");
        let middle = stack.mount.as_ref().unwrap().spec.as_ref().unwrap();
        assert_eq!(middle.affinity, "PIG");
        assert_eq!(middle.heads, 2);
        assert!(middle.materials.contains(&"SPECTRAL".into()));
        assert_eq!(middle.attack.concept, "MELEE");
        let bottom = middle.mount.as_ref().unwrap().spec.as_ref().unwrap();
        assert_eq!(bottom.affinity, "PIG");
        assert!(bottom.materials.contains(&"SLIME".into()));
        assert_eq!(bottom.heads, 1);
        assert!(bottom.mount.is_none());
        let body = crate::anatomy::build(&stack, &mut ChaCha8Rng::seed_from_u64(73));
        for (id, material) in [
            ("torso", "SLIME"),
            ("mount_rider_torso", "SPECTRAL"),
            ("rider_torso", "CLOTH_IVORY"),
        ] {
            assert!(
                body.nodes
                    .iter()
                    .any(|node| node.id == id && node.material == material),
                "{id}: {:?}",
                body.nodes
                    .iter()
                    .filter(|node| node.id == id)
                    .map(|node| node.material.as_str())
                    .collect::<Vec<_>>()
            );
        }
        assert!(body
            .nodes
            .iter()
            .any(|node| node.id == "mount_rider_head_1"));
        assert!(body.nodes.iter().any(|node| node.id == "rider_weapon"));
        assert!(!body
            .nodes
            .iter()
            .any(|node| node.id == "mount_rider_weapon"));
    }

    #[test]
    fn repeated_riding_and_standalone_laser_parse() {
        let stack = parse_vocabulary("pig riding a pig riding a pig");
        assert_eq!(stack.affinity, "PIG");
        assert_eq!(stack.mount.as_ref().unwrap().affinity, "PIG");
        assert_eq!(
            stack
                .mount
                .as_ref()
                .unwrap()
                .spec
                .as_ref()
                .unwrap()
                .mount
                .as_ref()
                .unwrap()
                .affinity,
            "PIG"
        );
        let laser = parse_vocabulary("orc with laser");
        assert_eq!(laser.attack.concept, "LASER");
        assert_eq!(laser.attack.delivery, "PROJECTILE");
        assert!(parse_vocabulary("kelpie riding through mist")
            .mount
            .is_none());
        let skeletal_mount = parse_vocabulary("pig riding a skeleton horse");
        assert!(!skeletal_mount.materials.contains(&"BONE".into()));
        assert!(skeletal_mount
            .mount
            .unwrap()
            .spec
            .unwrap()
            .materials
            .contains(&"BONE".into()));
    }

    #[test]
    fn flying_animals_and_new_archetypes_have_local_recipes() {
        for bird in [
            "hawk",
            "vulture",
            "owl",
            "eagle",
            "crow",
            "condor",
            "heron",
            "pelican",
            "albatross",
            "hummingbird",
        ] {
            let spec = parse_vocabulary(bird);
            assert_eq!(spec.body_plan, "WINGED", "{bird}");
            assert!(spec.is_bird(), "{bird}");
            assert!(spec.materials[0].starts_with("FEATHER"), "{bird}");
        }
        let bat = parse_vocabulary("bat");
        assert_eq!(bat.body_plan, "WINGED");
        assert_eq!(bat.materials[0], "FUR_DARK");
        assert!(!bat.is_bird());
        let rabbit = parse_vocabulary("killer rabbit");
        assert_eq!(rabbit.affinity, "KILLER_RABBIT");
        assert!(rabbit.features.contains(&"LONG_EARS".into()));
        let jar = parse_vocabulary("Jar-Jar Binks");
        assert_eq!(jar.affinity, "JAR_JAR");
        assert!(jar.features.contains(&"EYE_STALKS".into()));
        let tree = parse_vocabulary("evil tree monster");
        assert!(tree.features.contains(&"ANCHORED".into()));
        assert_eq!(tree.materials[0], "WOOD");
        assert!(!parse_vocabulary("walking tree monster")
            .features
            .contains(&"ANCHORED".into()));
        let fishman = parse_vocabulary("fish-man with crab hands");
        assert_eq!(fishman.affinity, "FISHMAN");
        assert!(fishman.features.contains(&"PINCER_HANDS".into()));
        assert!(fishman.features.contains(&"GILL".into()));
        assert_eq!(
            parse_vocabulary("skeleton with crab hands").affinity,
            "SKELETON"
        );
        assert!(parse_vocabulary("kraken").size >= 0.9);
    }

    #[test]
    fn prehistoric_and_aquatic_archetypes_are_distinct() {
        use rand::SeedableRng;
        for (prompt, plan, affinity, feature) in [
            ("dino", "QUADRUPED", "LONGNECK_DINO", "LONG_NECK"),
            ("T-Rex", "THEROPOD", "TREX", "TAIL"),
            ("raptor", "THEROPOD", "RAPTOR", "TAIL"),
            ("stegosaurus", "QUADRUPED", "STEGO", "BACK_PLATES"),
            ("triceratops", "QUADRUPED", "TRICERATOPS", "TRIPLE_HORN"),
            ("ankylosaurus", "QUADRUPED", "ANKYLOSAUR", "TAIL_CLUB"),
            ("mammoth", "QUADRUPED", "MAMMOTH", "TRUNK"),
            ("smilodon", "QUADRUPED", "SABERTOOTH_CAT", "SABER_FANGS"),
            ("killer whale", "SERPENT", "ORCA", "FIN"),
            ("harpie", "WINGED", "HARPY", "CLAW"),
            ("siren", "WINGED", "SIREN", "CLAW"),
            ("beetle", "INSECT", "BEETLE", "SHELL"),
        ] {
            let spec = parse_vocabulary(prompt);
            assert_eq!(spec.body_plan, plan, "{prompt}");
            assert_eq!(spec.affinity, affinity, "{prompt}");
            assert!(
                spec.features.iter().any(|value| value == feature),
                "{prompt}"
            );
        }
        let whale = parse_vocabulary("killer whale");
        let body = crate::anatomy::build(&whale, &mut rand_chacha::ChaCha8Rng::seed_from_u64(3));
        let (_, modes, _, _) = crate::animation::make(&whale, &body);
        assert_eq!(modes[0].id, "BEACHED_FLOP");
        assert!(modes[0].speed < 10.0);
        assert_eq!(parse_vocabulary("bracchiosaurus").affinity, "LONGNECK_DINO");
        assert_eq!(parse_vocabulary("stegasaurus").affinity, "STEGO");
        assert_eq!(parse_vocabulary("rhinocerous").affinity, "RHINO");
        let beetle = parse_vocabulary("beetle");
        let insect_body =
            crate::anatomy::build(&beetle, &mut rand_chacha::ChaCha8Rng::seed_from_u64(4));
        assert_eq!(
            insect_body
                .nodes
                .iter()
                .filter(|node| node.kind == "FOOT")
                .count(),
            6
        );
        assert!(insect_body.nodes.iter().any(|node| node.kind == "SHELL"));
    }
}

const CONCEPTS: &[(&str, &str, &str)] = &[
    // Greek and Roman traditions.
    ("typhon", "HUMANOID", "TYPHON"),
    ("echidna", "SERPENT", "ECHIDNA"),
    ("lamia", "SERPENT", "LAMIA"),
    ("pegasus", "WINGED", "PEGASUS"),
    ("siren", "WINGED", "SIREN"),
    ("harpie", "WINGED", "HARPY"),
    ("faun", "HUMANOID", "FAUN"),
    ("strix", "WINGED", "STRIX"),
    ("lemure", "FLOATING", "LEMURE"),
    // Egyptian traditions.
    ("ammit", "QUADRUPED", "AMMIT"),
    ("apophis", "SERPENT", "APOPHIS"),
    ("bennu", "WINGED", "BENNU"),
    ("serpopard", "QUADRUPED", "SERPOPARD"),
    // Indian traditions.
    ("naga", "SERPENT", "NAGA"),
    ("asura", "HUMANOID", "ASURA"),
    ("vetala", "FLOATING", "VETALA"),
    ("yaksha", "HUMANOID", "YAKSHA"),
    ("makara", "QUADRUPED", "MAKARA"),
    ("vanara", "HUMANOID", "VANARA"),
    ("pisacha", "HUMANOID", "PISACHA"),
    // Norse and Celtic traditions.
    ("jormungandr", "SERPENT", "JORMUNGANDR"),
    ("nidhogg", "WINGED", "NIDHOGG"),
    ("draugr", "HUMANOID", "DRAUGR"),
    ("jotunn", "HUMANOID", "JOTUNN"),
    ("huldra", "HUMANOID", "HULDRA"),
    ("valkyrie", "WINGED", "VALKYRIE"),
    ("banshee", "FLOATING", "BANSHEE"),
    ("dullahan", "HUMANOID", "DULLAHAN"),
    ("kelpie", "QUADRUPED", "KELPIE"),
    ("selkie", "QUADRUPED", "SELKIE"),
    ("pooka", "QUADRUPED", "POOKA"),
    ("fomorian", "HUMANOID", "FOMORIAN"),
    ("balor", "HUMANOID", "BALOR"),
    ("cailleach", "HUMANOID", "CAILLEACH"),
    // African folklore. These are regional traditions, not a single pantheon.
    ("grootslang", "SERPENT", "GROOTSLANG"),
    ("tokoloshe", "HUMANOID", "TOKOLOSHE"),
    ("asanbosam", "HUMANOID", "ASANBOSAM"),
    ("inkanyamba", "SERPENT", "INKANYAMBA"),
    ("mokele mbembe", "QUADRUPED", "MOKELE_MBEMBE"),
    // Mexica/Aztec traditions.
    ("ahuizotl", "QUADRUPED", "AHUIZOTL"),
    ("cipactli", "QUADRUPED", "CIPACTLI"),
    ("quetzalcoatl", "SERPENT", "QUETZALCOATL"),
    ("tzitzimitl", "HUMANOID", "TZITZIMITL"),
    ("xolotl", "HUMANOID", "XOLOTL"),
    ("nagual", "HUMANOID", "NAGUAL"),
    // Modern cryptids, fantasy, science fiction, and explicit easter eggs.
    ("sand worm", "SERPENT", "SAND_WORM"),
    ("sandworm", "SERPENT", "SAND_WORM"),
    ("jar jar binks", "HUMANOID", "JAR_JAR"),
    ("jar jar", "HUMANOID", "JAR_JAR"),
    ("killer rabbit", "QUADRUPED", "KILLER_RABBIT"),
    ("rabbit", "QUADRUPED", "RABBIT"),
    ("hare", "QUADRUPED", "RABBIT"),
    ("evil tree", "FUNGUS", "TREE_MONSTER"),
    ("tree monster", "FUNGUS", "TREE_MONSTER"),
    ("treant", "FUNGUS", "TREE_MONSTER"),
    ("ent", "FUNGUS", "TREE_MONSTER"),
    ("merman", "HUMANOID", "MERMAN"),
    ("mermaid", "HUMANOID", "MERMAID"),
    ("fish man", "HUMANOID", "FISHMAN"),
    ("fishman", "HUMANOID", "FISHMAN"),
    ("crab", "ARACHNID", "CRAB"),
    ("guy", "HUMANOID", "HUMANOID"),
    // Sea life and storm creatures.
    ("sharknado", "FLOATING", "SHARKNADO"),
    ("shark tornado", "FLOATING", "SHARKNADO"),
    ("tornado", "FLOATING", "TORNADO"),
    ("whirlwind", "FLOATING", "TORNADO"),
    ("sea dragon", "SERPENT", "SEA_DRAGON"),
    ("sea serpent", "SERPENT", "SEA_SERPENT"),
    ("kraken", "AMORPHOUS", "KRAKEN"),
    ("octopi", "AMORPHOUS", "CEPHALOPOD"),
    ("shark", "SERPENT", "SHARK"),
    ("fish", "SERPENT", "FISH"),
    ("eel", "SERPENT", "EEL"),
    ("whale", "SERPENT", "WHALE"),
    ("killer whale", "SERPENT", "ORCA"),
    ("orca", "SERPENT", "ORCA"),
    ("jellyfish", "FLOATING", "JELLYFISH"),
    ("bigfoot", "HUMANOID", "BIGFOOT"),
    ("sasquatch", "HUMANOID", "BIGFOOT"),
    ("yeti", "HUMANOID", "YETI"),
    ("chupacabra", "QUADRUPED", "CHUPACABRA"),
    ("chubacabra", "QUADRUPED", "CHUPACABRA"),
    ("orc", "HUMANOID", "ORC"),
    ("goblin", "HUMANOID", "GOBLIN"),
    ("troll", "HUMANOID", "TROLL"),
    ("anime girl", "HUMANOID", "ANIME_GIRL"),
    ("evil robot", "HUMANOID", "ROBOT"),
    ("robot", "HUMANOID", "ROBOT"),
    ("alien", "HUMANOID", "ALIEN"),
    ("clown", "HUMANOID", "CLOWN"),
    ("adolph hitler", "HUMANOID", "HITLER"),
    ("adolf hitler", "HUMANOID", "HITLER"),
    ("hitler", "HUMANOID", "HITLER"),
    ("tank", "TANK", "TANK"),
    ("drone", "FLOATING", "DRONE"),
    ("mushroom", "FUNGUS", "MUSHROOM"),
    ("hornet", "WINGED", "HORNET"),
    ("centipede", "ARACHNID", "CENTIPEDE"),
    ("beetle", "INSECT", "BEETLE"),
    // Extinct fauna. The short form intentionally selects the long-necked archetype.
    ("tyrannosaurus rex", "THEROPOD", "TREX"),
    ("tyrannosaurus", "THEROPOD", "TREX"),
    ("t rex", "THEROPOD", "TREX"),
    ("trex", "THEROPOD", "TREX"),
    ("velociraptor", "THEROPOD", "RAPTOR"),
    ("raptor", "THEROPOD", "RAPTOR"),
    ("stegosaurus", "QUADRUPED", "STEGO"),
    ("stego", "QUADRUPED", "STEGO"),
    ("triceratops", "QUADRUPED", "TRICERATOPS"),
    ("trike", "QUADRUPED", "TRICERATOPS"),
    ("ankylosaurus", "QUADRUPED", "ANKYLOSAUR"),
    ("ankylosaur", "QUADRUPED", "ANKYLOSAUR"),
    ("brontosaurus", "QUADRUPED", "LONGNECK_DINO"),
    ("brachiosaurus", "QUADRUPED", "LONGNECK_DINO"),
    ("sauropod", "QUADRUPED", "LONGNECK_DINO"),
    ("dinosaur", "QUADRUPED", "LONGNECK_DINO"),
    ("dino", "QUADRUPED", "LONGNECK_DINO"),
    ("woolly mammoth", "QUADRUPED", "MAMMOTH"),
    ("mammoth", "QUADRUPED", "MAMMOTH"),
    ("saber tooth tiger", "QUADRUPED", "SABERTOOTH_CAT"),
    ("sabertooth tiger", "QUADRUPED", "SABERTOOTH_CAT"),
    ("smilodon", "QUADRUPED", "SABERTOOTH_CAT"),
    ("saber tooth", "QUADRUPED", "SABERTOOTH_CAT"),
    ("sabertooth", "QUADRUPED", "SABERTOOTH_CAT"),
    // Common and safari animal forms.
    ("grizzly", "QUADRUPED", "BEAR"),
    ("polar bear", "QUADRUPED", "BEAR"),
    ("black bear", "QUADRUPED", "BEAR"),
    ("panda", "QUADRUPED", "BEAR"),
    ("bear", "QUADRUPED", "BEAR"),
    ("elk", "QUADRUPED", "MOOSE"),
    ("moose", "QUADRUPED", "MOOSE"),
    ("buffalo", "QUADRUPED", "BUFFALO"),
    ("water buffalo", "QUADRUPED", "BUFFALO"),
    ("bison", "QUADRUPED", "BUFFALO"),
    ("yak", "QUADRUPED", "BUFFALO"),
    ("rhinoceros", "QUADRUPED", "RHINO"),
    ("rhinocerous", "QUADRUPED", "RHINO"),
    ("rhino", "QUADRUPED", "RHINO"),
    ("mammuth", "QUADRUPED", "MAMMOTH"),
    ("mastodon", "QUADRUPED", "MAMMOTH"),
    ("elephant", "QUADRUPED", "ELEPHANT"),
    ("reindeer", "QUADRUPED", "MOOSE"),
    ("hippopotamus", "QUADRUPED", "HIPPO"),
    ("hippo", "QUADRUPED", "HIPPO"),
    ("giraffe", "QUADRUPED", "GIRAFFE"),
    ("camel", "QUADRUPED", "GIRAFFE"),
    ("llama", "QUADRUPED", "GIRAFFE"),
    ("zebra", "QUADRUPED", "ZEBRA"),
    ("donkey", "QUADRUPED", "HORSE"),
    ("mule", "QUADRUPED", "HORSE"),
    ("hyena", "QUADRUPED", "HYENA"),
    ("coyote", "QUADRUPED", "WOLF"),
    ("fox", "QUADRUPED", "WOLF"),
    ("gorilla", "HUMANOID", "GORILLA"),
    ("leopard", "QUADRUPED", "LEOPARD"),
    ("jaguar", "QUADRUPED", "LEOPARD"),
    ("cheetah", "QUADRUPED", "CHEETAH"),
    ("tiger", "QUADRUPED", "TIGER"),
    ("panther", "QUADRUPED", "PANTHER"),
    ("cougar", "QUADRUPED", "PANTHER"),
    ("lynx", "QUADRUPED", "PANTHER"),
    ("warthog", "QUADRUPED", "WARTHOG"),
    ("badger", "QUADRUPED", "BEAR"),
    ("wolverine", "QUADRUPED", "BEAR"),
    ("tapir", "QUADRUPED", "PIG"),
    ("pig", "QUADRUPED", "PIG"),
    ("antelope", "QUADRUPED", "ANTELOPE"),
    ("gazelle", "QUADRUPED", "GAZELLE"),
    ("deer", "QUADRUPED", "ANTELOPE"),
    ("ostrich", "WINGED", "OSTRICH"),
    ("bat", "WINGED", "BAT"),
    ("hawk", "WINGED", "HAWK"),
    ("owl", "WINGED", "OWL"),
    ("eagle", "WINGED", "EAGLE"),
    ("crow", "WINGED", "CROW"),
    ("condor", "WINGED", "CONDOR"),
    ("heron", "WINGED", "HERON"),
    ("pelican", "WINGED", "PELICAN"),
    ("albatross", "WINGED", "ALBATROSS"),
    ("hummingbird", "WINGED", "HUMMINGBIRD"),
    ("pigeon", "WINGED", "PIGEON"),
    ("sparrow", "WINGED", "SPARROW"),
    ("bird", "WINGED", "BIRD"),
    ("skeleton", "HUMANOID", "SKELETON"),
    ("zombie", "HUMANOID", "ZOMBIE"),
    ("phantom", "FLOATING", "PHANTOM"),
    ("anubis", "HUMANOID", "JACKAL"),
    ("sobek", "HUMANOID", "CROCODILE"),
    ("sekhmet", "HUMANOID", "LION"),
    ("bastet", "HUMANOID", "CAT"),
    ("fenrir", "QUADRUPED", "WOLF"),
    ("cerberus", "QUADRUPED", "CERBERUS"),
    ("sleipnir", "QUADRUPED", "SLEIPNIR"),
    ("manticore", "QUADRUPED", "MANTICORE"),
    ("chimera", "QUADRUPED", "CHIMERIC"),
    ("griffin", "QUADRUPED", "GRIFFIN"),
    ("garuda", "WINGED", "GARUDA"),
    ("rakshasa", "HUMANOID", "RAKSHASA"),
    ("djinn", "FLOATING", "DJINN"),
    ("medusa", "HUMANOID", "GORGON"),
    ("gorgon", "HUMANOID", "GORGON"),
    ("spider", "ARACHNID", "SPIDER"),
    ("scorpion", "ARACHNID", "SCORPION"),
    ("scarab", "INSECT", "SCARAB"),
    ("insect", "INSECT", "INSECT"),
    ("centaur", "QUADRUPED", "CENTAUR"),
    ("sphinx", "QUADRUPED", "SPHINX"),
    ("horse", "QUADRUPED", "HORSE"),
    ("bull", "QUADRUPED", "BULL"),
    ("wolf", "QUADRUPED", "WOLF"),
    ("jackal", "QUADRUPED", "JACKAL"),
    ("dog", "QUADRUPED", "DOG"),
    ("lion", "QUADRUPED", "LION"),
    ("boar", "QUADRUPED", "BOAR"),
    ("goat", "QUADRUPED", "GOAT"),
    ("ox", "QUADRUPED", "BULL"),
    ("ram", "QUADRUPED", "RAM"),
    ("crocodile", "QUADRUPED", "CROCODILE"),
    ("alligator", "QUADRUPED", "CROCODILE"),
    ("serpent", "SERPENT", "SERPENT"),
    ("snake", "SERPENT", "SNAKE"),
    ("cobra", "SERPENT", "COBRA"),
    ("hydra", "SERPENT", "HYDRA"),
    ("snail", "GASTROPOD", "SNAIL"),
    ("slug", "GASTROPOD", "SLUG"),
    ("blob", "AMORPHOUS", "BLOB"),
    ("ooze", "AMORPHOUS", "OOZE"),
    ("slime", "AMORPHOUS", "SLIME"),
    ("dragon", "WINGED", "DRAGON"),
    ("wyvern", "WINGED", "WYVERN"),
    ("harpy", "WINGED", "HARPY"),
    ("falcon", "WINGED", "FALCON"),
    ("ibis", "WINGED", "IBIS"),
    ("vulture", "WINGED", "VULTURE"),
    ("raven", "WINGED", "RAVEN"),
    ("squid", "AMORPHOUS", "CEPHALOPOD"),
    ("octopus", "AMORPHOUS", "CEPHALOPOD"),
    ("ghost", "FLOATING", "SPECTRAL"),
    ("specter", "FLOATING", "SPECTRAL"),
    ("wraith", "FLOATING", "SPECTRAL"),
    ("cyclops", "HUMANOID", "CYCLOPEAN"),
    ("minotaur", "HUMANOID", "MINOTAUR"),
    ("satyr", "HUMANOID", "SATYR"),
    ("giant", "HUMANOID", "GIANT"),
    ("demon", "HUMANOID", "DEMONIC"),
    ("archer", "HUMANOID", "ARCHER"),
    ("warrior", "HUMANOID", "WARRIOR"),
    ("knight", "HUMANOID", "KNIGHT"),
    ("humanoid", "HUMANOID", "HUMANOID"),
];

fn contains_word(words: &[&str], needle: &str) -> bool {
    words.contains(&needle)
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (i, a) in left.bytes().enumerate() {
        let mut current = vec![i + 1; right.len() + 1];
        for (j, b) in right.bytes().enumerate() {
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + usize::from(a != b));
        }
        previous = current;
    }
    previous[right.len()]
}

fn closest_creature_typo(words: &[&str]) -> Option<(&'static str, &'static str)> {
    let names = [
        ("tyrannosaurus", "THEROPOD", "TREX"),
        ("velociraptor", "THEROPOD", "RAPTOR"),
        ("stegosaurus", "QUADRUPED", "STEGO"),
        ("triceratops", "QUADRUPED", "TRICERATOPS"),
        ("ankylosaurus", "QUADRUPED", "ANKYLOSAUR"),
        ("brontosaurus", "QUADRUPED", "LONGNECK_DINO"),
        ("brachiosaurus", "QUADRUPED", "LONGNECK_DINO"),
        ("mammoth", "QUADRUPED", "MAMMOTH"),
        ("giraffe", "QUADRUPED", "GIRAFFE"),
        ("rhinoceros", "QUADRUPED", "RHINO"),
        ("elephant", "QUADRUPED", "ELEPHANT"),
        ("crocodile", "QUADRUPED", "CROCODILE"),
        ("kangaroo", "QUADRUPED", "RABBIT"),
    ];
    words
        .iter()
        .filter(|word| word.len() >= 6)
        .flat_map(|word| {
            names.iter().filter_map(move |(name, plan, affinity)| {
                let distance = edit_distance(word, name);
                (distance <= if name.len() >= 11 { 3 } else { 2 })
                    .then_some((distance, *plan, *affinity))
            })
        })
        .min_by_key(|(distance, _, _)| *distance)
        .map(|(_, plan, affinity)| (plan, affinity))
}

fn phrase_at(words: &[&str], phrase: &str) -> Option<usize> {
    let parts: Vec<&str> = phrase.split_whitespace().collect();
    words.windows(parts.len()).position(|window| {
        window
            .iter()
            .zip(&parts)
            .enumerate()
            .all(|(index, (word, part))| {
                *word == *part
                    || (index + 1 == parts.len()
                        && part.len() > 3
                        && (word.strip_suffix('s') == Some(*part)
                            || word.strip_suffix("es") == Some(*part)))
            })
    })
}

fn modifier_affinity(affinity: &str) -> bool {
    [
        "SKELETON", "ZOMBIE", "SPECTRAL", "PHANTOM", "GIANT", "SLIME", "OOZE", "BLOB",
    ]
    .contains(&affinity)
}

pub fn parse_vocabulary(prompt: &str) -> MonsterSpec {
    let lower = prompt.to_lowercase();
    let clauses: Vec<&str> = lower.split(" riding ").take(4).collect();
    if clauses.len() == 1 {
        return parse_vocabulary_clause(prompt);
    }
    let parsed: Vec<MonsterSpec> = clauses
        .iter()
        .map(|clause| parse_vocabulary_clause(clause))
        .collect();
    if parsed.iter().any(|spec| spec.affinity == "UNKNOWN") {
        return parse_vocabulary_clause(prompt);
    }
    let mut next = parsed[parsed.len() - 1].clone();
    for mut rider in parsed[..parsed.len() - 1].iter().rev().cloned() {
        let survivor = if lower.contains("mount survives")
            || lower.contains("leave the mount")
            || lower.contains("rider dies")
        {
            "MOUNT"
        } else if lower.contains("rider survives")
            || lower.contains("leave the rider")
            || lower.contains("mount dies")
        {
            "RIDER"
        } else {
            "EITHER"
        };
        rider.mount = Some(MountIntent {
            body_plan: next.body_plan.clone(),
            affinity: next.affinity.clone(),
            survivor: survivor.into(),
            spec: Some(Box::new(next)),
        });
        rider.size = rider.size.max(0.65);
        next = rider;
    }
    next
}

fn parse_vocabulary_clause(prompt: &str) -> MonsterSpec {
    let lower = prompt.to_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut matches: Vec<_> = CONCEPTS
        .iter()
        .filter_map(|concept| {
            phrase_at(&words, concept.0)
                .map(|start| (start, concept.0.split_whitespace().count(), concept))
        })
        .collect();
    matches.sort_by_key(|(start, length, _)| (*start, std::cmp::Reverse(*length)));
    let mut selected = Vec::new();
    for candidate in matches {
        if selected
            .iter()
            .any(|(start, length, _): &(usize, usize, _)| {
                candidate.0 < start + length && *start < candidate.0 + candidate.1
            })
        {
            continue;
        }
        selected.push(candidate);
    }
    let boundary = words
        .iter()
        .position(|word| ["with", "holding", "wielding"].contains(word))
        .unwrap_or(words.len());
    let subject = selected
        .iter()
        .filter(|(start, _, _)| *start < boundary)
        .collect::<Vec<_>>();
    let found = subject
        .iter()
        .find(|(_, _, (_, _, affinity))| !modifier_affinity(affinity))
        .copied()
        .or_else(|| subject.first().copied())
        .or_else(|| {
            selected
                .iter()
                .find(|(_, _, (_, _, affinity))| !modifier_affinity(affinity))
        })
        .or_else(|| selected.first());
    let typo_match = if found.is_none() {
        closest_creature_typo(&words)
    } else {
        None
    };
    let (body_plan, affinity) = found
        .map(|(_, _, (_, plan, affinity))| (*plan, *affinity))
        .or(typo_match)
        .unwrap_or(("HUMANOID", "UNKNOWN"));
    let primary_start = found.map(|(start, _, _)| *start);
    let mount: Option<MountIntent> = None;
    let spawn_marker = words.iter().position(|word| {
        [
            "spawn", "spawns", "summon", "summons", "hatch", "hatches", "minions",
        ]
        .contains(word)
    });
    let spawn_target = spawn_marker.and_then(|marker| {
        selected.iter().find(|(start, _, (_, _, candidate))| {
            *start > marker && !modifier_affinity(candidate) && Some(*start) != primary_start
        })
    });
    let spawn = spawn_marker
        .map(|_| SpawnIntent {
            target: spawn_target
                .map(|(_, _, (_, _, candidate))| (*candidate).into())
                .unwrap_or("SELF".into()),
            count: if contains_word(&words, "swarm") || contains_word(&words, "many") {
                3
            } else {
                1
            },
            max_active: 4,
        })
        .or_else(|| {
            if ["SHARKNADO", "SHARK", "TORNADO"].contains(&affinity) {
                Some(SpawnIntent {
                    target: "SHARK".into(),
                    count: 2,
                    max_active: 5,
                })
            } else {
                None
            }
        });
    let mut secondary_affinities: Vec<String> = selected
        .iter()
        .filter(|(start, _, (_, _, candidate))| {
            Some(*start) != primary_start
                && spawn_target.is_none_or(|(spawn_start, _, _)| start != spawn_start)
                && !modifier_affinity(candidate)
                && *candidate != affinity
        })
        .map(|(_, _, (_, _, candidate))| (*candidate).to_string())
        .take(2)
        .collect();
    if secondary_affinities.is_empty() {
        for companion in match affinity {
            "AMMIT" => &["CROCODILE", "LION"][..],
            "GROOTSLANG" => &["ELEPHANT"][..],
            "CHIMERIC" => &["GOAT", "SERPENT"][..],
            "SPHINX" => &["HUMANOID"][..],
            "QUETZALCOATL" => &["FALCON"][..],
            _ => &[][..],
        } {
            secondary_affinities.push((*companion).into());
        }
    }
    let mut part_scales = BTreeMap::new();
    let mut large_creature = false;
    let mut small_creature = false;
    for (index, word) in words.iter().enumerate() {
        let factor = if ["giant", "huge", "colossal", "massive"].contains(word) {
            1.8
        } else if ["tiny", "small", "little"].contains(word) {
            0.65
        } else {
            continue;
        };
        let target = words
            .iter()
            .enumerate()
            .skip(index + 1)
            .take(3)
            .find_map(|(at, noun)| {
                let part = match *noun {
                    "head" => Some("HEAD_0"),
                    "heads" => Some("HEAD"),
                    "arm" => Some("ARM_0"),
                    "arms" => Some("ARM"),
                    "wing" => Some("WING_0"),
                    "wings" => Some("WING"),
                    "leg" => Some("LEG_0"),
                    "legs" => Some("LEG"),
                    "tail" | "tails" => Some("TAIL"),
                    "horn" => Some("HORN_0"),
                    "horns" => Some("HORN"),
                    "eye" => Some("EYE_0"),
                    "eyes" => Some("EYE"),
                    "claw" | "claws" => Some("CLAW"),
                    "sword" | "axe" | "club" | "spear" | "bow" | "shotgun" | "chainsaw" | "gun"
                    | "weapon" => Some("WEAPON"),
                    _ => None,
                };
                if part.is_some() {
                    return part;
                }
                if selected
                    .iter()
                    .any(|(start, length, _)| at >= *start && at < start + length)
                    || ["monster", "creature", "beast", "body"].contains(noun)
                {
                    return Some("CREATURE");
                }
                None
            })
            .unwrap_or("CREATURE");
        if target == "CREATURE" {
            if factor > 1.0 {
                large_creature = true;
            } else {
                small_creature = true;
            }
        } else {
            part_scales.insert(target.into(), factor);
        }
    }
    let size: f32 = if large_creature {
        0.9
    } else if small_creature {
        0.25
    } else if affinity == "GOBLIN" || affinity == "TOKOLOSHE" {
        0.3
    } else if affinity == "HITLER" {
        0.43
    } else if affinity == "KRAKEN" {
        0.95
    } else if matches!(affinity, "LONGNECK_DINO" | "TREX" | "MAMMOTH" | "ORCA") {
        0.86
    } else if matches!(affinity, "STEGO" | "TRICERATOPS" | "ANKYLOSAUR") {
        0.78
    } else if affinity == "RAPTOR" {
        0.47
    } else if affinity == "RABBIT" || affinity == "KILLER_RABBIT" {
        0.28
    } else if affinity == "JAR_JAR" {
        0.62
    } else if affinity == "TREE_MONSTER" {
        0.78
    } else if affinity == "HUMMINGBIRD" {
        0.2
    } else if affinity == "BAT" {
        0.32
    } else if matches!(affinity, "CONDOR" | "ALBATROSS") {
        0.75
    } else {
        0.55
    };
    let size = if mount.is_some() {
        size.max(0.65)
    } else {
        size
    };
    let bulk = if affinity == "KRAKEN"
        || affinity == "TREE_MONSTER"
        || matches!(affinity, "MAMMOTH" | "ANKYLOSAUR" | "TRICERATOPS")
    {
        0.85
    } else if affinity == "JAR_JAR" {
        0.25
    } else if words
        .iter()
        .any(|w| ["heavy", "bulky", "fat", "armored"].contains(w))
    {
        0.8
    } else if words
        .iter()
        .any(|w| ["thin", "gaunt", "skeletal"].contains(w))
    {
        0.25
    } else {
        0.5
    };
    let mut materials = Vec::new();
    for (word, material) in [
        ("skeletal", "BONE"),
        ("skeleton", "BONE"),
        ("bone", "BONE"),
        ("mummy", "WRAPPING"),
        ("mummified", "WRAPPING"),
        ("ghost", "SPECTRAL"),
        ("phantom", "SPECTRAL"),
        ("wraith", "SPECTRAL"),
        ("spectral", "SPECTRAL"),
        ("zombie", "ROTTEN"),
        ("draugr", "ROTTEN"),
        ("mushroom", "FUNGUS"),
        ("fungal", "FUNGUS"),
        ("alien", "ALIEN_SKIN"),
        ("robot", "METAL"),
        ("tank", "METAL"),
        ("drone", "METAL"),
        ("translucent", "SPECTRAL"),
        ("metal", "METAL"),
        ("mechanical", "METAL"),
        ("stone", "STONE"),
        ("wood", "WOOD"),
        ("fur", "FUR"),
        ("feathered", "FEATHER"),
        ("scaly", "SCALES"),
        ("slimy", "SLIME"),
        ("slime", "SLIME"),
        ("ooze", "SLIME"),
        ("chitin", "CHITIN"),
        ("cancerous", "TUMOR"),
        ("tumor", "TUMOR"),
    ] {
        if contains_word(&words, word) && !materials.contains(&material.to_string()) {
            materials.push(material.to_string());
        }
    }
    if materials.is_empty() {
        materials.push(
            match body_plan {
                "ARACHNID" | "INSECT" => "CHITIN",
                "SERPENT" if affinity == "SAND_WORM" => "SAND_HIDE",
                "SERPENT" if affinity == "SHARK" => "SHARK_HIDE",
                "SERPENT" if affinity == "ORCA" => "ORCA_HIDE",
                "FLOATING" if ["SHARKNADO", "TORNADO"].contains(&affinity) => "STORM",
                "FLOATING" if affinity == "JELLYFISH" => "MEMBRANE",
                "SERPENT" => "SCALES",
                "AMORPHOUS" if affinity == "KRAKEN" => "DEEP_SEA",
                "GASTROPOD" | "AMORPHOUS" => "SLIME",
                "FUNGUS" if affinity == "TREE_MONSTER" => "WOOD",
                "FUNGUS" => "FUNGUS",
                "HUMANOID" if affinity == "JAR_JAR" => "GUNGAN_SKIN",
                "HUMANOID" if matches!(affinity, "FISHMAN" | "MERMAN" | "MERMAID") => "SCALES",
                "TANK" => "METAL",
                "WINGED" if matches!(affinity, "DRAGON" | "WYVERN") => "SCALES",
                "WINGED" if affinity == "BAT" => "FUR_DARK",
                "WINGED" if matches!(affinity, "RAVEN" | "CROW" | "CONDOR") => "FEATHER_BLACK",
                "WINGED" if matches!(affinity, "HAWK" | "OWL" | "VULTURE" | "HERON") => {
                    "FEATHER_BROWN"
                }
                "WINGED" if matches!(affinity, "EAGLE" | "ALBATROSS" | "PELICAN") => {
                    "FEATHER_WHITE"
                }
                "WINGED" if affinity == "HUMMINGBIRD" => "FEATHER_GOLD",
                "WINGED" if matches!(affinity, "SIREN" | "HARPY") => "FEATHER_BROWN",
                "WINGED" => "FEATHER",
                "FLOATING" => "SPECTRAL",
                "QUADRUPED" if matches!(affinity, "RABBIT" | "KILLER_RABBIT") => "FUR_WHITE",
                "QUADRUPED" if affinity == "MAMMOTH" => "FUR_BROWN",
                "QUADRUPED" if affinity == "SABERTOOTH_CAT" => "FUR_GOLD",
                "QUADRUPED"
                    if matches!(
                        affinity,
                        "LONGNECK_DINO" | "STEGO" | "TRICERATOPS" | "ANKYLOSAUR"
                    ) =>
                {
                    "SCALES"
                }
                "THEROPOD" => "SCALES",
                "QUADRUPED"
                    if [
                        "BEAR", "MOOSE", "BUFFALO", "WOLF", "HYENA", "LEOPARD", "PANTHER",
                    ]
                    .contains(&affinity) =>
                {
                    "FUR"
                }
                "QUADRUPED" if ["LION", "CHEETAH", "TIGER", "GAZELLE"].contains(&affinity) => {
                    "FUR_GOLD"
                }
                "HUMANOID" if affinity == "YETI" => "FUR_WHITE",
                "HUMANOID" if affinity == "BIGFOOT" => "FUR_DARK",
                _ => "FLESH",
            }
            .into(),
        );
    }
    let mut features = Vec::new();
    for (word, feature) in [
        ("horned", "HORN"),
        ("horns", "HORN"),
        ("fangs", "FANG"),
        ("claws", "CLAW"),
        ("wings", "WING"),
        ("winged", "WING"),
        ("tail", "TAIL"),
        ("eyes", "MANY_EYES"),
        ("pustules", "PUSTULE"),
        ("tentacles", "TENTACLE"),
        ("antlers", "ANTLER"),
        ("tusks", "TUSK"),
        ("shell", "SHELL"),
        ("smoke", "SMOKE"),
        ("flaming", "FLAME"),
        ("burrow", "BURROW"),
        ("burrowing", "BURROW"),
        ("climb", "CLIMB"),
        ("climbing", "CLIMB"),
        ("limping", "LIMP"),
        ("hopping", "HOP"),
        ("spores", "SPORES"),
        ("spore", "SPORES"),
        ("moustache", "MOUSTACHE"),
        ("mustache", "MOUSTACHE"),
        ("chainsaw", "CHAINSAW"),
    ] {
        if contains_word(&words, word) && !features.contains(&feature.to_string()) {
            features.push(feature.into());
        }
    }
    if contains_word(&words, "laser") && contains_word(&words, "eyes") {
        features.push("EYE_LASER".into());
    }
    for (recipe_affinity, feature) in [
        ("MANTICORE", "TAIL"),
        ("SCORPION", "TAIL"),
        ("GRIFFIN", "WING"),
        ("CEPHALOPOD", "TENTACLE"),
        ("KRAKEN", "TENTACLE"),
        ("SHARK", "FIN"),
        ("SHARKNADO", "FIN"),
        ("SEA_DRAGON", "FIN"),
        ("SEA_SERPENT", "FIN"),
        ("FISH", "FIN"),
        ("ORCA", "FIN"),
        ("WHALE", "FIN"),
        ("TREX", "TAIL"),
        ("RAPTOR", "TAIL"),
        ("LONGNECK_DINO", "TAIL"),
        ("LONGNECK_DINO", "LONG_NECK"),
        ("STEGO", "TAIL"),
        ("STEGO", "BACK_PLATES"),
        ("TRICERATOPS", "FRILL"),
        ("TRICERATOPS", "TRIPLE_HORN"),
        ("ANKYLOSAUR", "ARMOR_PLATES"),
        ("ANKYLOSAUR", "TAIL_CLUB"),
        ("MAMMOTH", "TUSK"),
        ("MAMMOTH", "TRUNK"),
        ("SABERTOOTH_CAT", "SABER_FANGS"),
        ("SIREN", "CLAW"),
        ("HARPY", "CLAW"),
        ("SCARAB", "SHELL"),
        ("BEETLE", "SHELL"),
        ("TORNADO", "WHIRLWIND"),
        ("SHARKNADO", "WHIRLWIND"),
        ("SHARK", "WHIRLWIND"),
        ("GORGON", "MANY_EYES"),
        ("SNAIL", "SHELL"),
        ("DRAGON", "TAIL"),
        ("WYVERN", "TAIL"),
        ("MOOSE", "ANTLER"),
        ("RHINO", "HORN"),
        ("BUFFALO", "HORN"),
        ("BEAR", "CLAW"),
        ("BIGFOOT", "FUR"),
        ("YETI", "FUR"),
        ("CHUPACABRA", "FANG"),
        ("HORNET", "WING"),
        ("HORNET", "STINGER"),
        ("CENTIPEDE", "FANG"),
        ("SAND_WORM", "BURROW"),
        ("SAND_WORM", "FANG"),
        ("MUSHROOM", "SPORES"),
        ("DRONE", "ROTOR"),
        ("TANK", "TREAD"),
        ("ALIEN", "LARGE_HEAD"),
        ("CLOWN", "CLOWN_FACE"),
        ("ANIME_GIRL", "ANIME_FACE"),
        ("HITLER", "HITLER_FACE"),
        ("QUETZALCOATL", "WING"),
        ("QUETZALCOATL", "FEATHER"),
        ("GROOTSLANG", "TUSK"),
        ("AMMIT", "FANG"),
        ("AHUIZOTL", "TAIL"),
    ] {
        if affinity == recipe_affinity && !features.iter().any(|f| f == feature) {
            features.push(feature.into());
        }
    }
    if affinity == "SKELETON" && !features.iter().any(|f| f == "SKELETAL") {
        features.push("SKELETAL".into());
    }
    if contains_word(&words, "skeleton") || contains_word(&words, "skeletal") {
        features.push("SKELETAL".into());
    }
    if affinity == "ZOMBIE" || contains_word(&words, "zombie") {
        features.push("UNDEAD".into());
    }
    if ["PHANTOM", "SPECTRAL"].contains(&affinity)
        || contains_word(&words, "ghost")
        || contains_word(&words, "phantom")
    {
        features.push("PHASE".into());
    }
    if matches!(affinity, "RABBIT" | "KILLER_RABBIT" | "JAR_JAR") {
        features.push("LONG_EARS".into());
    }
    if affinity == "KILLER_RABBIT" {
        features.push("FANG".into());
        features.push("HOP".into());
    }
    if words.windows(2).any(|pair| pair == ["saber", "tooth"])
        || contains_word(&words, "sabertooth")
        || contains_word(&words, "smilodon")
    {
        features.push("SABER_FANGS".into());
    }
    if affinity == "JAR_JAR" {
        features.push("EYE_STALKS".into());
        features.push("LONG_SNOUT".into());
    }
    if matches!(affinity, "FISHMAN" | "MERMAN" | "MERMAID") {
        features.extend(["FIN".into(), "GILL".into(), "WEBBED".into()]);
    }
    if affinity == "TREE_MONSTER" {
        features.push("ROOTS".into());
    }
    if contains_word(&words, "anchored")
        || contains_word(&words, "rooted")
        || (affinity == "TREE_MONSTER" && !contains_word(&words, "walking"))
    {
        features.push("ANCHORED".into());
    }
    if contains_word(&words, "crab") && contains_word(&words, "hands") {
        features.push("PINCER_HANDS".into());
    }
    if affinity == "KRAKEN" {
        features.push("TENTACLE".into());
    }
    let element = [
        "fire",
        "ice",
        "poison",
        "venom",
        "lightning",
        "acid",
        "shadow",
        "plasma",
    ]
    .iter()
    .find(|w| contains_word(&words, w))
    .map(|x| x.to_uppercase())
    .unwrap_or("PHYSICAL".into());
    let modern = [
        "laser",
        "shotgun",
        "tnt",
        "rocket",
        "missile",
        "plasma",
        "cybernetic",
        "mechanical",
        "railgun",
        "cannon",
    ]
    .iter()
    .find(|w| contains_word(&words, w))
    .map(|x| x.to_uppercase());
    let equipment = [
        "sword", "club", "axe", "spear", "bow", "arrow", "arrows", "chainsaw",
    ]
    .iter()
    .find(|w| contains_word(&words, w))
    .map(|x| match *x {
        "arrow" | "arrows" => "BOW".to_string(),
        other => other.to_uppercase(),
    });
    let concept = modern.or(equipment).unwrap_or_else(|| {
        if words
            .iter()
            .any(|w| ["shoots", "spits", "breathes", "fires"].contains(w))
        {
            "PROJECTILE".into()
        } else if affinity == "TANK" {
            "CANNON".into()
        } else if affinity == "DRONE" {
            "LASER".into()
        } else if affinity == "MUSHROOM" {
            "SPORES".into()
        } else {
            "MELEE".into()
        }
    });
    let delivery = if [
        "LASER",
        "SHOTGUN",
        "ROCKET",
        "MISSILE",
        "PLASMA",
        "RAILGUN",
        "CANNON",
        "BOW",
        "PROJECTILE",
        "SPORES",
    ]
    .contains(&concept.as_str())
    {
        "PROJECTILE"
    } else {
        "MELEE"
    };
    let mut limb_count = match body_plan {
        "ARACHNID" if affinity == "CENTIPEDE" => 12,
        "ARACHNID" => 8,
        "INSECT" => 6,
        "QUADRUPED" => 4,
        "THEROPOD" => 2,
        "SERPENT" | "GASTROPOD" | "FLOATING" | "AMORPHOUS" | "FUNGUS" | "TANK" => 0,
        "WINGED" if matches!(affinity, "DRAGON" | "WYVERN") => 4,
        _ => 2,
    };
    if affinity == "SLEIPNIR" {
        limb_count = 8;
    }
    let mut arm_count = if body_plan == "HUMANOID"
        || affinity == "TREE_MONSTER"
        || matches!(affinity, "TREX" | "RAPTOR" | "SIREN" | "HARPY")
    {
        2
    } else {
        0
    };
    for (word, count) in [
        ("six", 6),
        ("eight", 8),
        ("four", 4),
        ("three", 3),
        ("one", 1),
    ] {
        if contains_word(&words, word) {
            if contains_word(&words, "arms") || contains_word(&words, "armed") {
                arm_count = count;
            } else if words.iter().any(|w| ["legs", "limbs"].contains(w)) {
                limb_count = count;
            }
        }
    }
    let many_heads = words.iter().any(|w| ["heads", "headed"].contains(w));
    let heads = if many_heads {
        [("five", 5), ("four", 4), ("three", 3), ("two", 2)]
            .iter()
            .find(|(word, _)| contains_word(&words, word))
            .map(|(_, count)| *count)
            .unwrap_or(
                if contains_word(&words, "multi") || contains_word(&words, "multiple") {
                    3
                } else {
                    1
                },
            )
    } else if ["HYDRA", "CERBERUS", "TYPHON"].contains(&affinity) {
        3
    } else {
        1
    };
    let base_confidence = if found.is_some() {
        0.87
    } else if typo_match.is_some() {
        0.68
    } else {
        0.35
    };
    let confidences = BTreeMap::from([
        ("body_plan".into(), base_confidence),
        ("affinity".into(), base_confidence),
        (
            "attack.delivery".into(),
            if concept == "MELEE" { 0.55 } else { 0.9 },
        ),
        (
            "attack.element".into(),
            if element == "PHYSICAL" { 0.55 } else { 0.9 },
        ),
    ]);
    MonsterSpec {
        body_plan: body_plan.into(),
        affinity: affinity.into(),
        size,
        bulk,
        limb_count,
        arm_count,
        heads,
        materials,
        features,
        part_scales,
        attack: AttackIntent {
            delivery: delivery.into(),
            element,
            concept,
        },
        secondary_affinities,
        spawn,
        mount,
        confidence: base_confidence,
        parser: "vocabulary-v1".into(),
        confidences,
        token_evidence: Vec::new(),
        description_synonyms: BTreeMap::new(),
    }
}

#[cfg(feature = "bert")]
#[derive(Deserialize)]
struct SemanticHead {
    labels: Vec<String>,
    weights: Vec<Vec<f32>>,
    biases: Vec<f32>,
    #[serde(default = "default_threshold")]
    threshold: f32,
    #[serde(default)]
    token_labels: Vec<String>,
    #[serde(default)]
    token_weights: Vec<Vec<f32>>,
    #[serde(default)]
    token_biases: Vec<f32>,
}
#[cfg(feature = "bert")]
fn default_threshold() -> f32 {
    0.7
}
#[cfg(feature = "bert")]
fn probability(weights: &[f32], bias: f32, embedding: &[f32]) -> Option<f32> {
    if weights.len() != embedding.len()
        || !bias.is_finite()
        || !weights.iter().all(|v| v.is_finite())
    {
        return None;
    }
    let score = weights
        .iter()
        .zip(embedding)
        .fold(bias, |sum, (a, b)| sum + a * b);
    Some(1.0 / (1.0 + (-score.clamp(-40.0, 40.0)).exp()))
}
#[cfg(feature = "bert")]
pub fn parse_with_bert(
    prompt: &str,
    model_dir: &std::path::Path,
) -> Result<MonsterSpec, Box<dyn std::error::Error + Send + Sync>> {
    let config = std::fs::read(model_dir.join("config.json"))?;
    let tokenizer = std::fs::read(model_dir.join("tokenizer.json"))?;
    let weights = std::fs::read(model_dir.join("model.safetensors"))?;
    let head_path = model_dir.join("semantic_head.json");
    let head = if head_path.exists() {
        Some(std::fs::read(head_path)?)
    } else {
        None
    };
    parse_with_bert_bytes(prompt, &config, &tokenizer, weights, head.as_deref())
}

#[cfg(feature = "bert")]
pub fn parse_with_embedded_bert(
    prompt: &str,
) -> Result<MonsterSpec, Box<dyn std::error::Error + Send + Sync>> {
    thread_local! {
        static MODEL: std::cell::OnceCell<BertRuntime> = const { std::cell::OnceCell::new() };
    }
    MODEL.with(|cell| {
        if cell.get().is_none() {
            let runtime = BertRuntime::new(
                include_bytes!("../models/bert-mini/config.json"),
                include_bytes!("../models/bert-mini/tokenizer.json"),
                include_bytes!("../models/bert-mini/model.safetensors").to_vec(),
            )?;
            let _ = cell.set(runtime);
        }
        parse_with_bert_runtime(prompt, cell.get().expect("initialized above"), None)
    })
}

#[cfg(feature = "bert")]
struct BertRuntime {
    device: candle_core::Device,
    tokenizer: tokenizers::Tokenizer,
    model: candle_transformers::models::bert::BertModel,
    candidates: std::cell::RefCell<std::collections::HashMap<String, Vec<f32>>>,
}

#[cfg(feature = "bert")]
impl BertRuntime {
    fn new(
        config_json: &[u8],
        tokenizer_json: &[u8],
        weights: Vec<u8>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use candle_core::{DType, Device};
        use candle_nn::VarBuilder;
        use candle_transformers::models::bert::{BertModel, Config};
        use tokenizers::Tokenizer;
        let device = Device::Cpu;
        let config: Config = serde_json::from_slice(config_json)?;
        let tokenizer = Tokenizer::from_bytes(tokenizer_json)?;
        let vb = VarBuilder::from_buffered_safetensors(weights, DType::F32, &device)?;
        let model = BertModel::load(vb, &config)?;
        Ok(Self {
            device,
            tokenizer,
            model,
            candidates: std::cell::RefCell::new(std::collections::HashMap::new()),
        })
    }

    fn embed(
        &self,
        text: &str,
    ) -> Result<
        (Vec<f32>, Vec<Vec<f32>>, Vec<(usize, usize)>),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        use candle_core::{IndexOp, Tensor};
        let encoding = self.tokenizer.encode(text, true)?;
        let ids = encoding.get_ids();
        let token_types = encoding.get_type_ids();
        let mask = encoding.get_attention_mask();
        let ids = Tensor::new(ids, &self.device)?.unsqueeze(0)?;
        let token_types = Tensor::new(token_types, &self.device)?.unsqueeze(0)?;
        let mask = Tensor::new(mask, &self.device)?.unsqueeze(0)?;
        let output = self.model.forward(&ids, &token_types, Some(&mask))?;
        let tokens = output.i(0)?.to_vec2::<f32>()?;
        Ok((tokens[0].clone(), tokens, encoding.get_offsets().to_vec()))
    }

    fn candidate(&self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(value) = self.candidates.borrow().get(text) {
            return Ok(value.clone());
        }
        let value = self.embed(text)?.0;
        let mut cache = self.candidates.borrow_mut();
        if cache.len() < 4096 {
            cache.insert(text.to_owned(), value.clone());
        }
        Ok(value)
    }
}

#[cfg(feature = "bert")]
fn parse_with_bert_bytes(
    prompt: &str,
    config_json: &[u8],
    tokenizer_json: &[u8],
    weights: Vec<u8>,
    head_json: Option<&[u8]>,
) -> Result<MonsterSpec, Box<dyn std::error::Error + Send + Sync>> {
    let runtime = BertRuntime::new(config_json, tokenizer_json, weights)?;
    parse_with_bert_runtime(prompt, &runtime, head_json)
}

#[cfg(feature = "bert")]
fn parse_with_bert_runtime(
    prompt: &str,
    runtime: &BertRuntime,
    head_json: Option<&[u8]>,
) -> Result<MonsterSpec, Box<dyn std::error::Error + Send + Sync>> {
    let (query, tokens, offsets) = runtime.embed(prompt)?;
    let mut spec = parse_vocabulary(prompt);
    let mut best = (f32::NEG_INFINITY, "HUMANOID", "UNKNOWN");
    let mut runner_up = f32::NEG_INFINITY;
    if spec.affinity == "UNKNOWN" {
        for (word, plan, affinity) in CONCEPTS {
            let candidate = runtime.candidate(&format!("a {word} monster"))?;
            let dot: f32 = query.iter().zip(&candidate).map(|(a, b)| a * b).sum();
            let qa: f32 = query.iter().map(|v| v * v).sum::<f32>().sqrt();
            let ca: f32 = candidate.iter().map(|v| v * v).sum::<f32>().sqrt();
            let score = dot / (qa * ca).max(1e-6);
            if score > best.0 {
                runner_up = best.0;
                best = (score, plan, affinity);
            } else if score > runner_up {
                runner_up = score;
            }
        }
    }
    // Untrained CLS similarity is often high for every concept. Require a clear
    // separation before replacing an unknown vocabulary result.
    let prototype_confidence = if spec.affinity == "UNKNOWN" {
        ((best.0 - runner_up - 0.02) * 6.0).clamp(0.0, 0.85)
    } else {
        0.0
    };
    if spec.affinity == "UNKNOWN" && best.0 > 0.78 && prototype_confidence > 0.65 {
        spec.body_plan = best.1.into();
        spec.affinity = best.2.into();
        spec.confidences
            .insert("body_plan".into(), prototype_confidence);
        spec.confidences
            .insert("affinity".into(), prototype_confidence);
    }
    spec.confidence = spec.confidence.max(prototype_confidence);
    spec.parser = "candle-bert-prototypes-v2".into();
    if let Some(head_json) = head_json {
        let head: SemanticHead = serde_json::from_slice(head_json)?;
        if head.labels.len() != head.weights.len()
            || head.labels.len() != head.biases.len()
            || head.token_labels.len() != head.token_weights.len()
            || head.token_labels.len() != head.token_biases.len()
            || !(0.0..=1.0).contains(&head.threshold)
        {
            return Err("invalid semantic_head.json dimensions".into());
        }
        for ((label, weights), bias) in head.labels.iter().zip(&head.weights).zip(&head.biases) {
            let Some(score) = probability(weights, *bias, &query) else {
                return Err("semantic head hidden size mismatch".into());
            };
            if score < head.threshold {
                continue;
            }
            let Some((category, value)) = label.split_once(':') else {
                continue;
            };
            let confidence_key = match category {
                "BODY_PLAN" => "body_plan",
                "AFFINITY" => "affinity",
                "ATTACK_ELEMENT" => "attack.element",
                "ATTACK_DELIVERY" => "attack.delivery",
                _ => category,
            };
            let current = spec.confidences.get(confidence_key).copied().unwrap_or(0.0);
            if score <= current {
                continue;
            }
            if spec.mount.is_some() && spec.affinity != "UNKNOWN" {
                continue;
            }
            match category {
                "BODY_PLAN" => {
                    spec.body_plan = value.into();
                    spec.confidences.insert("body_plan".into(), score);
                }
                "AFFINITY" => {
                    spec.affinity = value.into();
                    spec.confidences.insert("affinity".into(), score);
                }
                "MATERIAL" => {
                    if !spec.materials.iter().any(|m| m == value) {
                        spec.materials.insert(0, value.into());
                    }
                }
                "FEATURE" => {
                    if !spec.features.iter().any(|f| f == value) {
                        spec.features.push(value.into());
                    }
                }
                "ATTACK_ELEMENT" => {
                    spec.attack.element = value.into();
                    spec.confidences.insert("attack.element".into(), score);
                }
                "ATTACK_DELIVERY" => {
                    spec.attack.delivery = value.into();
                    spec.confidences.insert("attack.delivery".into(), score);
                }
                "ATTACK_CONCEPT" => {
                    if !["MELEE", "PROJECTILE"].contains(&spec.attack.concept.as_str()) {
                        continue;
                    }
                    let modern = [
                        "LASER",
                        "SHOTGUN",
                        "TNT",
                        "ROCKET",
                        "MISSILE",
                        "PLASMA",
                        "CYBERNETIC",
                        "MECHANICAL",
                        "RAILGUN",
                        "CANNON",
                    ];
                    if !modern.contains(&value) || prompt.to_uppercase().contains(value) {
                        spec.attack.concept = value.into();
                    }
                }
                _ => {}
            }
        }
        for (token, offset) in tokens.iter().zip(offsets) {
            if offset.0 == offset.1 {
                continue;
            }
            let mut best_token = (0.0, None);
            for ((label, weights), bias) in head
                .token_labels
                .iter()
                .zip(&head.token_weights)
                .zip(&head.token_biases)
            {
                let Some(score) = probability(weights, *bias, token) else {
                    return Err("token head hidden size mismatch".into());
                };
                if score > best_token.0 {
                    best_token = (score, Some(label));
                }
            }
            if best_token.0 >= head.threshold {
                spec.token_evidence.push(TokenEvidence {
                    label: best_token.1.unwrap().clone(),
                    start: offset.0,
                    end: offset.1,
                    confidence: best_token.0,
                });
            }
        }
        spec.confidence = spec
            .confidence
            .max(spec.confidences.values().copied().fold(0.0, f32::max));
        spec.parser = "candle-bert-head-v1".into();
    }
    let identity = spec.affinity.to_lowercase().replace('_', " ");
    for (slot, candidates) in [
        (
            "prefix",
            &["Ashen", "Dread", "Nether", "Infernal", "Fell", "Cursed"][..],
        ),
        ("noun", &["creature", "beast", "specimen", "thing"][..]),
        (
            "strike",
            &["strikes", "slashes", "swipes", "lashes out"][..],
        ),
    ] {
        let mut ranked = Vec::new();
        for word in candidates {
            let candidate = runtime.candidate(&format!("a {word} {identity}"))?;
            let dot: f32 = query.iter().zip(&candidate).map(|(a, b)| a * b).sum();
            let qa: f32 = query.iter().map(|v| v * v).sum::<f32>().sqrt();
            let ca: f32 = candidate.iter().map(|v| v * v).sum::<f32>().sqrt();
            ranked.push((dot / (qa * ca).max(1e-6), (*word).to_string()));
        }
        ranked.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        spec.description_synonyms.insert(
            slot.into(),
            ranked.into_iter().map(|(_, word)| word).collect(),
        );
    }
    Ok(spec)
}

#[cfg(all(test, feature = "bert"))]
mod bundled_model_tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn bundled_model_and_tokenizer_match() {
        let weights = include_bytes!("../models/bert-mini/model.safetensors");
        assert_eq!(
            format!("{:x}", Sha256::digest(weights)),
            "7fb69ad9f6866d8983183c930e33828f326470bf6ad8bbb2ad4ed957a92e9414"
        );
        let tokenizer: serde_json::Value =
            serde_json::from_slice(include_bytes!("../models/bert-mini/tokenizer.json")).unwrap();
        let vocab = tokenizer["model"]["vocab"].as_object().unwrap();
        let words = include_str!("../models/bert-mini/vocab.txt");
        assert_eq!(words.lines().count(), 30_522);
        assert_eq!(vocab.len(), 30_522);
        for (index, token) in words.lines().enumerate() {
            assert_eq!(vocab[token].as_u64(), Some(index as u64));
        }
    }

    #[test]
    fn unknown_prompt_keeps_low_confidence() {
        let spec = parse_with_embedded_bert("blorpt that hums ominously").unwrap();
        assert_eq!(spec.affinity, "UNKNOWN");
        assert!(spec.confidence < 0.5);
        assert_eq!(spec.description_synonyms["prefix"].len(), 6);
        assert_eq!(spec.description_synonyms["noun"].len(), 4);
    }
}
