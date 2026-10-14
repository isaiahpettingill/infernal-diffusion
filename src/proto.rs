use prost::Message;

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Monster {
    #[prost(uint32, tag = "1")]
    pub format_version: u32,
    #[prost(string, tag = "2")]
    pub id: String,
    #[prost(string, tag = "3")]
    pub display_name: String,
    #[prost(message, optional, tag = "4")]
    pub generation: Option<GenerationInfo>,
    #[prost(message, optional, tag = "5")]
    pub sprites: Option<SpriteSet>,
    #[prost(message, optional, tag = "6")]
    pub physics: Option<PhysicsInfo>,
    #[prost(message, optional, tag = "7")]
    pub gameplay: Option<GameplayInfo>,
    #[prost(message, repeated, tag = "8")]
    pub animations: Vec<Animation>,
    #[prost(message, repeated, tag = "9")]
    pub attacks: Vec<Attack>,
    #[prost(message, repeated, tag = "10")]
    pub colliders: Vec<Collider>,
    #[prost(message, repeated, tag = "11")]
    pub movement_modes: Vec<MovementMode>,
    #[prost(string, repeated, tag = "12")]
    pub tags: Vec<String>,
    #[prost(string, tag = "13")]
    pub description: String,
    #[prost(message, optional, tag = "14")]
    pub behavior: Option<BehaviorInfo>,
    #[prost(message, repeated, tag = "15")]
    pub projectiles: Vec<Projectile>,
    #[prost(message, optional, tag = "16")]
    pub size: Option<SizeInfo>,
    #[prost(message, optional, tag = "17")]
    pub mount: Option<MountInfo>,
    #[prost(message, optional, tag = "18")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_profile: Option<V2Profile>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MountInfo {
    #[prost(string, tag = "1")]
    pub rider_package: String,
    #[prost(string, tag = "2")]
    pub mount_package: String,
    #[prost(string, tag = "3")]
    pub survivor: String,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SpawnInfo {
    #[prost(string, tag = "1")]
    pub package_path: String,
    #[prost(string, tag = "2")]
    pub monster_id: String,
    #[prost(uint32, tag = "3")]
    pub count: u32,
    #[prost(uint32, tag = "4")]
    pub max_active: u32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SizeInfo {
    #[prost(float, tag = "1")]
    pub normalized_size: f32,
    #[prost(string, tag = "2")]
    pub size_class: String,
    #[prost(float, tag = "3")]
    pub morphology_scale: f32,
    #[prost(float, tag = "4")]
    pub pixels_per_unit: f32,
    #[prost(uint32, tag = "5")]
    pub visible_width_px: u32,
    #[prost(uint32, tag = "6")]
    pub visible_height_px: u32,
    #[prost(float, tag = "7")]
    pub anchor_x_px: f32,
    #[prost(float, tag = "8")]
    pub anchor_y_px: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
/// Optional legacy demo-AI hints. Engines own targeting and movement policy.
pub struct BehaviorInfo {
    #[prost(string, tag = "1")]
    pub style: String,
    #[prost(float, tag = "2")]
    pub aggro_range: f32,
    #[prost(float, tag = "3")]
    pub preferred_range: f32,
    #[prost(float, tag = "4")]
    pub aggression: f32,
    #[prost(float, tag = "5")]
    pub retreat_health_fraction: f32,
    #[prost(string, tag = "6")]
    pub approach_mode: String,
    #[prost(string, tag = "7")]
    pub escape_mode: String,
    #[prost(message, repeated, tag = "8")]
    pub attack_preferences: Vec<AttackPreference>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AttackPreference {
    #[prost(string, tag = "1")]
    pub attack_id: String,
    #[prost(float, tag = "2")]
    pub weight: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GenerationInfo {
    #[prost(string, tag = "1")]
    pub generator_version: String,
    #[prost(string, tag = "2")]
    pub recipe_version: String,
    #[prost(string, tag = "3")]
    pub prompt: String,
    #[prost(uint64, tag = "4")]
    pub seed: u64,
    #[prost(string, tag = "5")]
    pub parser: String,
    #[prost(float, tag = "6")]
    pub parser_confidence: f32,
    #[prost(string, repeated, tag = "7")]
    pub repairs: Vec<String>,
    #[prost(string, repeated, tag = "8")]
    pub recipe_ids: Vec<String>,
    #[prost(message, repeated, tag = "9")]
    pub confidences: Vec<SemanticConfidence>,
    #[prost(message, repeated, tag = "10")]
    pub token_evidence: Vec<TokenEvidence>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SemanticConfidence {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(float, tag = "2")]
    pub confidence: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TokenEvidence {
    #[prost(string, tag = "1")]
    pub label: String,
    #[prost(uint32, tag = "2")]
    pub start: u32,
    #[prost(uint32, tag = "3")]
    pub end: u32,
    #[prost(float, tag = "4")]
    pub confidence: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SpriteSet {
    #[prost(string, tag = "1")]
    pub image_file: String,
    #[prost(uint32, tag = "2")]
    pub frame_width: u32,
    #[prost(uint32, tag = "3")]
    pub frame_height: u32,
    #[prost(uint32, tag = "4")]
    pub columns: u32,
    #[prost(uint32, tag = "5")]
    pub frame_count: u32,
    #[prost(uint32, tag = "6")]
    pub palette_size: u32,
    #[prost(string, tag = "7")]
    pub emission_file: String,
    #[prost(float, repeated, tag = "8")]
    pub direction_angles_deg: Vec<f32>,
    #[prost(uint32, tag = "9")]
    pub direction_stride: u32,
    #[prost(string, tag = "10")]
    pub render_mode: String,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PhysicsInfo {
    #[prost(float, tag = "1")]
    pub mass: f32,
    #[prost(float, tag = "2")]
    pub gravity_scale: f32,
    #[prost(float, tag = "3")]
    pub speed: f32,
    #[prost(float, tag = "4")]
    pub knockback_scale: f32,
    #[prost(float, tag = "5")]
    pub center_of_mass_x: f32,
    #[prost(float, tag = "6")]
    pub center_of_mass_y: f32,
    #[prost(string, repeated, tag = "7")]
    pub side_effects: Vec<String>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GameplayInfo {
    #[prost(uint32, tag = "1")]
    pub health: u32,
    #[prost(uint32, tag = "2")]
    pub defense: u32,
    #[prost(float, tag = "3")]
    pub threat: f32,
    #[prost(uint32, tag = "4")]
    pub health_size_bonus: u32,
    #[prost(uint32, tag = "5")]
    pub health_armor_bonus: u32,
    #[prost(uint32, tag = "6")]
    pub health_magic_bonus: u32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Animation {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub semantic_state: String,
    #[prost(message, repeated, tag = "3")]
    pub frames: Vec<AnimationFrame>,
    #[prost(message, repeated, tag = "4")]
    pub events: Vec<AnimationEvent>,
    #[prost(bool, tag = "5")]
    pub looped: bool,
    #[prost(string, tag = "6")]
    pub movement_mode: String,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AnimationFrame {
    #[prost(uint32, tag = "1")]
    pub sprite_frame_id: u32,
    #[prost(uint32, tag = "2")]
    pub duration_ms: u32,
    #[prost(float, tag = "3")]
    pub root_dx: f32,
    #[prost(float, tag = "4")]
    pub root_dy: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AnimationEvent {
    #[prost(uint32, tag = "1")]
    pub time_ms: u32,
    #[prost(string, tag = "2")]
    pub kind: String,
    #[prost(string, tag = "3")]
    pub reference_id: String,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Attack {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub delivery: String,
    #[prost(string, tag = "3")]
    pub element: String,
    #[prost(string, tag = "4")]
    pub origin_node: String,
    #[prost(uint32, tag = "5")]
    pub damage: u32,
    #[prost(uint32, tag = "6")]
    pub telegraph_ms: u32,
    #[prost(uint32, tag = "7")]
    pub cooldown_ms: u32,
    #[prost(float, tag = "8")]
    pub range: f32,
    #[prost(string, tag = "9")]
    pub animation_id: String,
    #[prost(message, repeated, tag = "10")]
    pub steps: Vec<AttackStep>,
    #[prost(string, tag = "11")]
    pub projectile_id: String,
    #[prost(message, optional, tag = "12")]
    pub spawn: Option<SpawnInfo>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Projectile {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub kind: String,
    #[prost(string, tag = "3")]
    pub image_file: String,
    #[prost(uint32, tag = "4")]
    pub frame_width: u32,
    #[prost(uint32, tag = "5")]
    pub frame_height: u32,
    #[prost(uint32, tag = "6")]
    pub columns: u32,
    #[prost(uint32, tag = "7")]
    pub first_frame: u32,
    #[prost(uint32, tag = "8")]
    pub frame_count: u32,
    #[prost(uint32, tag = "9")]
    pub frame_duration_ms: u32,
    #[prost(float, tag = "10")]
    pub collision_radius: f32,
    #[prost(string, tag = "11")]
    pub orientation_mode: String,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AttackStep {
    #[prost(string, tag = "1")]
    pub primitive: String,
    #[prost(uint32, tag = "2")]
    pub at_ms: u32,
    #[prost(float, tag = "3")]
    pub value: f32,
    #[prost(string, tag = "4")]
    pub target: String,
    #[prost(uint32, tag = "5")]
    pub count: u32,
    #[prost(float, tag = "6")]
    pub spread_deg: f32,
    #[prost(float, tag = "7")]
    pub speed: f32,
    #[prost(float, tag = "8")]
    pub radius: f32,
    #[prost(uint32, tag = "9")]
    pub duration_ms: u32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Collider {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub node_id: String,
    #[prost(float, tag = "3")]
    pub x: f32,
    #[prost(float, tag = "4")]
    pub y: f32,
    #[prost(float, tag = "5")]
    pub radius: f32,
    #[prost(bool, tag = "6")]
    pub hurtbox: bool,
    #[prost(message, repeated, tag = "7")]
    pub views: Vec<ColliderView>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ColliderView {
    #[prost(uint32, tag = "1")]
    pub direction_index: u32,
    #[prost(float, tag = "2")]
    pub x: f32,
    #[prost(float, tag = "3")]
    pub y: f32,
    #[prost(float, tag = "4")]
    pub radius: f32,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
/// Engine-interpreted locomotion descriptor; never a navigation/AI instruction.
pub struct MovementMode {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(float, tag = "2")]
    pub speed: f32,
    #[prost(string, tag = "3")]
    pub animation_id: String,
    /// Broad capability category. Empty only in packages from older generators.
    #[prost(string, tag = "4")]
    pub category: String,
    /// Unordered physical/style qualifiers, not a sequence of movement commands.
    #[prost(string, repeated, tag = "5")]
    pub modifiers: Vec<String>,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct V2Profile {
    #[prost(uint32, tag = "1")]
    pub profile_version: u32,
    #[prost(string, repeated, tag = "2")]
    pub categories: Vec<String>,
    #[prost(string, repeated, tag = "3")]
    pub capabilities: Vec<String>,
    #[prost(string, repeated, tag = "4")]
    pub modifiers: Vec<String>,
    #[prost(message, repeated, tag = "5")]
    pub attack_patterns: Vec<AttackPatternDescriptor>,
    #[prost(message, optional, tag = "6")]
    pub lure: Option<LureDescriptor>,
    #[prost(message, optional, tag = "7")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectral: Option<SpectralDescriptor>,
    #[prost(message, optional, tag = "8")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hybrid: Option<HybridRecipeDescriptor>,
    #[prost(message, repeated, tag = "9")]
    pub body_motion_samples: Vec<BodyMotionSample>,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AttackPatternDescriptor {
    #[prost(string, tag = "1")]
    pub attack_id: String,
    #[prost(string, tag = "2")]
    pub pattern: String,
    #[prost(uint32, tag = "3")]
    pub projectile_count: u32,
    #[prost(uint32, tag = "4")]
    pub volley_count: u32,
    #[prost(uint32, tag = "5")]
    pub interval_ms: u32,
    #[prost(float, tag = "6")]
    pub spread_deg: f32,
    #[prost(string, tag = "7")]
    pub aim_mode: String,
    #[prost(message, repeated, tag = "8")]
    pub motion_samples: Vec<AttackOriginSample>,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LureDescriptor {
    #[prost(string, tag = "1")]
    pub lure_animation_id: String,
    #[prost(string, tag = "2")]
    pub reveal_animation_id: String,
    #[prost(string, tag = "3")]
    pub trigger: String,
    #[prost(float, tag = "4")]
    pub trigger_radius: f32,
    #[prost(string, tag = "5")]
    pub attack_id: String,
    #[prost(string, tag = "6")]
    pub effect: String,
    #[prost(bool, tag = "7")]
    pub harmless: bool,
    #[prost(uint32, tag = "8")]
    pub telegraph_ms: u32,
    #[prost(string, tag = "9")]
    pub distant_state: String,
    #[prost(string, tag = "10")]
    pub revealed_state: String,
    #[prost(string, tag = "11")]
    pub reset_policy: String,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SpectralDescriptor {
    #[prost(string, tag = "1")]
    pub kind: String,
    #[prost(string, tag = "2")]
    pub visual_style: String,
    #[prost(float, tag = "3")]
    pub opacity: f32,
    #[prost(string, tag = "4")]
    pub collision_mode: String,
    #[prost(message, repeated, tag = "5")]
    pub damage_rules: Vec<DamageRule>,
    #[prost(float, tag = "6")]
    pub default_damage_multiplier: f32,
    #[prost(bool, tag = "7")]
    pub default_projectile_pass_through: bool,
    #[prost(message, repeated, tag = "8")]
    pub phase_windows: Vec<PhaseWindow>,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DamageRule {
    #[prost(uint32, tag = "1")]
    pub priority: u32,
    #[prost(string, repeated, tag = "2")]
    pub elements: Vec<String>,
    #[prost(string, tag = "3")]
    pub delivery: String,
    #[prost(string, tag = "4")]
    pub required_tag: String,
    #[prost(float, tag = "5")]
    pub damage_multiplier: f32,
    #[prost(bool, tag = "6")]
    pub projectile_pass_through: bool,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PhaseWindow {
    #[prost(string, tag = "1")]
    pub animation_id: String,
    #[prost(uint32, tag = "2")]
    pub start_ms: u32,
    #[prost(uint32, tag = "3")]
    pub end_ms: u32,
    #[prost(bool, tag = "4")]
    pub solid_pass_through: bool,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HybridRecipeDescriptor {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(string, tag = "2")]
    pub upper_body: String,
    #[prost(string, tag = "3")]
    pub lower_body: String,
    #[prost(string, tag = "4")]
    pub head: String,
    #[prost(uint32, tag = "5")]
    pub lower_limb_count: u32,
    #[prost(uint32, tag = "6")]
    pub upper_arm_count: u32,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AttackOriginSample {
    #[prost(uint32, tag = "1")]
    pub time_ms: u32,
    #[prost(uint32, tag = "2")]
    pub sprite_frame_id: u32,
    #[prost(string, tag = "3")]
    pub node_id: String,
    #[prost(message, repeated, tag = "4")]
    pub views: Vec<ColliderView>,
    #[prost(bool, tag = "5")]
    pub active: bool,
}

#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BodyMotionSample {
    #[prost(string, tag = "1")]
    pub animation_id: String,
    #[prost(uint32, tag = "2")]
    pub time_ms: u32,
    #[prost(uint32, tag = "3")]
    pub sprite_frame_id: u32,
    #[prost(message, repeated, tag = "4")]
    pub hurtboxes: Vec<PosedHurtbox>,
}
#[derive(Clone, PartialEq, Message, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PosedHurtbox {
    #[prost(string, tag = "1")]
    pub node_id: String,
    #[prost(message, repeated, tag = "2")]
    pub views: Vec<ColliderView>,
}
