use crate::{anatomy::Body, parser::MonsterSpec, proto::Attack};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};

fn title(value: &str) -> String {
    let text = value.to_lowercase().replace('_', " ");
    text.split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn material_adjective(material: &str) -> &str {
    match material {
        "FLESH" => "fleshy",
        "FUR" | "FUR_DARK" | "FUR_WHITE" | "FUR_BROWN" => "furred",
        "FUR_GOLD" => "golden",
        "SCALES" => "scaled",
        "CHITIN" => "chitinous",
        "BONE" => "skeletal",
        "HORN" => "horned",
        "FEATHER" | "FEATHER_BLACK" | "FEATHER_BROWN" | "FEATHER_WHITE" | "FEATHER_GOLD" => {
            "feathered"
        }
        "LEAF" => "leafy",
        "GUNGAN_SKIN" => "amphibian-skinned",
        "DEEP_SEA" => "deep-sea",
        "ORCA_HIDE" => "black-and-white",
        "MEMBRANE" => "membranous",
        "SLIME" => "slimy",
        "SAND_HIDE" => "sandy",
        "STONE" => "stony",
        "METAL" => "metal",
        "WOOD" => "wooden",
        "WRAPPING" => "wrapped",
        "TUMOR" => "tumorous",
        "SPECTRAL" => "spectral",
        "ROTTEN" => "rotten",
        "FUNGUS" => "fungal",
        "ALIEN_SKIN" => "alien",
        "FIRE" => "fiery",
        "SMOKE" => "smoky",
        "ENERGY" => "luminous",
        _ => "strange",
    }
}

pub fn describe(spec: &MonsterSpec, body: &Body, attacks: &[Attack]) -> (String, String) {
    let name = if spec.affinity == "UNKNOWN" {
        "Unformed Horror".into()
    } else {
        let identity = if let Some(mount) = &spec.mount {
            let mut name = format!("{} Rider", title(&spec.affinity));
            let mut layer = Some(mount);
            while let Some(current) = layer {
                name.push_str(&format!(" on {}", title(&current.affinity)));
                layer = current.spec.as_ref().and_then(|child| child.mount.as_ref());
            }
            name
        } else if let Some(secondary) = spec.secondary_affinities.first() {
            format!("{}-{}", title(secondary), title(&spec.affinity))
        } else {
            title(&spec.affinity)
        };
        format!(
            "{} {}",
            if spec.size > 0.8 {
                "Giant"
            } else if spec.size < 0.35 {
                "Lesser"
            } else {
                "Infernal"
            },
            identity
        )
    };
    let heads = body.nodes.iter().filter(|n| n.kind == "HEAD").count();
    let feet = body.nodes.iter().filter(|n| n.kind == "FOOT").count();
    let hands = body.nodes.iter().filter(|n| n.kind == "HAND").count();
    let wings = body.nodes.iter().filter(|n| n.kind == "WING").count();
    let mut anatomy = Vec::new();
    anatomy.push(format!(
        "{heads} {}",
        if heads == 1 { "head" } else { "heads" }
    ));
    if feet > 0 {
        anatomy.push(format!("{feet} walking limbs"));
    }
    if hands > 0 {
        anatomy.push(format!("{hands} arms"));
    }
    if wings > 0 {
        anatomy.push(format!("{wings} wings"));
    }
    let attack_text = attacks.first().map_or_else(String::new, |attack| {
        let origin = attack
            .origin_node
            .split('_')
            .filter(|part| !part.chars().all(|c| c.is_ascii_digit()))
            .collect::<Vec<_>>()
            .join(" ");
        let element = if attack.element == "PHYSICAL" {
            String::new()
        } else {
            format!("{} ", attack.element.to_lowercase())
        };
        match attack.delivery.as_str() {
            "PROJECTILE" => format!("It launches {element}projectiles from its {origin}."),
            "MELEE" if element.is_empty() => format!("It strikes from its {origin}."),
            "MELEE" => format!("It strikes with {} from its {origin}.", element.trim()),
            "FIELD" => format!("It spreads a {element}field from its {origin}."),
            "SUMMON" => format!("It summons allies from its {origin}."),
            _ => format!("It attacks from its {origin}."),
        }
    });
    let support = if body.repairs.is_empty() {
        String::new()
    } else {
        format!(
            " Magical support: {}.",
            body.repairs
                .iter()
                .map(|r| r.to_lowercase().replace('_', " "))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let offspring = spec
        .spawn
        .as_ref()
        .map(|spawn| {
            let target = if spawn.target == "SELF" {
                "smaller self".to_string()
            } else {
                spawn.target.to_lowercase()
            };
            format!(
                " It can summon up to {} {} minions at once.",
                spawn.max_active, target
            )
        })
        .unwrap_or_default();
    let survivor = spec
        .mount
        .as_ref()
        .map(|mount| {
            format!(
                " When defeated, the {} continues fighting.",
                mount.survivor.to_lowercase()
            )
        })
        .unwrap_or_default();
    let mut description = format!(
        "A {} creature with {}.",
        material_adjective(&spec.materials[0]),
        anatomy.join(", ")
    );
    if !attack_text.is_empty() {
        description.push(' ');
        description.push_str(&attack_text);
    }
    description.push_str(&support);
    description.push_str(&offspring);
    description.push_str(&survivor);
    (name, description)
}

/// Adds optional bestiary commentary without consuming another generation stage's RNG.
/// The semantic specification and seed both influence the voice, so regenerating a
/// package gives the same text even if rendering internals change.
pub fn describe_seeded(
    spec: &MonsterSpec,
    body: &Body,
    attacks: &[Attack],
    seed: u64,
) -> (String, String) {
    let (mut name, mut description) = describe(spec, body, attacks);
    let mut hash = Sha256::new();
    hash.update(b"infernal.description.voice.v1");
    hash.update(seed.to_le_bytes());
    hash.update(serde_json::to_vec(spec).expect("monster specifications are serializable"));
    let mut rng = ChaCha8Rng::from_seed(hash.finalize().into());
    let choose = |slot: &str, fallback: &[&str], rng: &mut ChaCha8Rng| -> String {
        if let Some(ranked) = spec.description_synonyms.get(slot) {
            let limit = ranked.len().min(3);
            if limit > 0 {
                return ranked[rng.random_range(0..limit)].clone();
            }
        }
        fallback[rng.random_range(0..fallback.len())].into()
    };
    if let Some(identity) = name.strip_prefix("Infernal ") {
        let prefix = choose(
            "prefix",
            &["Ashen", "Dread", "Nether", "Fell", "Cursed", "Infernal"],
            &mut rng,
        );
        name = format!("{prefix} {identity}");
    }
    let noun = choose(
        "noun",
        &["creature", "beast", "specimen", "thing"],
        &mut rng,
    );
    description = description.replacen(" creature with ", &format!(" {noun} with "), 1);
    if description.contains("It strikes") {
        let strike = choose(
            "strike",
            &["strikes", "slashes", "swipes", "lashes out"],
            &mut rng,
        );
        description = description.replacen("It strikes", &format!("It {strike}"), 1);
    }

    if rng.random_range(0..100) < 62 {
        let affinity = spec.affinity.to_lowercase().replace('_', " ");
        let material = spec
            .materials
            .first()
            .map(|m| m.to_lowercase().replace('_', " "))
            .unwrap_or_else(|| "unknown".into());
        let mut comments = vec![
            "Ugly mess of a beast.".to_string(),
            "Really weird.".to_string(),
            "Finest slop from Hades yet.".to_string(),
            "Hades, of course, calls this art.".to_string(),
            "Even Ozzy Osbourne would be scared to bite this one.".to_string(),
            "An abomination, surely.".to_string(),
            "Won the local ugly awards and tried to go national.".to_string(),
            "Like the thoughts of a psychopath on LSD, but in vivid color and now in 3D."
                .to_string(),
            "In other words, a small claims lawyer.".to_string(),
            format!("The {affinity} committee denies any connection to this specimen."),
            format!("Someone gave {material} a bad idea and it learned to walk."),
        ];
        if spec.is_blob() || spec.is_gastropod() || material.contains("rotten") {
            comments.extend([
                "Like the theoretical child of pancreatic cancer and a rotten milkshake."
                    .to_string(),
                format!("The {affinity} appears to have been poured rather than born."),
                "Even the puddle wants its dignity back.".to_string(),
            ]);
        }
        if !spec.secondary_affinities.is_empty() || spec.heads > 1 || spec.arm_count > 2 {
            comments.extend([
                "Several bad ideas held a meeting and elected this as chair.".to_string(),
                "Every extra part seems to have filed a separate complaint.".to_string(),
            ]);
        }
        if spec.mount.is_some() {
            comments.push("Neither half admits to knowing the other.".to_string());
        }
        let comment = &comments[rng.random_range(0..comments.len())];
        description.push(' ');
        description.push_str(comment);
    }

    if rng.random_range(0..100) < 43 {
        let mut directives = vec![
            "Kill on sight.".to_string(),
            "Invasive species: legal to kill in all jurisdictions of Hell.".to_string(),
            "Exterminate if found.".to_string(),
            "Illegal to transport to new regions.".to_string(),
            "Biohazard: dispose with care.".to_string(),
            "Erase, shred, then burn.".to_string(),
        ];
        if spec.spawn.is_some() {
            directives.extend([
                "Do not allow it to establish a breeding population.".to_string(),
                "Destroy any offspring before leaving the area.".to_string(),
            ]);
        }
        if spec.attack.element == "FIRE" {
            directives.push("Keep away from dry stores and open flame.".to_string());
        }
        if spec.affinity == "GHOST" || spec.affinity == "PHANTOM" {
            directives.push("Do not assume a locked door will contain it.".to_string());
        }
        let directive = &directives[rng.random_range(0..directives.len())];
        description.push(' ');
        description.push_str(directive);
    }
    (name, description)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn voice_is_seeded_optional_and_uses_creature_data() {
        let spec = crate::parser::parse_vocabulary("sludge slug that spawns smaller slugs");
        let body = Body {
            nodes: vec![],
            mass: 1.0,
            com: (0.0, 0.0),
            gravity: 1.0,
            repairs: vec![],
        };
        let base = describe(&spec, &body, &[]).1;
        let mut outputs = BTreeSet::new();
        let mut plain = false;
        let mut voiced = false;
        for seed in 0..128 {
            let first = describe_seeded(&spec, &body, &[], seed);
            assert_eq!(first, describe_seeded(&spec, &body, &[], seed));
            assert!(first.1.contains("with 0 heads."));
            let sentence_count = first.1.matches('.').count();
            plain |= sentence_count == base.matches('.').count();
            voiced |= sentence_count > base.matches('.').count();
            outputs.insert(first.1);
        }
        assert!(plain && voiced);
        assert!(outputs.len() > 10);
    }
}
