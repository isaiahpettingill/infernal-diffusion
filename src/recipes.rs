use crate::{
    anatomy::{self, Body},
    animation::{self, Pose},
    description,
    parser::MonsterSpec,
    physics, proto, render,
};
use image::RgbaImage;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Category {
    SemanticConcept,
    Anatomy,
    Geometry,
    Attachment,
    Material,
    Texture,
    Movement,
    Impact,
    Death,
    Rigging,
    AttackPrimitive,
    AttackTemplate,
    MagicalRepair,
    Description,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeDescriptor {
    pub id: String,
    pub version: u32,
    pub category: Category,
    pub parameter_schema: serde_json::Value,
    pub compatible_plans: Vec<String>,
}

#[derive(Default)]
pub struct RecipeRegistry {
    entries: BTreeMap<String, RecipeDescriptor>,
}

impl RecipeRegistry {
    pub fn register(&mut self, descriptor: RecipeDescriptor) -> Result<(), String> {
        if descriptor.id.trim().is_empty()
            || descriptor.version == 0
            || !descriptor.parameter_schema.is_object()
        {
            return Err(
                "recipe requires a stable ID, positive version, and object parameter schema".into(),
            );
        }
        if self.entries.contains_key(&descriptor.id) {
            return Err(format!("duplicate recipe ID {}", descriptor.id));
        }
        self.entries.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }
    pub fn get(&self, id: &str) -> Option<&RecipeDescriptor> {
        self.entries.get(id)
    }
    pub fn compatible(&self, plan: &str) -> Vec<&RecipeDescriptor> {
        self.entries
            .values()
            .filter(|r| {
                r.compatible_plans.is_empty() || r.compatible_plans.iter().any(|p| p == plan)
            })
            .collect()
    }
    pub fn versions(&self, plan: &str) -> Vec<String> {
        self.compatible(plan)
            .into_iter()
            .map(|r| format!("{}@{}", r.id, r.version))
            .collect()
    }
}

pub fn default_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::default();
    for (category, id, plans) in [
        (Category::SemanticConcept, "semantic.vocabulary", vec![]),
        (
            Category::SemanticConcept,
            "semantic.folklore_cryptid_modern",
            vec![],
        ),
        (Category::Anatomy, "anatomy.continuous", vec![]),
        (Category::Anatomy, "anatomy.seeded_hybrid", vec![]),
        (Category::Anatomy, "anatomy.mounted_composite", vec![]),
        (Category::Geometry, "geometry.ellipse_capsule_sdf", vec![]),
        (Category::Attachment, "attachment.connective", vec![]),
        (Category::Material, "material.semantic_palette", vec![]),
        (Category::Texture, "texture.local_pattern", vec![]),
        (Category::Movement, "movement.gait", vec![]),
        (Category::Movement, "movement.burrow_root_tread", vec![]),
        (Category::Impact, "impact.articulated", vec![]),
        (Category::Death, "death.motor_release", vec![]),
        (Category::Rigging, "rig.rapier_revolute", vec![]),
        (
            Category::AttackPrimitive,
            "attack.bounded_primitives",
            vec![],
        ),
        (Category::AttackTemplate, "attack.anatomy_origin", vec![]),
        (Category::AttackTemplate, "attack.anatomy_mobility", vec![]),
        (Category::AttackTemplate, "attack.bounded_summon", vec![]),
        (Category::MagicalRepair, "repair.viability", vec![]),
        (Category::Description, "description.seeded_voice", vec![]),
    ] {
        let schema = match &category {
            Category::Anatomy => serde_json::json!({"type":"object","properties":{
                "size":{"type":"number","minimum":0,"maximum":1},
                "bulk":{"type":"number","minimum":0,"maximum":1},
                "limb_count":{"type":"integer","minimum":0,"maximum":12}}}),
            Category::Geometry => serde_json::json!({"type":"object","properties":{
                "length":{"type":"number","minimum":0},"width":{"type":"number","minimum":0},
                "curvature":{"type":"number","minimum":-1,"maximum":1}}}),
            Category::Material | Category::Texture => {
                serde_json::json!({"type":"object","properties":{
                "opacity":{"type":"number","minimum":0,"maximum":1},
                "frequency":{"type":"number","minimum":0}}})
            }
            Category::Movement | Category::Impact | Category::Death | Category::Rigging => {
                serde_json::json!({"type":"object","properties":{
                "speed":{"type":"number","minimum":0},"stiffness":{"type":"number","minimum":0}}})
            }
            Category::AttackPrimitive | Category::AttackTemplate => {
                serde_json::json!({"type":"object","properties":{
                "damage":{"type":"integer","minimum":0,"maximum":500},
                "count":{"type":"integer","minimum":0,"maximum":16},
                "cooldown_ms":{"type":"integer","minimum":300}}})
            }
            Category::MagicalRepair => serde_json::json!({"type":"object","properties":{
                "strength":{"type":"number","minimum":0,"maximum":1}}}),
            _ => serde_json::json!({"type":"object","properties":{}}),
        };
        registry
            .register(RecipeDescriptor {
                id: id.into(),
                version: 1,
                category,
                parameter_schema: schema,
                compatible_plans: plans,
            })
            .expect("built-in recipe IDs are unique");
    }
    registry
}

