mod models;
pub use models::*;

pub fn decode(bytes: &[u8]) -> Result<Monster, Box<dyn std::error::Error>> {
    let monster: Monster = ciborium::from_reader(bytes)?;
    if !(2..=7).contains(&monster.format_version) {
        return Err(format!("unsupported monster format_version {}", monster.format_version).into());
    }
    Ok(monster)
}

pub fn load(path: impl AsRef<std::path::Path>) -> Result<Monster, Box<dyn std::error::Error>> {
    decode(&std::fs::read(path)?)
}

#[cfg(test)]
mod tests {
    #[test]
    fn generated_fixture_decodes() {
        let bytes = include_bytes!("../../fixtures/monster.cbor");
        let monster = super::decode(bytes).unwrap();
        assert!(!monster.id.is_empty());
        assert!(!monster.animations.is_empty());
        assert!(monster.gameplay.as_ref().unwrap().health > 0);
        assert_eq!(monster.generation.as_ref().unwrap().seed, 42);
        assert!(!monster.projectiles.is_empty());
        assert!(monster.attacks.iter().any(|attack| attack.spawn.is_some()));
    }
}
