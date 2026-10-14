//! Explicit V2 composition. Provenance travels in ordinary feature tags so V1
//! spec constructors and wire consumers retain their existing shape.
use crate::{
    anatomy::{Body, Node},
    parser::MonsterSpec,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpperBody {
    Humanoid,
    Stormtrooper,
    Harpy,
    Tree,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LowerBody {
    Humanoid,
    Horse,
    Spider,
    Mermaid,
    Tree,
    Serpent,
    Wolf,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HeadKind {
    Humanoid,
    Fish,
    Wolf,
    Bull,
    Bird,
    Spider,
    Stormtrooper,
    Tree,
    Cat,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HybridRecipe {
    pub schema_version: u32,
    pub upper_body: UpperBody,
    pub lower_body: LowerBody,
    pub head: HeadKind,
    pub lower_limb_count: u8,
    pub upper_arm_count: u8,
}
impl HybridRecipe {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("unsupported hybrid schema_version (expected 1)".into());
        }
        if self.lower_limb_count > 12 || self.upper_arm_count > 8 {
            return Err("hybrid supports at most 12 lower legs and 8 upper arms".into());
        }
        if matches!(self.lower_body, LowerBody::Mermaid | LowerBody::Serpent)
            && self.lower_limb_count != 0
        {
            return Err(
                "fish/serpent lower body requires zero legs; choose a legged lower body instead"
                    .into(),
            );
        }
        Ok(())
    }
}
fn token<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .unwrap()
        .as_str()
        .unwrap()
        .to_owned()
}
fn tag<'a>(spec: &'a MonsterSpec, prefix: &str) -> Option<&'a str> {
    spec.features.iter().find_map(|f| f.strip_prefix(prefix))
}
fn lower_identity(lower: LowerBody) -> (&'static str, &'static str) {
    match lower {
        LowerBody::Humanoid => ("HUMANOID", "HUMANOID"),
        LowerBody::Horse => ("QUADRUPED", "HORSE"),
        LowerBody::Spider => ("ARACHNID", "SPIDER"),
        LowerBody::Mermaid => ("HUMANOID", "MERMAID"),
        LowerBody::Tree => ("HUMANOID", "TREE_MONSTER"),
        LowerBody::Serpent => ("SERPENT", "SERPENT"),
        LowerBody::Wolf => ("QUADRUPED", "WOLF"),
    }
}
fn upper_identity(upper: UpperBody) -> (&'static str, &'static str) {
    match upper {
        UpperBody::Humanoid => ("HUMANOID", "HUMANOID"),
        UpperBody::Stormtrooper => ("HUMANOID", "STORMTROOPER"),
        UpperBody::Harpy => ("WINGED", "HARPY"),
        UpperBody::Tree => ("FUNGUS", "TREE_MONSTER"),
    }
}
fn head_identity(head: HeadKind) -> (&'static str, &'static str) {
    match head {
        HeadKind::Humanoid => ("HUMANOID", "HUMANOID"),
        HeadKind::Fish => ("SERPENT", "FISH"),
        HeadKind::Wolf => ("QUADRUPED", "WOLF"),
        HeadKind::Bull => ("QUADRUPED", "BULL"),
        HeadKind::Bird => ("WINGED", "BIRD"),
        HeadKind::Spider => ("ARACHNID", "SPIDER"),
        HeadKind::Stormtrooper => ("HUMANOID", "STORMTROOPER"),
        HeadKind::Tree => ("FUNGUS", "TREE_MONSTER"),
        HeadKind::Cat => ("QUADRUPED", "CAT"),
    }
}
pub fn apply_spec(recipe: &HybridRecipe, spec: &mut MonsterSpec) {
    let (plan, affinity) = lower_identity(recipe.lower_body);
    spec.body_plan = plan.into();
    spec.affinity = affinity.into();
    spec.limb_count = recipe.lower_limb_count;
    spec.arm_count = recipe.upper_arm_count;
    spec.secondary_affinities.clear();
    spec.mount = None;
    spec.features.retain(|f| {
        !f.starts_with("HYBRID_")
            && !matches!(
                f.as_str(),
                "ANCHORED"
                    | "FIN"
                    | "GILL"
                    | "WEBBED"
                    | "HOP"
                    | "CLIMB"
                    | "BURROW"
                    | "WING"
                    | "ROOTS"
                    | "FOUR_ARMED"
                    | "EXTRA_TAIL"
                    | "MUTATION_EXTRA_TAIL"
                    | "MUTANT_UNSTABLE"
            )
    });
    for f in [
        format!("HYBRID_VERSION_{}", recipe.schema_version),
        format!("HYBRID_UPPER_{}", token(recipe.upper_body)),
        format!("HYBRID_LOWER_{}", token(recipe.lower_body)),
        format!("HYBRID_HEAD_{}", token(recipe.head)),
        format!("HYBRID_LEGS_{}", recipe.lower_limb_count),
        format!("HYBRID_ARMS_{}", recipe.upper_arm_count),
        "EXPLICIT_ARMS".into(),
        "EXPLICIT_LEGS".into(),
        "EXPLICIT_TAILS".into(),
    ] {
        if !spec.features.contains(&f) {
            spec.features.push(f);
        }
    }
    spec.materials = vec![match recipe.lower_body {
        LowerBody::Horse | LowerBody::Wolf => "FUR",
        LowerBody::Spider => "CHITIN",
        LowerBody::Mermaid | LowerBody::Serpent => "SCALES",
        LowerBody::Tree => "WOOD",
        _ => "FLESH",
    }
    .into()];
    if recipe.lower_body == LowerBody::Mermaid && !spec.features.iter().any(|f| f == "FIN") {
        spec.features.push("FIN".into());
    }
    if recipe.lower_body == LowerBody::Tree {
        spec.features.push("WALKING_ROOTS".into());
    }
    if recipe.upper_body == UpperBody::Stormtrooper
        && !spec.features.iter().any(|f| f == "EXPLICIT_ATTACK")
    {
        spec.attack.concept = "LASER".into();
        spec.attack.delivery = "PROJECTILE".into();
        spec.attack.element = "ENERGY".into();
    }
}
pub fn from_spec(spec: &MonsterSpec) -> Option<HybridRecipe> {
    let decode = |prefix: &str| -> Option<serde_json::Value> {
        Some(serde_json::Value::String(tag(spec, prefix)?.to_owned()))
    };
    let recipe = HybridRecipe {
        schema_version: tag(spec, "HYBRID_VERSION_")?.parse().ok()?,
        upper_body: serde_json::from_value(decode("HYBRID_UPPER_")?).ok()?,
        lower_body: serde_json::from_value(decode("HYBRID_LOWER_")?).ok()?,
        head: serde_json::from_value(decode("HYBRID_HEAD_")?).ok()?,
        lower_limb_count: tag(spec, "HYBRID_LEGS_")?.parse().ok()?,
        upper_arm_count: tag(spec, "HYBRID_ARMS_")?.parse().ok()?,
    };
    recipe.validate().ok()?;
    Some(recipe)
}
/// Resolve only owned upper/head IDs; all other nodes keep the lower identity.
pub fn node_identity(spec: &MonsterSpec, id: &str) -> Option<(&'static str, &'static str)> {
    let recipe = from_spec(spec)?;
    if ["head_", "neck_", "eye_", "mouth_", "hair", "horn_", "fang_"]
        .iter()
        .any(|p| id.starts_with(p))
    {
        Some(head_identity(recipe.head))
    } else if id == "v2_upper_torso"
        || [
            "shoulder_",
            "arm_",
            "hand_",
            "weapon",
            "armor_",
            "belt",
            "collar",
            "wing_",
        ]
        .iter()
        .any(|p| id.starts_with(p))
    {
        Some(upper_identity(recipe.upper_body))
    } else {
        None
    }
}
fn count(words: &[&str], nouns: &[&str]) -> Result<Option<u8>, String> {
    for p in words.windows(2) {
        if !nouns.contains(&p[1]) {
            continue;
        }
        let value = match p[0] {
            "zero" | "no" => Some(0),
            "one" => Some(1),
            "two" => Some(2),
            "three" => Some(3),
            "four" => Some(4),
            "five" => Some(5),
            "six" => Some(6),
            "seven" => Some(7),
            "eight" => Some(8),
            "nine" => Some(9),
            "ten" => Some(10),
            "eleven" => Some(11),
            "twelve" => Some(12),
            "thirteen" => Some(13),
            "fourteen" => Some(14),
            "fifteen" => Some(15),
            "sixteen" => Some(16),
            "twenty" => Some(20),
            "hundred" => Some(100),
            v if v.chars().all(|c| c.is_ascii_digit()) => Some(
                v.parse::<u8>()
                    .map_err(|_| format!("hybrid {} count is out of range", p[1]))?,
            ),
            _ => None,
        };
        if value.is_some() {
            return Ok(value);
        }
    }
    Ok(None)
}
pub fn parse_prompt(prompt: &str) -> Option<HybridRecipe> {
    parse_prompt_checked(prompt).ok().flatten()
}

