use prost::Message;

#[derive(Clone, PartialEq, Message)]
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
}
#[derive(Clone, PartialEq, Message)]
pub struct MountInfo {
    #[prost(string, tag = "1")]
    pub rider_package: String,
    #[prost(string, tag = "2")]
    pub mount_package: String,
    #[prost(string, tag = "3")]
    pub survivor: String,
}
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
pub struct AttackPreference {
    #[prost(string, tag = "1")]
    pub attack_id: String,
    #[prost(float, tag = "2")]
    pub weight: f32,
}
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
pub struct SemanticConfidence {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(float, tag = "2")]
    pub confidence: f32,
}
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
pub struct AnimationEvent {
    #[prost(uint32, tag = "1")]
    pub time_ms: u32,
    #[prost(string, tag = "2")]
    pub kind: String,
    #[prost(string, tag = "3")]
    pub reference_id: String,
}
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
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
#[derive(Clone, PartialEq, Message)]
pub struct MovementMode {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(float, tag = "2")]
    pub speed: f32,
    #[prost(string, tag = "3")]
    pub animation_id: String,
}
