//! Seeded, recipe-aware prompts for arena summoners and other NPCs.

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum PromptError {
    #[error("difficulty must be 1, 2, or 3")]
    InvalidDifficulty,
    #[error("arena round must start at 1")]
    InvalidRound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArenaPrompt {
    pub prompt: String,
    pub difficulty: u8,
    pub monster_seed: u64,
}

#[derive(Clone, Copy)]
struct Archetype {
    name: &'static str,
    humanoid: bool,
}

#[derive(Clone, Copy)]
enum AttackPhrase {
    Has(&'static str),
    Does(&'static str),
}

const EASY: &[Archetype] = &[
    Archetype {
        name: "goblin",
        humanoid: true,
    },
    Archetype {
        name: "wolf",
        humanoid: false,
    },
    Archetype {
        name: "rabbit",
        humanoid: false,
    },
    Archetype {
        name: "beetle",
        humanoid: false,
    },
    Archetype {
        name: "slug",
        humanoid: false,
    },
    Archetype {
        name: "spider",
        humanoid: false,
    },
    Archetype {
        name: "bat",
        humanoid: false,
    },
    Archetype {
        name: "skeleton",
        humanoid: true,
    },
    Archetype {
        name: "pig",
        humanoid: false,
    },
    Archetype {
        name: "mushroom",
        humanoid: false,
    },
    Archetype {
        name: "snake",
        humanoid: false,
    },
    Archetype {
        name: "hornet",
        humanoid: false,
    },
];
const MEDIUM: &[Archetype] = &[
    Archetype {
        name: "orc",
        humanoid: true,
    },
    Archetype {
        name: "troll",
        humanoid: true,
    },
    Archetype {
        name: "scorpion",
        humanoid: false,
    },
    Archetype {
        name: "lion",
        humanoid: false,
    },
    Archetype {
        name: "boar",
        humanoid: false,
    },
    Archetype {
        name: "cobra",
        humanoid: false,
    },
    Archetype {
        name: "hawk",
        humanoid: false,
    },
    Archetype {
        name: "centaur",
        humanoid: false,
    },
    Archetype {
        name: "minotaur",
        humanoid: true,
    },
    Archetype {
        name: "fish-man",
        humanoid: true,
    },
    Archetype {
        name: "raptor",
        humanoid: false,
    },
    Archetype {
        name: "bear",
        humanoid: false,
    },
];
const HARD: &[Archetype] = &[
    Archetype {
        name: "dragon",
        humanoid: false,
    },
    Archetype {
        name: "hydra",
        humanoid: false,
    },
    Archetype {
        name: "kraken",
        humanoid: false,
    },
    Archetype {
        name: "minotaur",
        humanoid: true,
    },
    Archetype {
        name: "giant",
        humanoid: true,
    },
    Archetype {
        name: "triceratops",
        humanoid: false,
    },
    Archetype {
        name: "mammoth",
        humanoid: false,
    },
    Archetype {
        name: "cerberus",
        humanoid: false,
    },
    Archetype {
        name: "sharknado",
        humanoid: false,
    },
    Archetype {
        name: "sand worm",
        humanoid: false,
    },
    Archetype {
        name: "wyvern",
        humanoid: false,
    },
    Archetype {
        name: "evil robot",
        humanoid: true,
    },
];

fn pool(difficulty: u8) -> Result<&'static [Archetype], PromptError> {
    match difficulty {
        1 => Ok(EASY),
        2 => Ok(MEDIUM),
        3 => Ok(HARD),
        _ => Err(PromptError::InvalidDifficulty),
    }
}

fn pick<'a, T>(rng: &mut ChaCha8Rng, values: &'a [T]) -> &'a T {
    &values[rng.random_range(0..values.len())]
}

