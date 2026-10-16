//! Seeded, recipe-aware prompts shared by all clients and summoner NPCs.

use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PromptError {
    #[error("difficulty must be 1, 2, or 3")]
    InvalidDifficulty,
    #[error("arena round must start at 1")]
    InvalidRound,
    #[error("prompt profile must be default, necromancer, shaman, lunatic, boss, or ocean")]
    InvalidProfile,
}

/// A summoner's vocabulary policy, independent of the V1/V2 generation profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum PromptProfile {
    #[default]
    Default = 0,
    Necromancer = 1,
    Shaman = 2,
    Lunatic = 3,
    Boss = 4,
    Ocean = 5,
}
impl PromptProfile {
    pub const ALL: [Self; 6] = [
        Self::Default,
        Self::Necromancer,
        Self::Shaman,
        Self::Lunatic,
        Self::Boss,
        Self::Ocean,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Necromancer => "necromancer",
            Self::Shaman => "shaman",
            Self::Lunatic => "lunatic",
            Self::Boss => "boss",
            Self::Ocean => "ocean",
        }
    }
}
impl std::str::FromStr for PromptProfile {
    type Err = PromptError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|p| p.as_str().eq_ignore_ascii_case(value))
            .ok_or(PromptError::InvalidProfile)
    }
}
impl TryFrom<u32> for PromptProfile {
    type Error = PromptError;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::ALL
            .get(value as usize)
            .copied()
            .ok_or(PromptError::InvalidProfile)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaPrompt {
    pub prompt: String,
    pub difficulty: u8,
    pub monster_seed: u64,
}

// Sample families first, then one distinct recipe within that family. Animals
// cannot crowd out insects, plants, sea life or folklore just because there are
// more animal names. Difficulty changes threat modifiers, not recipe availability.
// These are canonical shared-parser phrases, not alias duplicates or prose noise.
const FAMILIES: &[&[&str]] = &[
    &[
        "goblin", "orc", "troll", "cyclops", "minotaur", "satyr", "demon", "giant", "warrior",
        "knight", "archer",
    ],
    &[
        "wolf", "bear", "lion", "tiger", "leopard", "cheetah", "panther", "hyena", "rabbit", "dog",
        "boar", "pig",
    ],
    &[
        "elephant", "rhino", "hippo", "giraffe", "buffalo", "moose", "antelope", "gazelle",
        "horse", "zebra", "bull", "ram", "goat", "gorilla",
    ],
    &[
        "bat",
        "hawk",
        "owl",
        "eagle",
        "crow",
        "condor",
        "heron",
        "pelican",
        "albatross",
        "hummingbird",
        "ostrich",
        "vulture",
        "ibis",
        "pigeon",
        "sparrow",
    ],
    &[
        "spider",
        "scorpion",
        "centipede",
        "beetle",
        "hornet",
        "scarab",
        "snail",
        "slug",
        "crab",
    ],
    OCEAN,
    &[
        "dragon",
        "wyvern",
        "hydra",
        "cerberus",
        "griffin",
        "manticore",
        "chimera",
        "sphinx",
        "centaur",
        "pegasus",
        "harpy",
        "siren",
    ],
    &[
        "ammit",
        "apophis",
        "bennu",
        "serpopard",
        "naga",
        "asura",
        "yaksha",
        "makara",
        "vanara",
        "jormungandr",
        "nidhogg",
        "kelpie",
        "pooka",
        "fomorian",
        "balor",
        "grootslang",
        "quetzalcoatl",
        "ahuizotl",
    ],
    &[
        "skeleton", "zombie", "ghost", "phantom", "draugr", "banshee", "lemure", "vetala",
        "dullahan", "djinn",
    ],
    &[
        "evil robot",
        "stormtrooper",
        "tank",
        "drone",
        "alien",
        "mushroom",
        "tree monster",
        "slime",
        "ooze",
        "blob",
    ],
    &[
        "tyrannosaurus rex",
        "raptor",
        "stegosaurus",
        "triceratops",
        "ankylosaurus",
        "brontosaurus",
        "mammoth",
        "smilodon",
    ],
    &[
        "snake",
        "cobra",
        "serpent",
        "crocodile",
        "sand worm",
        "chupacabra",
        "bigfoot",
        "yeti",
    ],
];
const OCEAN: &[&str] = &[
    "fish",
    "eel",
    "shark",
    "whale",
    "orca",
    "jellyfish",
    "octopus",
    "kraken",
    "sea serpent",
    "sea dragon",
    "mermaid",
    "merman",
    "fish man",
];
const SHAMAN_BEASTS: &[&str] = &[
    "bear",
    "boar",
    "wolf",
    "lion",
    "tiger",
    "buffalo",
    "rhino",
    "elephant",
    "mammoth",
    "gorilla",
    "moose",
    "bull",
    "crocodile",
];
const UNDEAD: &[&str] = &["skeletal", "zombie", "ghost"];
const MATERIALS: &[&str] = &[
    "stone",
    "metal",
    "chitin",
    "scaly",
    "wood",
    "fungal",
    "slimy",
    "feathered",
    "fur",
    "skeletal",
    "spectral",
];
const FEATURES: &[&str] = &[
    "horns",
    "fangs",
    "claws",
    "wings",
    "a tail",
    "extra eyes",
    "pustules",
    "tentacles",
    "antlers",
    "tusks",
    "a shell",
    "spores",
];
const MOVEMENTS: &[&str] = &["burrowing", "climbing", "hopping", "limping"];
const ELEMENTS: &[&str] = &[
    "fire",
    "ice",
    "poison",
    "lightning",
    "acid",
    "shadow",
    "plasma",
];

fn pick<'a, T>(rng: &mut ChaCha8Rng, values: &'a [T]) -> &'a T {
    &values[rng.random_range(0..values.len())]
}
fn broad_base(rng: &mut ChaCha8Rng) -> &'static str {
    let family = pick(rng, FAMILIES);
    pick(rng, family)
}

