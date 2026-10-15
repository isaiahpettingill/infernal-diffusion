//! Public-pipeline regressions for V2 semantic refinements. V1 entry points stay untouched.
use infernal_diffusion::{generate_in_memory_with_profile, GenerationProfile};
#[test]
fn species_modifiers_export_truthful_packages() {
    for prompt in [
        "cute elephant",
        "chibi wolf",
        "water bear",
        "zombie wolf",
        "grendel",
        "lobster",
        "potato monster",
        "sphere",
        "flaming orc",
        "reaper",
        "cacodaemon",
    ] {
        let p = generate_in_memory_with_profile(prompt, 0, GenerationProfile::V2)
            .unwrap_or_else(|e| panic!("{prompt}: {e}"));
        let m = &p.packages[0].monster;
        assert!(m.v2_profile.is_some());
        if prompt.starts_with("cute") || prompt.starts_with("chibi") {
            assert!(m.v2_profile.as_ref().unwrap().lure.is_none());
            assert!(m.attacks.iter().all(|a| a.delivery != "PROJECTILE"));
        }
        if prompt == "water bear" {
            assert!(m.animations.iter().any(|a| a.id == "water_hover"));
            assert!(m.animations.iter().any(|a| a.id == "attack_water"));
            assert!(m.movement_modes.iter().any(|v| v.category == "SWIMMING"));
        }
        if prompt == "potato monster" {
            assert!(m.projectiles.iter().any(|p| p.kind == "POTATO"));
        }
    }
}