fn compose(
    seed: u64,
    difficulty: u8,
    forced_archetype: Option<usize>,
) -> Result<String, PromptError> {
    let archetypes = pool(difficulty)?;
    let mut rng = ChaCha8Rng::seed_from_u64(seed ^ ((difficulty as u64) << 56) ^ 0x504f_4d50_5452);
    let archetype = archetypes[forced_archetype
        .unwrap_or_else(|| rng.random_range(0..archetypes.len()))
        % archetypes.len()];
    let size = match difficulty {
        1 => "small",
        2 => "",
        3 => "huge",
        _ => unreachable!(),
    };
    let style = pick(
        &mut rng,
        &["", "scarred", "crooked", "mottled", "ragged", "bristling"],
    );
    let creature = [size, style, archetype.name]
        .into_iter()
        .filter(|piece| !piece.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let article = if creature.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    let opening = pick(
        &mut rng,
        &[
            "",
            "Behold, ",
            "Let us try ",
            "Consider ",
            "I call forth ",
            "Perhaps we should face ",
        ],
    );
    let article = if opening.is_empty() {
        if article == "an" {
            "An"
        } else {
            "A"
        }
    } else {
        article
    };
    let mut prompt = format!("{opening}{article} {creature}");

    // Size, protection, and magical attacks make later waves reliably harder.
    if difficulty == 3 {
        let armor = pick(&mut rng, &["stone", "metal", "chitin", "scaly"]);
        prompt.push_str(&format!(" with {armor} armor"));
    }
    let attack = match difficulty {
        1 if archetype.humanoid => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("a club"),
                AttackPhrase::Has("a spear"),
                AttackPhrase::Has("claws"),
            ],
        ),
        1 => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("claws"),
                AttackPhrase::Has("fangs"),
                AttackPhrase::Has("horns"),
                AttackPhrase::Has("pustules"),
            ],
        ),
        2 if archetype.humanoid => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("an axe"),
                AttackPhrase::Has("a sword"),
                AttackPhrase::Has("a bow"),
                AttackPhrase::Does("breathes fire"),
            ],
        ),
        2 => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("fangs"),
                AttackPhrase::Has("claws"),
                AttackPhrase::Does("spits poison"),
                AttackPhrase::Does("breathes fire"),
            ],
        ),
        3 if archetype.humanoid => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("a huge fire axe"),
                AttackPhrase::Has("a lightning sword"),
                AttackPhrase::Has("laser eyes"),
                AttackPhrase::Has("a poison bow"),
            ],
        ),
        3 => *pick(
            &mut rng,
            &[
                AttackPhrase::Has("laser eyes"),
                AttackPhrase::Does("breathes fire"),
                AttackPhrase::Has("poison fangs"),
                AttackPhrase::Does("breathes lightning"),
            ],
        ),
        _ => unreachable!(),
    };
    match attack {
        AttackPhrase::Has(part) => {
            prompt.push_str(if difficulty == 3 { " and " } else { " with " });
            prompt.push_str(part);
        }
        AttackPhrase::Does(action) => {
            prompt.push_str(if difficulty == 3 {
                ", and it "
            } else {
                " that "
            });
            prompt.push_str(action);
        }
    }

    // A small amount of seeded noise makes combinations less formulaic while
    // retaining a known creature keyword and parser-visible anatomy.
    if rng.random_bool(match difficulty {
        1 => 0.25,
        2 => 0.5,
        _ => 0.7,
    }) {
        let detail = *pick(
            &mut rng,
            &["antlers", "extra eyes", "pustules", "a tail", "horns"],
        );
        if !matches!(attack, AttackPhrase::Has(part) if part == detail) {
            prompt.push_str(if matches!(attack, AttackPhrase::Does(_)) {
                " and has "
            } else {
                " and "
            });
            prompt.push_str(detail);
        }
    }
    let mounted = difficulty >= 2
        && archetype.humanoid
        && rng.random_bool(if difficulty == 3 { 0.28 } else { 0.12 });
    if mounted {
        prompt.push_str(" riding a ");
        prompt.push_str(pick(&mut rng, &["horse", "wolf", "boar", "spider"]));
    }
    if rng.random_bool(0.28) {
        prompt.push_str(pick(
            &mut rng,
            &[
                ", if you dare",
                ", I suppose",
                ", for reasons best left unexplained",
                ", just to see what happens",
            ],
        ));
    }
    prompt.push('.');
    if difficulty == 3 && !mounted && rng.random_bool(0.22) {
        prompt.push_str(" It summons ");
        prompt.push_str(pick(&mut rng, &["goblins", "spiders", "scarabs"]));
        prompt.push('.');
    }
    Ok(prompt)
}

/// Compose a deterministic prompt from parser-visible creature and trait words.
pub fn random_prompt(seed: u64, difficulty: u8) -> Result<String, PromptError> {
    compose(seed, difficulty, None)
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
    let prompt = compose(prompt_seed, difficulty, Some(archetype))?;
    let monster_seed = mix(prompt_seed ^ 0x4d4f_4e53_5445_52) & i64::MAX as u64;
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

    #[test]
    fn every_difficulty_emits_known_recipes_and_size_progression() {
        for archetype in EASY.iter().chain(MEDIUM).chain(HARD) {
            let spec = crate::parser::parse_vocabulary(archetype.name);
            assert_ne!(spec.affinity, "UNKNOWN", "{}", archetype.name);
        }
        for seed in 0..60 {
            let mut sizes = Vec::new();
            for difficulty in 1..=3 {
                let prompt = random_prompt(seed, difficulty).unwrap();
                assert_eq!(prompt, random_prompt(seed, difficulty).unwrap());
                let spec = crate::parser::parse_vocabulary(&prompt);
                assert_ne!(spec.affinity, "UNKNOWN", "{prompt}");
                if !prompt.contains("It summons") && spec.affinity != "SHARKNADO" {
                    assert!(spec.spawn.is_none(), "unexpected minion spawn: {prompt}");
                }
                sizes.push(spec.size);
                if difficulty == 3 {
                    assert!(
                        spec.materials
                            .iter()
                            .any(|material| ["STONE", "METAL", "CHITIN", "SCALES"]
                                .contains(&material.as_str())),
                        "{prompt}"
                    );
                    assert!(
                        spec.attack.element != "PHYSICAL" || spec.attack.delivery == "PROJECTILE",
                        "{prompt}"
                    );
                }
            }
            assert!(sizes[0] < sizes[1] && sizes[1] < sizes[2], "{sizes:?}");
        }
        assert_eq!(random_prompt(1, 0), Err(PromptError::InvalidDifficulty));
        assert_eq!(random_prompt(1, 4), Err(PromptError::InvalidDifficulty));
    }

    #[test]
    fn arena_rounds_progress_and_keep_speech_replayable() {
        assert_eq!(arena_prompt(1, 0), Err(PromptError::InvalidRound));
        let rounds: Vec<_> = (1..=9)
            .map(|round| arena_prompt(42, round).unwrap())
            .collect();
        assert_eq!(
            rounds
                .iter()
                .map(|wave| wave.difficulty)
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
        let seeds: std::collections::HashSet<_> =
            rounds.iter().map(|wave| wave.monster_seed).collect();
        assert_eq!(seeds.len(), rounds.len());
    }
}