fn compose(
    seed: u64,
    difficulty: u8,
    profile: PromptProfile,
    forced_base: Option<usize>,
) -> Result<String, PromptError> {
    if !(1..=3).contains(&difficulty) {
        return Err(PromptError::InvalidDifficulty);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(
        seed ^ ((difficulty as u64) << 56) ^ ((profile as u64) << 48) ^ 0x504f_4d50_5452,
    );
    if profile == PromptProfile::Lunatic {
        return Ok(lunatic(&mut rng));
    }
    let difficulty = if profile == PromptProfile::Boss {
        3
    } else {
        difficulty
    };
    let base = if let Some(index) = forced_base {
        let count = FAMILIES.iter().map(|family| family.len()).sum::<usize>();
        *FAMILIES
            .iter()
            .flat_map(|family| family.iter())
            .nth(index % count)
            .unwrap()
    } else if profile == PromptProfile::Ocean {
        *pick(&mut rng, OCEAN)
    } else if profile == PromptProfile::Shaman {
        if rng.random_bool(0.45) {
            *pick(&mut rng, &["orc", "troll"])
        } else {
            *pick(&mut rng, SHAMAN_BEASTS)
        }
    } else {
        broad_base(&mut rng)
    };
    // V2 centaur recipes intentionally replace material/topology; use another
    // base for bosses so their armor contract cannot be overwritten.
    let base = if profile == PromptProfile::Boss && base == "centaur" {
        "minotaur"
    } else {
        base
    };
    let mut subject = base.to_owned();
    // Ordinary blended species are understood by both parsers via secondary
    // affinities. No unsupported structured-hybrid parts are invented.
    let hybrid_chance = if profile == PromptProfile::Shaman {
        0.65
    } else {
        0.22
    };
    if profile != PromptProfile::Ocean && rng.random_bool(hybrid_chance) {
        let other = if profile == PromptProfile::Shaman {
            *pick(&mut rng, SHAMAN_BEASTS)
        } else {
            broad_base(&mut rng)
        };
        let other = if profile == PromptProfile::Boss && other == "centaur" {
            "bull"
        } else {
            other
        };
        if other != base {
            subject.push_str(&format!(" {other} hybrid"));
        }
    }
    let size = if profile == PromptProfile::Shaman {
        "large"
    } else {
        match difficulty {
            1 => "small",
            2 => "",
            _ => "huge",
        }
    };
    let mutation = if profile == PromptProfile::Necromancer {
        *pick(&mut rng, UNDEAD)
    } else if profile == PromptProfile::Boss {
        "armored"
    } else {
        ""
    };
    // Stormtrooper recipes own their armor palette in V1; skeletal/zombie
    // feature tags survive that override, whereas spectral material does not.
    let mutation = if mutation == "ghost" && subject.contains("stormtrooper") {
        "skeletal"
    } else {
        mutation
    };
    // Size is directly adjacent to the subject so it cannot accidentally bind
    // to a later horn, weapon or head instead of the creature.
    let mut words = vec![mutation, size, &subject];
    words.retain(|word| !word.is_empty());
    let mut prompt = words.join(" ");
    if difficulty == 3 {
        prompt.push_str(&format!(
            " with {} armor",
            pick(&mut rng, &["stone", "metal", "chitin"])
        ));
    } else if rng.random_bool(0.6) {
        prompt.push_str(&format!(" with {} hide", pick(&mut rng, MATERIALS)));
    }
    let humanoid = crate::parser::parse_vocabulary(base).body_plan == "HUMANOID";
    if difficulty == 3 || (difficulty == 2 && rng.random_bool(0.6)) {
        let element = pick(&mut rng, ELEMENTS);
        if humanoid && rng.random_bool(0.4) && profile != PromptProfile::Boss {
            prompt.push_str(&format!(
                " wielding a {element} {}",
                pick(&mut rng, &["sword", "axe", "spear", "bow"])
            ));
        } else {
            prompt.push_str(&format!(
                " that {} {element}",
                pick(&mut rng, &["shoots", "spits", "breathes"])
            ));
        }
    } else if humanoid {
        let weapon = pick(&mut rng, &["club", "spear", "sword", "axe", "bow"]);
        let article = if *weapon == "axe" { "an" } else { "a" };
        prompt.push_str(&format!(" wielding {article} {weapon}"));
    } else {
        prompt.push_str(&format!(
            " with {}",
            pick(&mut rng, &["claws", "fangs", "horns", "tentacles"])
        ));
    }
    prompt.push_str(&format!(" and {}", pick(&mut rng, FEATURES)));
    if profile == PromptProfile::Boss || rng.random_bool(0.25) {
        let count = if humanoid && rng.random_bool(0.5) {
            *pick(&mut rng, &["four arms", "six arms", "eight arms"])
        } else {
            *pick(&mut rng, &["two heads", "three heads", "two tails"])
        };
        prompt.push_str(&format!(" and {count}"));
    }
    if profile != PromptProfile::Ocean && rng.random_bool(0.2) {
        prompt.push_str(&format!(", {}", pick(&mut rng, MOVEMENTS)));
    }
    // Explicit marine recipes already expose SWIM in both V1 and V2. Never add
    // a land mount or an anchored modifier that would negate that capability.
    if profile == PromptProfile::Ocean {
        prompt.push_str(", swimming in water");
    }
    if profile != PromptProfile::Ocean && humanoid && rng.random_bool(0.12) {
        prompt.push_str(" riding a ");
        if profile == PromptProfile::Necromancer {
            prompt.push_str("skeletal ");
        }
        prompt.push_str(pick(&mut rng, SHAMAN_BEASTS));
    }
    prompt.push('.');
    Ok(prompt)
}

fn lunatic(rng: &mut ChaCha8Rng) -> String {
    // Shuffle meaningful phrase-sized units: splitting 'sea dragon' or 'six
    // arms' into isolated words would destroy parser intent rather than expand
    // the bestiary. Every category is sampled independently and cross-family.
    let mut chunks = vec![
        broad_base(rng).to_owned(),
        broad_base(rng).to_owned(),
        broad_base(rng).to_owned(),
        pick(rng, MATERIALS).to_string(),
        pick(rng, MATERIALS).to_string(),
        pick(rng, FEATURES).to_string(),
        pick(rng, FEATURES).to_string(),
        pick(rng, MOVEMENTS).to_string(),
        pick(rng, &["tiny", "small", "large", "huge", "colossal", "cute"]).to_string(),
        pick(
            rng,
            &[
                "two heads",
                "three heads",
                "four arms",
                "six arms",
                "eight arms",
                "two tails",
            ],
        )
        .to_string(),
        format!(
            "{} {}",
            pick(rng, &["shoots", "spits", "breathes"]),
            pick(rng, ELEMENTS)
        ),
    ];
    chunks.shuffle(rng);
    format!("{}.", chunks.join(" "))
}

/// Compose a deterministic prompt with balanced coverage across recipe families.
/// This is equivalent to selecting PromptProfile::Default explicitly.
pub fn random_prompt(seed: u64, difficulty: u8) -> Result<String, PromptError> {
    random_prompt_with_profile(seed, difficulty, PromptProfile::Default)
}

/// Named summoner profile. Boss always adds hard-tier armor and elemental ranged
/// attacks; other profiles retain the selected difficulty's size/attack budget.
pub fn random_prompt_with_profile(
    seed: u64,
    difficulty: u8,
    profile: PromptProfile,
) -> Result<String, PromptError> {
    compose(seed, difficulty, profile, None)
}

/// A one-based arena round. Difficulty rises after rounds 3 and 6, then stays at 3.
/// Adjacent rounds rotate their base archetypes even if their other words vary.
pub fn arena_prompt(run_seed: u64, round: u32) -> Result<ArenaPrompt, PromptError> {
    if round == 0 {
        return Err(PromptError::InvalidRound);
    }
    let difficulty = (1 + (round - 1) / 3).min(3) as u8;
    let prompt_seed = mix(run_seed ^ (round as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    let archetype = (mix(run_seed) as usize).wrapping_add(round as usize - 1);
    let prompt = compose(
        prompt_seed,
        difficulty,
        PromptProfile::Default,
        Some(archetype),
    )?;
    let monster_seed = mix(prompt_seed ^ 0x004d_4f4e_5354_4552) & i64::MAX as u64;
    Ok(ArenaPrompt {
        prompt,
        difficulty,
        monster_seed,
    })
}

fn mix(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeSet, HashSet};
    fn specs(prompt: &str) -> [crate::parser::MonsterSpec; 2] {
        let v1 = crate::parser::parse_vocabulary(prompt);
        [v1.clone(), crate::v2::interpret(v1, prompt)]
    }
    #[test]
    fn profiles_validate_and_replay_full_width_seeds() {
        for profile in PromptProfile::ALL {
            assert_eq!(profile.as_str().parse(), Ok(profile));
            assert_eq!(profile.as_str().to_uppercase().parse(), Ok(profile));
            assert_eq!(PromptProfile::try_from(profile as u32), Ok(profile));
            for seed in [0, 1, 42, i64::MAX as u64, u64::MAX] {
                for difficulty in 1..=3 {
                    let p = random_prompt_with_profile(seed, difficulty, profile).unwrap();
                    assert_eq!(
                        p,
                        random_prompt_with_profile(seed, difficulty, profile).unwrap()
                    );
                    assert!(p.len() < 500, "Godot input budget: {p}");
                    for spec in specs(&p) {
                        assert_ne!(spec.affinity, "UNKNOWN", "{p}");
                    }
                }
            }
        }
        for value in ["", "v2", "wizard", "Ocean "] {
            assert!(value.parse::<PromptProfile>().is_err());
        }
        assert!(PromptProfile::try_from(6).is_err());
        assert!(PromptProfile::try_from(u32::MAX).is_err());
        for d in [0, 4, u8::MAX] {
            assert_eq!(random_prompt(42, d), Err(PromptError::InvalidDifficulty));
        }
        assert_eq!(
            random_prompt(42, 2),
            random_prompt_with_profile(42, 2, PromptProfile::Default)
        );
    }
    #[test]
    fn default_samples_all_families_and_meaningful_modifiers_at_every_difficulty() {
        for family in FAMILIES {
            for name in *family {
                assert_ne!(
                    crate::parser::parse_vocabulary(name).affinity,
                    "UNKNOWN",
                    "{name}"
                );
            }
        }
        for difficulty in 1..=3 {
            let mut names = HashSet::new();
            let mut plans = HashSet::new();
            let mut materials = HashSet::new();
            let mut features = HashSet::new();
            let mut prompts = HashSet::new();
            let mut family_hits = vec![0; FAMILIES.len()];
            for seed in 0..2048 {
                let p = random_prompt(seed, difficulty).unwrap();
                let spec = crate::parser::parse_vocabulary(&p);
                names.insert(spec.affinity.clone());
                plans.insert(spec.body_plan.clone());
                materials.extend(spec.materials);
                features.extend(spec.features);
                for (i, family) in FAMILIES.iter().enumerate() {
                    if family
                        .iter()
                        .any(|name| crate::parser::parse_vocabulary(name).affinity == spec.affinity)
                    {
                        family_hits[i] += 1;
                    }
                }
                prompts.insert(p);
            }
            assert!(names.len() >= 130, "{difficulty}: {} recipes", names.len());
            assert!(plans.len() >= 11, "{plans:?}");
            assert!(
                materials.len() >= if difficulty == 3 { 8 } else { 12 },
                "{materials:?}"
            );
            assert!(features.len() >= 25, "{features:?}");
            assert!(prompts.len() >= 2030);
            // Broad distribution, not one favored recipe with cosmetic words.
            assert!(
                family_hits.iter().all(|count| *count >= 60 && *count < 350),
                "{family_hits:?}"
            );
        }
    }
    #[test]
    fn every_profile_keeps_parser_visible_constraints_across_many_seeds() {
        for profile in PromptProfile::ALL {
            let mut recipes = BTreeSet::new();
            let mut hybrids = 0;
            for seed in 0..512 {
                let p = random_prompt_with_profile(seed, (seed % 3 + 1) as u8, profile).unwrap();
                assert!(crate::v2::hybrid::parse_prompt_checked(&p).is_ok(), "{p}");
                for spec in specs(&p) {
                    assert_ne!(spec.affinity, "UNKNOWN", "{p}");
                    recipes.insert(spec.affinity.clone());
                    if !spec.secondary_affinities.is_empty() {
                        hybrids += 1;
                    }
                    match profile {
                        PromptProfile::Necromancer => {
                            let traits = crate::v2::damage_traits::describe(&spec);
                            assert!(
                                traits
                                    .iter()
                                    .any(|t| ["SKELETAL", "UNDEAD", "SPECTRAL"]
                                        .contains(&t.as_str())),
                                "{p}: {traits:?}"
                            );
                        }
                        PromptProfile::Shaman => {
                            let allowed: HashSet<_> = SHAMAN_BEASTS
                                .iter()
                                .chain(["orc", "troll"].iter())
                                .map(|n| crate::parser::parse_vocabulary(n).affinity)
                                .collect();
                            assert!(allowed.contains(&spec.affinity), "{p}: {}", spec.affinity);
                            assert!(
                                spec.secondary_affinities
                                    .iter()
                                    .all(|a| allowed.contains(a)),
                                "{p}"
                            );
                            assert!(spec.size >= 0.75, "{p}");
                        }
                        PromptProfile::Boss => {
                            assert!(spec.bulk >= 0.8, "{p}");
                            assert!(
                                spec.materials.iter().any(|m| [
                                    "STONE",
                                    "METAL",
                                    "CHITIN",
                                    "ARMOR_WHITE"
                                ]
                                .contains(&m.as_str())),
                                "{p}: {:?}",
                                spec.materials
                            );
                            assert_ne!(spec.attack.element, "PHYSICAL", "{p}");
                            assert_eq!(spec.attack.delivery, "PROJECTILE", "{p}");
                        }
                        PromptProfile::Ocean => {
                            assert!(crate::locomotion::aquatic(&spec), "{p}");
                            assert!(spec.mount.is_none(), "{p}");
                            let body =
                                crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(seed));
                            let (_, modes, _, _) = crate::animation::make(&spec, &body);
                            assert!(modes.iter().any(|m| m.id == "SWIM"), "{p}");
                        }
                        _ => {}
                    }
                }
            }
            if [
                PromptProfile::Default,
                PromptProfile::Necromancer,
                PromptProfile::Lunatic,
                PromptProfile::Boss,
            ]
            .contains(&profile)
            {
                assert!(recipes.len() >= 100, "{profile:?}: {}", recipes.len());
            }
            if profile == PromptProfile::Shaman {
                assert!(
                    recipes.len() >= 15 && hybrids > 400,
                    "{recipes:?}, {hybrids}"
                );
            }
            if profile == PromptProfile::Ocean {
                assert_eq!(recipes.len(), 13);
            }
        }
    }
    #[test]
    fn arena_rounds_progress_and_keep_speech_replayable() {
        assert_eq!(arena_prompt(1, 0), Err(PromptError::InvalidRound));
        let rounds: Vec<_> = (1..=40).map(|r| arena_prompt(42, r).unwrap()).collect();
        assert_eq!(
            rounds
                .iter()
                .take(9)
                .map(|w| w.difficulty)
                .collect::<Vec<_>>(),
            [1, 1, 1, 2, 2, 2, 3, 3, 3]
        );
        for (round, wave) in rounds.iter().enumerate() {
            assert_eq!(*wave, arena_prompt(42, round as u32 + 1).unwrap());
            assert!(wave.monster_seed <= i64::MAX as u64);
        }
        assert!(rounds
            .windows(2)
            .all(|pair| pair[0].prompt != pair[1].prompt));
        assert_eq!(
            rounds
                .iter()
                .map(|w| w.monster_seed)
                .collect::<HashSet<_>>()
                .len(),
            rounds.len()
        );
    }
}