pub fn mesh_registry() -> RecipeRegistry {
    let mut registry = RecipeRegistry::default();
    for id in [
        "mesh.skull_canid",
        "mesh.skull_reptile",
        "mesh.skull_worm",
        "mesh.skull_feline",
        "mesh.skull_bovine",
        "mesh.skull_humanoid",
        "mesh.skull_cyclops",
        "mesh.skull_arthropod",
        "mesh.skull_avian",
        "mesh.skull_generic",
        "mesh.skull_bear",
        "mesh.skull_moose",
        "mesh.skull_rhino",
        "mesh.skull_elephant",
        "mesh.skull_alien",
        "mesh.skull_goblin",
        "mesh.skull_robot",
        "mesh.skull_mushroom",
        "mesh.skull_clown",
        "mesh.skull_anime",
        "mesh.skull_hitler",
        "mesh.skull_tank",
        "mesh.part_horn",
        "mesh.part_claw",
        "mesh.part_fang",
        "mesh.part_wing",
        "mesh.part_wing_feather",
        "mesh.part_shell",
        "mesh.part_hoof",
        "mesh.part_paw",
        "mesh.part_weapon",
        "mesh.part_weapon_sword",
        "mesh.part_weapon_club",
        "mesh.part_weapon_axe",
        "mesh.part_weapon_spear",
        "mesh.part_weapon_bow",
        "mesh.part_weapon_chainsaw",
        "mesh.part_tread",
        "mesh.part_tank_hull",
        "mesh.part_rotor",
    ] {
        registry
            .register(RecipeDescriptor {
                id: id.into(),
                version: 1,
                category: Category::Geometry,
                parameter_schema: serde_json::json!({"type":"object","properties":{
                    "scale_x":{"type":"number","exclusiveMinimum":0},
                    "scale_y":{"type":"number","exclusiveMinimum":0},
                    "scale_z":{"type":"number","exclusiveMinimum":0},
                    "rotation":{"type":"number"}}}),
                compatible_plans: vec![],
            })
            .expect("built-in mesh recipe IDs are unique");
    }
    registry
}

/// Implement any method to replace that generation stage. A custom pipeline
/// must publish a stable ID and version because these enter the generation hash.
pub trait GenerationStages {
    fn id(&self) -> &str {
        "core.pipeline"
    }
    fn version(&self) -> u32 {
        2
    }
    fn registry(&self) -> RecipeRegistry {
        default_registry()
    }
    fn anatomy(&self, spec: &MonsterSpec, rng: &mut ChaCha8Rng) -> Body {
        anatomy::build(spec, rng)
    }
    fn repair(&self, body: &mut Body) {
        physics::assess_and_repair(body)
    }
    fn animate(
        &self,
        spec: &MonsterSpec,
        body: &Body,
    ) -> (
        Vec<proto::Animation>,
        Vec<proto::MovementMode>,
        Vec<proto::Attack>,
        Vec<Pose>,
    ) {
        animation::make(spec, body)
    }
    fn render(&self, body: &Body, poses: &[Pose], size: u32) -> (RgbaImage, u32, u32) {
        render::sheet(body, poses, size)
    }
    fn describe(
        &self,
        spec: &MonsterSpec,
        body: &Body,
        attacks: &[proto::Attack],
    ) -> (String, String) {
        description::describe(spec, body, attacks)
    }
    fn describe_seeded(
        &self,
        spec: &MonsterSpec,
        body: &Body,
        attacks: &[proto::Attack],
        _seed: u64,
    ) -> (String, String) {
        self.describe(spec, body, attacks)
    }
}

pub struct DefaultStages;
impl GenerationStages for DefaultStages {
    fn describe_seeded(
        &self,
        spec: &MonsterSpec,
        body: &Body,
        attacks: &[proto::Attack],
        seed: u64,
    ) -> (String, String) {
        description::describe_seeded(spec, body, attacks, seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_rejects_duplicate_ids() {
        let mut registry = default_registry();
        let existing = registry.get("anatomy.continuous").unwrap().clone();
        assert!(registry.register(existing).is_err());
        assert!(registry
            .versions("ARACHNID")
            .iter()
            .any(|r| r == "rig.rapier_revolute@1"));
    }
    struct CustomRepair;
    impl GenerationStages for CustomRepair {
        fn id(&self) -> &str {
            "test.custom_repair"
        }
        fn version(&self) -> u32 {
            1
        }
        fn repair(&self, body: &mut Body) {
            body.repairs.push("CUSTOM_SUPPORT".into());
        }
    }
    #[test]
    fn stage_override_is_recorded_and_changes_identity() {
        let spec = crate::parser::parse_vocabulary("spider");
        let base = tempfile::tempdir().unwrap();
        let custom = tempfile::tempdir().unwrap();
        let one = crate::generate_from_spec(spec.clone(), 8, base.path()).unwrap();
        let two =
            crate::generate_from_spec_with_stages(spec, 8, custom.path(), &CustomRepair).unwrap();
        assert_ne!(one.id, two.id);
        assert!(two
            .generation
            .unwrap()
            .repairs
            .contains(&"CUSTOM_SUPPORT".into()));
    }
}
