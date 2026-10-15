//! Declarative defense categories only. The engine owns numerical balancing.
use crate::parser::MonsterSpec;

pub const ALLOWED: &[&str] = &[
    "SKELETAL",
    "GELATINOUS",
    "ARMORED",
    "UNDEAD",
    "WOODEN",
    "SPECTRAL",
    "FIRE_ATTUNED",
];

pub fn describe(spec: &MonsterSpec) -> Vec<String> {
    let feature = |names: &[&str]| spec.features.iter().any(|v| names.contains(&v.as_str()));
    let material = |names: &[&str]| spec.materials.iter().any(|v| names.contains(&v.as_str()));
    let mut result = Vec::new();
    if spec.affinity == "SKELETON" || feature(&["SKELETAL"]) {
        result.push("SKELETAL".into());
    }
    if ["SLIME", "OOZE", "BLOB"].contains(&spec.affinity.as_str())
        || feature(&["SPLITTING_SLIME"])
        || material(&["SLIME"])
    {
        result.push("GELATINOUS".into());
    }
    if feature(&["ARMORED", "ARMOR", "SHIELD_SENTINEL"]) || material(&["METAL", "STONE", "CHITIN"])
    {
        result.push("ARMORED".into());
    }
    if spec.affinity == "ZOMBIE" || feature(&["UNDEAD", "ZOMBIFIED"]) {
        result.push("UNDEAD".into());
    }
    if spec.affinity == "TREE_MONSTER" || material(&["WOOD"]) {
        result.push("WOODEN".into());
    }
    if ["SPECTRAL", "PHANTOM"].contains(&spec.affinity.as_str())
        || material(&["SPECTRAL"])
        || feature(&["GHOST", "SHADE"])
    {
        result.push("SPECTRAL".into());
    }
    if feature(&["FLAMING"]) || material(&["FIRE", "LAVA"]) {
        result.push("FIRE_ATTUNED".into());
    }
    result
}

pub fn validate(values: &[String]) -> Result<(), String> {
    if values.len() > ALLOWED.len() {
        return Err("too many damage traits".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for value in values {
        if !ALLOWED.contains(&value.as_str()) || !seen.insert(value) {
            return Err(format!("unknown or duplicated damage trait: {value}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_vocabulary;
    #[test]
    fn stable_categories_without_numerical_defenses() {
        for (prompt, expected) in [
            ("skeleton", "SKELETAL"),
            ("slime", "GELATINOUS"),
            ("zombie", "UNDEAD"),
            ("metal robot", "ARMORED"),
            ("wood tree monster", "WOODEN"),
            ("ghost", "SPECTRAL"),
        ] {
            let spec = parse_vocabulary(prompt);
            let traits = describe(&spec);
            assert!(traits.iter().any(|s| s == expected), "{prompt}: {traits:?}");
            assert_eq!(traits, describe(&spec));
            validate(&traits).unwrap();
        }
        assert!(validate(&["SKELETAL".into(), "SKELETAL".into()]).is_err());
        assert!(validate(&["EXECUTE_SCRIPT".into()]).is_err());
    }
}