pub fn parse_prompt_checked(prompt: &str) -> Result<Option<HybridRecipe>, String> {
    let text = prompt.to_lowercase();
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let has = |w: &str| words.contains(&w);
    let centaur = has("centaur") || has("centaurs");
    let head_form = has("head") && (has("humanoid") || has("human"));
    let split_form = (has("top") || has("upper")) && (has("bottom") || has("lower"));
    let walking_tree = has("walking") && has("tree");
    if !centaur && !head_form && !split_form && !walking_tree {
        return Ok(None);
    }
    let upper = if has("stormtrooper") {
        UpperBody::Stormtrooper
    } else if has("harpy") {
        UpperBody::Harpy
    } else if walking_tree {
        UpperBody::Tree
    } else {
        UpperBody::Humanoid
    };
    let lower = if walking_tree {
        LowerBody::Tree
    } else if has("spider") && !head_form {
        LowerBody::Spider
    } else if has("mermaid") {
        LowerBody::Mermaid
    } else if has("serpent") || has("snake") {
        LowerBody::Serpent
    } else if centaur || has("horse") {
        LowerBody::Horse
    } else if has("wolf") && !head_form {
        LowerBody::Wolf
    } else {
        LowerBody::Humanoid
    };
    let head = if head_form {
        if has("fish") {
            HeadKind::Fish
        } else if has("wolf") {
            HeadKind::Wolf
        } else if has("bull") {
            HeadKind::Bull
        } else if has("bird") {
            HeadKind::Bird
        } else if has("spider") {
            HeadKind::Spider
        } else if has("cat") {
            HeadKind::Cat
        } else {
            HeadKind::Humanoid
        }
    } else {
        match upper {
            UpperBody::Stormtrooper => HeadKind::Stormtrooper,
            UpperBody::Tree => HeadKind::Tree,
            _ => HeadKind::Humanoid,
        }
    };
    let recipe = HybridRecipe {
        schema_version: 1,
        upper_body: upper,
        lower_body: lower,
        head,
        lower_limb_count: count(&words, &["legs", "legged", "limbs"])?.unwrap_or(match lower {
            LowerBody::Spider => 8,
            LowerBody::Horse | LowerBody::Wolf => 4,
            LowerBody::Mermaid | LowerBody::Serpent => 0,
            _ => 2,
        }),
        upper_arm_count: count(&words, &["arms", "armed"])?.unwrap_or(2),
    };
    recipe.validate()?;
    Ok(Some(recipe))
}

