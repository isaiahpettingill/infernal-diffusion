//! Narrow V2 grammar normalization for mount-first rider requests. The public
//! generation source remains the exact original prompt; only parsing is routed.
use crate::parser::{self, MonsterSpec};

pub fn normalize_mount(prompt: &str) -> Option<MonsterSpec> {
    let text = prompt.to_lowercase();
    if text.contains(" riding ") {
        return None;
    }
    let (before, suffix) = text.rsplit_once(" rider")?;
    if !suffix.is_empty() && !suffix.starts_with(" with ") {
        return None;
    }
    let (mount_clause, rider_clause) = before.rsplit_once(" with ")?;
    if mount_clause.trim().is_empty() || rider_clause.trim().is_empty() {
        return None;
    }
    let rider_clause = format!("{}{suffix}", rider_clause.trim());
    let canonical = format!("{rider_clause} riding {}", mount_clause.trim());
    #[cfg(feature = "bert")]
    let parsed = parser::parse_with_embedded_bert(&canonical).ok()?;
    #[cfg(not(feature = "bert"))]
    let parsed = parser::parse_vocabulary(&canonical);
    parsed.mount.as_ref()?;
    // The clauses cannot re-enter this mount-first normalization. Crucially,
    // 'ghost' in the mount clause does not leak into the rider's defenses.
    let mut rider = super::interpret(parsed, &rider_clause);
    let mount = rider.mount.as_ref()?;
    let lower = parser::mount_spec(&rider, mount);
    let lower = super::interpret(lower, mount_clause.trim());
    let mount = rider.mount.as_mut()?;
    mount.body_plan = lower.body_plan.clone();
    mount.affinity = lower.affinity.clone();
    mount.spec = Some(Box::new(lower));
    fn clear_offsets(spec: &mut MonsterSpec) {
        spec.token_evidence.clear();
        if let Some(lower) = spec.mount.as_mut().and_then(|m| m.spec.as_deref_mut()) {
            clear_offsets(lower);
        }
    }
    // Offsets generated for a rewritten string must never be presented as
    // offsets into the exact original public prompt.
    clear_offsets(&mut rider);
    Some(rider)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mount_first_grammar_keeps_modifiers_and_equipment_local() {
        let spec = normalize_mount("Ghost lion with orc rider with huge axe").unwrap();
        assert_eq!(spec.affinity, "ORC");
        assert_eq!(spec.attack.concept, "AXE");
        assert_eq!(spec.part_scales.get("WEAPON"), Some(&1.8));
        assert!(!spec.features.iter().any(|f| f == "GHOST"));
        assert!(spec.secondary_affinities.is_empty());
        let lower = spec.mount.unwrap().spec.unwrap();
        assert_eq!(lower.affinity, "LION");
        assert!(lower.features.iter().any(|f| f == "GHOST"));
        assert!(!lower.part_scales.contains_key("WEAPON"));
        assert!(lower.token_evidence.is_empty());
    }
    #[test]
    fn ordinary_phrases_and_existing_riding_grammar_are_untouched() {
        for prompt in [
            "orc with huge axe riding ghost lion",
            "lion with riders",
            "horse with a riderless saddle",
            "lion with rider arms",
            "rider with a sword",
        ] {
            assert!(normalize_mount(prompt).is_none(), "{prompt}");
        }
    }
}