pub fn assemble(spec: &MonsterSpec, body: &mut Body, seed: u64) {
    let Some(recipe) = from_spec(spec) else {
        return;
    };
    let s = crate::anatomy::morphology_scale(spec.size);
    // Strip the lower creature's head and arms as entire attachment subtrees.
    let old = body.nodes.clone();
    let mut remap = vec![None; old.len()];
    let mut lower = Vec::new();
    for (i, n) in old.iter().enumerate() {
        if matches!(
            n.kind.as_str(),
            "NECK" | "HEAD" | "SHOULDER" | "WEAPON" | "WING"
        ) || n.parent.is_some_and(|p| remap[p].is_none())
        {
            continue;
        }
        let mut n = n.clone();
        n.parent = n.parent.and_then(|p| remap[p]);
        remap[i] = Some(lower.len());
        lower.push(n);
    }
    body.nodes = lower;
    if body.nodes.is_empty() {
        return;
    }
    // Distribute extra horse/wolf pairs longitudinally instead of overlapping
    // the legacy builder's repeated rear-leg anchor.
    if matches!(recipe.lower_body, LowerBody::Horse | LowerBody::Wolf) && spec.limb_count > 4 {
        let pairs = spec.limb_count.div_ceil(2);
        for leg in 0..spec.limb_count {
            if let Some(i) = body
                .nodes
                .iter()
                .position(|n| n.id == format!("leg_root_{leg}"))
            {
                let target = (6.0 - 14.0 * (leg / 2) as f32 / (pairs - 1) as f32) * s;
                let dx = target - body.nodes[i].x;
                let mut moved = vec![false; body.nodes.len()];
                for j in i..body.nodes.len() {
                    moved[j] = j == i || body.nodes[j].parent.is_some_and(|p| moved[p]);
                    if moved[j] {
                        body.nodes[j].x += dx;
                    }
                }
            }
        }
    }
    let mut upper = spec.clone();
    let (plan, affinity) = upper_identity(recipe.upper_body);
    upper.body_plan = plan.into();
    upper.affinity = affinity.into();
    upper.limb_count = 0;
    upper.arm_count = recipe.upper_arm_count;
    upper.features.retain(|f| {
        !f.starts_with("HYBRID_")
            && !matches!(
                f.as_str(),
                "FIN" | "GILL" | "ROOTS" | "ANCHORED" | "WALKING_ROOTS" | "SHADE" | "GHOST"
            )
    });
    upper.secondary_affinities.clear();
    upper.mount = None;
    upper.materials = vec![match recipe.upper_body {
        UpperBody::Stormtrooper => "ARMOR_WHITE",
        UpperBody::Harpy => "FEATHER_BROWN",
        UpperBody::Tree => "WOOD",
        _ => "FLESH",
    }
    .into()];
    let mut top = crate::anatomy::build(
        &upper,
        &mut ChaCha8Rng::seed_from_u64(seed ^ 0x485942524944),
    );
    super::anatomy::refine(&upper, &mut top, seed);
    let lower_torso = body.nodes[0].clone();
    let compact = matches!(
        recipe.lower_body,
        LowerBody::Humanoid | LowerBody::Mermaid | LowerBody::Tree
    );
    let dy = if compact {
        -4.0 * s
    } else {
        lower_torso.y - lower_torso.ry - 4.0 * s
    };
    let dx = if compact { 0.0 } else { 4.0 * s };
    if compact {
        body.nodes[0].ry *= 0.55;
        body.nodes[0].rx *= 0.8;
    }
    let mut map = vec![None; top.nodes.len()];
    for (i, n) in top.nodes.iter().enumerate() {
        if matches!(
            n.kind.as_str(),
            "HIP" | "LIMB" | "SHIN" | "FOOT" | "TAIL" | "SOLE" | "TREAD"
        ) || n.id.starts_with("root_")
            || n.parent.is_some_and(|p| map[p].is_none())
        {
            continue;
        }
        let mut n: Node = n.clone();
        if i == 0 {
            n.id = "v2_upper_torso".into();
            n.parent = Some(0);
        } else {
            n.parent = n.parent.and_then(|p| map[p]);
        }
        if body.nodes.iter().any(|old| old.id == n.id) {
            continue;
        }
        n.x += dx;
        n.y += dy;
        if matches!(n.kind.as_str(), "HEAD" | "NECK") {
            n.material = match recipe.head {
                HeadKind::Stormtrooper => "ARMOR_WHITE",
                HeadKind::Tree => "WOOD",
                HeadKind::Fish => "SCALES",
                HeadKind::Spider => "CHITIN",
                HeadKind::Wolf | HeadKind::Cat | HeadKind::Bull => "FUR",
                HeadKind::Bird => "FEATHER",
                _ => "FLESH",
            }
            .into();
            if n.kind == "HEAD" && recipe.head == HeadKind::Fish {
                n.rx *= 1.05;
                n.ry *= 0.9;
            }
        }
        map[i] = Some(body.nodes.len());
        body.nodes.push(n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn build(prompt: &str) -> (HybridRecipe, MonsterSpec, Body) {
        let recipe = parse_prompt(prompt).unwrap();
        let mut spec = crate::parser::parse_vocabulary(prompt);
        spec.features.push("PROFILE_V2".into());
        apply_spec(&recipe, &mut spec);
        super::super::anatomy::prepare_spec(&mut spec);
        super::super::anatomy::prepare_seeded(&mut spec, 42);
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(42));
        super::super::anatomy::refine(&spec, &mut body, 42);
        (recipe, spec, body)
    }
    #[test]
    fn explicit_compositions_roundtrip_and_own_their_parts() {
        for (prompt, lower, head, legs) in [
            (
                "six-legged centaur",
                LowerBody::Horse,
                HeadKind::Humanoid,
                6,
            ),
            ("spider-centaur", LowerBody::Spider, HeadKind::Humanoid, 8),
            (
                "fish head on humanoid legs",
                LowerBody::Humanoid,
                HeadKind::Fish,
                2,
            ),
            (
                "stormtrooper top on a mermaid bottom",
                LowerBody::Mermaid,
                HeadKind::Stormtrooper,
                0,
            ),
            ("walking tree", LowerBody::Tree, HeadKind::Tree, 2),
        ] {
            let (recipe, spec, body) = build(prompt);
            assert_eq!(recipe.lower_body, lower);
            assert_eq!(recipe.head, head);
            assert_eq!(recipe.lower_limb_count, legs);
            assert_eq!(from_spec(&spec), Some(recipe.clone()));
            assert_eq!(
                serde_json::from_str::<HybridRecipe>(&serde_json::to_string(&recipe).unwrap())
                    .unwrap(),
                recipe
            );
            assert_eq!(
                body.nodes.iter().filter(|n| n.kind == "FOOT").count(),
                legs as usize,
                "{prompt}"
            );
            assert_eq!(
                body.nodes.iter().filter(|n| n.kind == "HAND").count(),
                2,
                "{prompt}"
            );
            assert_eq!(
                body.nodes.iter().filter(|n| n.kind == "HEAD").count(),
                1,
                "{prompt}"
            );
            assert!(body.nodes.iter().any(|n| n.id == "v2_upper_torso"));
            assert_eq!(node_identity(&spec, "head_0"), Some(head_identity(head)));
            let mut ids = std::collections::HashSet::new();
            for (i, n) in body.nodes.iter().enumerate() {
                assert!(ids.insert(&n.id), "duplicate {} in {prompt}", n.id);
                assert!(n.parent.map(|p| p < i).unwrap_or(i == 0));
                assert!(n.x.is_finite() && n.y.is_finite() && n.rx > 0.0 && n.ry > 0.0);
            }
            let (_, modes, _, poses) = crate::animation::make(&spec, &body);
            assert_eq!(
                modes[0].category,
                if lower == LowerBody::Mermaid {
                    "SWIMMING"
                } else {
                    "GROUNDED"
                },
                "{prompt}"
            );
            assert!(poses.iter().all(|p| p
                .physical_positions
                .iter()
                .all(|(x, y, a)| x.is_finite() && y.is_finite() && a.is_finite())));
            if lower == LowerBody::Horse {
                let roots: Vec<_> = body
                    .nodes
                    .iter()
                    .filter(|n| n.id.starts_with("leg_root_"))
                    .map(|n| n.x.to_bits())
                    .collect();
                let unique: std::collections::HashSet<_> = roots.into_iter().collect();
                assert_eq!(unique.len(), 3);
            }
        }
    }
    #[test]
    fn structured_recipe_rejects_unknown_versions_fields_and_unbounded_counts() {
        let mut recipe = parse_prompt("spider-centaur").unwrap();
        recipe.schema_version = 2;
        assert!(recipe.validate().is_err());
        recipe.schema_version = 1;
        recipe.lower_limb_count = 13;
        assert!(recipe.validate().is_err());
        recipe.lower_limb_count = 8;
        recipe.upper_arm_count = 9;
        assert!(recipe.validate().is_err());
        assert!(serde_json::from_str::<HybridRecipe>(r#"{"schema_version":1,"upper_body":"HUMANOID","lower_body":"SPIDER","head":"HUMANOID","lower_limb_count":8,"upper_arm_count":2,"arbitrary":true}"#).is_err());
    }
    #[test]
    fn explicit_counts_and_attack_win_over_composition_defaults() {
        let (recipe, spec, body) = build("six-legged spider-centaur with four arms");
        assert_eq!((recipe.lower_limb_count, recipe.upper_arm_count), (6, 4));
        assert_eq!(body.nodes.iter().filter(|n| n.kind == "HAND").count(), 4);
        assert!(!spec.features.iter().any(|f| f == "MUTANT_UNSTABLE"));
        let recipe = parse_prompt("stormtrooper top on mermaid bottom").unwrap();
        let mut spec = crate::parser::parse_vocabulary("goblin spits acid");
        spec.features.push("EXPLICIT_ATTACK".into());
        let attack = spec.attack.clone();
        apply_spec(&recipe, &mut spec);
        assert_eq!(spec.attack.element, attack.element);
        assert_eq!(spec.attack.concept, attack.concept);
    }
    #[test]
    fn recognized_invalid_prompt_counts_never_fall_back() {
        for prompt in [
            "13-legged centaur",
            "999999999999-legged centaur",
            "thirteen legged centaur",
            "spider-centaur with nine arms",
            "humanoid top on mermaid bottom with two legs",
        ] {
            assert!(parse_prompt_checked(prompt).is_err(), "{prompt}");
        }
        assert_eq!(parse_prompt_checked("ordinary wolf").unwrap(), None);
    }
    #[test]
    fn shade_composition_has_one_tendril_set_and_no_gun() {
        let recipe = parse_prompt("stormtrooper top on mermaid bottom").unwrap();
        let mut spec = crate::parser::parse_vocabulary("stormtrooper");
        spec.features.extend(["PROFILE_V2".into(), "SHADE".into()]);
        apply_spec(&recipe, &mut spec);
        super::super::anatomy::prepare_spec(&mut spec);
        let mut body = crate::anatomy::build(&spec, &mut ChaCha8Rng::seed_from_u64(7));
        super::super::anatomy::refine(&spec, &mut body, 7);
        assert_eq!(
            body.nodes
                .iter()
                .filter(|n| n.id.starts_with("v2_shade_tendril_"))
                .count(),
            15
        );
        assert!(!body.nodes.iter().any(|n| n.kind == "WEAPON"));
    }
}
