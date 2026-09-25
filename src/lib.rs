pub mod anatomy;
pub mod animation;
pub mod description;
pub mod mystery;
pub mod parser;
pub mod physics;
pub mod projectile;
pub mod proto;
pub mod recipes;
pub mod render;
pub mod render3d;
pub mod web;

use prost::Message;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use sha2::{Digest, Sha256};
use std::{
    ffi::{c_char, CStr, CString},
    io::Read,
    path::Path,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("prompt must contain visible text and be at most 4096 bytes")]
    InvalidPrompt,
    #[error("invalid monster: {0}")]
    InvalidMonster(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error("BERT model error: {0}")]
    Model(String),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderStyle {
    Flat2d,
    Mesh3d,
}

pub fn generate(
    prompt: &str,
    seed: u64,
    output_dir: &Path,
    model_dir: Option<&Path>,
) -> Result<proto::Monster, Error> {
    generate_prompt(
        prompt,
        seed,
        output_dir,
        model_dir,
        RenderStyle::Mesh3d,
        &mut DiskSink,
    )
}

pub fn generate_2d(
    prompt: &str,
    seed: u64,
    output_dir: &Path,
    model_dir: Option<&Path>,
) -> Result<proto::Monster, Error> {
    generate_prompt(
        prompt,
        seed,
        output_dir,
        model_dir,
        RenderStyle::Flat2d,
        &mut DiskSink,
    )
}

pub fn generate_3d(
    prompt: &str,
    seed: u64,
    output_dir: &Path,
    model_dir: Option<&Path>,
) -> Result<proto::Monster, Error> {
    generate_prompt(
        prompt,
        seed,
        output_dir,
        model_dir,
        RenderStyle::Mesh3d,
        &mut DiskSink,
    )
}

pub struct GeneratedPackage {
    pub path: std::path::PathBuf,
    pub monster: proto::Monster,
    pub sprites: image::RgbaImage,
    pub emission: image::RgbaImage,
    pub projectiles: Option<image::RgbaImage>,
}

pub struct GeneratedMonster {
    pub packages: Vec<GeneratedPackage>,
}

/// Generate full runtime data in memory. No protobuf or image bytes are written.
/// Call `save_generated` only when a package should be persisted.
pub fn generate_in_memory(prompt: &str, seed: u64) -> Result<GeneratedMonster, Error> {
    let mut sink = MemorySink {
        packages: Vec::new(),
    };
    generate_prompt(
        prompt,
        seed,
        Path::new("."),
        None,
        RenderStyle::Mesh3d,
        &mut sink,
    )?;
    Ok(GeneratedMonster {
        packages: sink.packages,
    })
}

pub fn save_generated(generated: &GeneratedMonster, output_dir: &Path) -> Result<(), Error> {
    for package in &generated.packages {
        let destination = output_dir.join(&package.path);
        std::fs::create_dir_all(&destination)?;
        package.sprites.save(destination.join("sprites.png"))?;
        package.emission.save(destination.join("emission.png"))?;
        if let Some(projectiles) = &package.projectiles {
            projectiles.save(destination.join("projectiles.png"))?;
        }
        std::fs::write(
            destination.join("monster.pb"),
            package.monster.encode_to_vec(),
        )?;
    }
    Ok(())
}

trait PackageSink {
    fn store(
        &mut self,
        path: &Path,
        monster: &proto::Monster,
        sheet: &image::RgbaImage,
        projectiles: Option<&image::RgbaImage>,
        preview: Option<(u32, u32, u32)>,
    ) -> Result<(), Error>;
}

struct DiskSink;
impl PackageSink for DiskSink {
    fn store(
        &mut self,
        path: &Path,
        monster: &proto::Monster,
        sheet: &image::RgbaImage,
        projectiles: Option<&image::RgbaImage>,
        preview: Option<(u32, u32, u32)>,
    ) -> Result<(), Error> {
        std::fs::create_dir_all(path)?;
        sheet.save(path.join("sprites.png"))?;
        if let Some(atlas) = projectiles {
            atlas.save(path.join("projectiles.png"))?;
        }
        render::emission_sheet(sheet).save(path.join("emission.png"))?;
        if let Some((size, columns, stride)) = preview {
            render3d::preview(sheet, size, columns, stride).save(path.join("preview.png"))?;
        }
        std::fs::write(path.join("monster.pb"), monster.encode_to_vec())?;
        Ok(())
    }
}

struct MemorySink {
    packages: Vec<GeneratedPackage>,
}
impl PackageSink for MemorySink {
    fn store(
        &mut self,
        path: &Path,
        monster: &proto::Monster,
        sheet: &image::RgbaImage,
        projectiles: Option<&image::RgbaImage>,
        _preview: Option<(u32, u32, u32)>,
    ) -> Result<(), Error> {
        self.packages.push(GeneratedPackage {
            path: path.to_path_buf(),
            monster: monster.clone(),
            sprites: sheet.clone(),
            emission: render::emission_sheet(sheet),
            projectiles: projectiles.cloned(),
        });
        Ok(())
    }
}

fn generate_prompt(
    prompt: &str,
    seed: u64,
    output_dir: &Path,
    model_dir: Option<&Path>,
    render_style: RenderStyle,
    sink: &mut dyn PackageSink,
) -> Result<proto::Monster, Error> {
    let profile = std::env::var_os("INFERNAL_PROFILE").is_some();
    let started = std::time::Instant::now();
    if prompt.trim().is_empty() || prompt.len() > 4096 {
        return Err(Error::InvalidPrompt);
    }
    let spec = if let Some(dir) = model_dir {
        #[cfg(feature = "bert")]
        {
            parser::parse_with_bert(prompt, dir).map_err(|e| Error::Model(e.to_string()))?
        }
        #[cfg(not(feature = "bert"))]
        {
            return Err(Error::Model(format!(
                "rebuild with --features bert to load {}",
                dir.display()
            )));
        }
    } else {
        #[cfg(feature = "bert")]
        {
            parser::parse_with_embedded_bert(prompt).map_err(|e| Error::Model(e.to_string()))?
        }
        #[cfg(not(feature = "bert"))]
        {
            parser::parse_vocabulary(prompt)
        }
    };
    let parsing_ms = started.elapsed().as_secs_f64() * 1000.0;
    let recipe_version = if let Some(dir) = model_dir {
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        for file in ["config.json", "tokenizer.json", "model.safetensors"] {
            let mut input = std::fs::File::open(dir.join(file))?;
            loop {
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                digest.update(&buffer[..n]);
            }
        }
        let head = dir.join("semantic_head.json");
        if head.exists() {
            let mut input = std::fs::File::open(head)?;
            loop {
                let n = input.read(&mut buffer)?;
                if n == 0 {
                    break;
                }
                digest.update(&buffer[..n]);
            }
        }
        format!("recipe-v1-bert-{}", hex_prefix(&digest.finalize(), 16))
    } else {
        #[cfg(feature = "bert")]
        {
            static EMBEDDED_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
            EMBEDDED_HASH
                .get_or_init(|| {
                    let mut digest = Sha256::new();
                    digest.update(include_bytes!("../models/bert-mini/config.json"));
                    digest.update(include_bytes!("../models/bert-mini/tokenizer.json"));
                    digest.update(include_bytes!("../models/bert-mini/model.safetensors"));
                    format!(
                        "recipe-v1-embedded-bert-{}",
                        hex_prefix(&digest.finalize(), 16)
                    )
                })
                .clone()
        }
        #[cfg(not(feature = "bert"))]
        {
            "recipe-v1".into()
        }
    };
    let result = generate_spec_internal(
        spec,
        prompt,
        seed,
        output_dir,
        recipe_version,
        &recipes::DefaultStages,
        sink,
        GenerationOptions {
            render_style,
            allow_auto_spawn: true,
            randomize_unknown: true,
        },
    );
    if profile {
        eprintln!(
            "infernal profile: parse={parsing_ms:.1}ms total={:.1}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    result
}

pub fn generate_from_spec(
    spec: parser::MonsterSpec,
    seed: u64,
    output_dir: &Path,
) -> Result<proto::Monster, Error> {
    let source = serde_json::to_string(&spec).map_err(|e| Error::InvalidMonster(e.to_string()))?;
    generate_spec_internal(
        spec,
        &source,
        seed,
        output_dir,
        "recipe-v1-spec".into(),
        &recipes::DefaultStages,
        &mut DiskSink,
        GenerationOptions {
            render_style: RenderStyle::Mesh3d,
            allow_auto_spawn: true,
            randomize_unknown: false,
        },
    )
}

pub fn generate_from_spec_2d(
    spec: parser::MonsterSpec,
    seed: u64,
    output_dir: &Path,
) -> Result<proto::Monster, Error> {
    let source = serde_json::to_string(&spec).map_err(|e| Error::InvalidMonster(e.to_string()))?;
    generate_spec_internal(
        spec,
        &source,
        seed,
        output_dir,
        "recipe-v1-spec".into(),
        &recipes::DefaultStages,
        &mut DiskSink,
        GenerationOptions {
            render_style: RenderStyle::Flat2d,
            allow_auto_spawn: true,
            randomize_unknown: false,
        },
    )
}

pub fn generate_from_spec_3d(
    spec: parser::MonsterSpec,
    seed: u64,
    output_dir: &Path,
) -> Result<proto::Monster, Error> {
    let source = serde_json::to_string(&spec).map_err(|e| Error::InvalidMonster(e.to_string()))?;
    generate_spec_internal(
        spec,
        &source,
        seed,
        output_dir,
        "recipe-v1-spec".into(),
        &recipes::DefaultStages,
        &mut DiskSink,
        GenerationOptions {
            render_style: RenderStyle::Mesh3d,
            allow_auto_spawn: true,
            randomize_unknown: false,
        },
    )
}

pub fn generate_from_spec_with_stages(
    spec: parser::MonsterSpec,
    seed: u64,
    output_dir: &Path,
    stages: &dyn recipes::GenerationStages,
) -> Result<proto::Monster, Error> {
    let source = serde_json::to_string(&spec).map_err(|e| Error::InvalidMonster(e.to_string()))?;
    generate_spec_internal(
        spec,
        &source,
        seed,
        output_dir,
        "recipe-v1-spec".into(),
        stages,
        &mut DiskSink,
        GenerationOptions {
            render_style: RenderStyle::Flat2d,
            allow_auto_spawn: true,
            randomize_unknown: false,
        },
    )
}

#[derive(Clone, Copy)]
struct GenerationOptions {
    render_style: RenderStyle,
    allow_auto_spawn: bool,
    randomize_unknown: bool,
}

fn generate_spec_internal(
    mut spec: parser::MonsterSpec,
    prompt: &str,
    seed: u64,
    output_dir: &Path,
    recipe_version: String,
    stages: &dyn recipes::GenerationStages,
    sink: &mut dyn PackageSink,
    options: GenerationOptions,
) -> Result<proto::Monster, Error> {
    let render_style = options.render_style;
    let allow_auto_spawn = options.allow_auto_spawn;
    if !spec.size.is_finite()
        || !(0.0..=1.0).contains(&spec.size)
        || !spec.bulk.is_finite()
        || !(0.0..=1.0).contains(&spec.bulk)
        || !spec.confidence.is_finite()
        || !(0.0..=1.0).contains(&spec.confidence)
        || spec.confidences.iter().any(|(key, value)| {
            key.is_empty() || !value.is_finite() || !(0.0..=1.0).contains(value)
        })
        || spec.part_scales.iter().any(|(key, value)| {
            ![
                "HEAD", "HEAD_0", "ARM", "ARM_0", "WING", "WING_0", "LEG", "LEG_0", "TAIL", "HORN",
                "HORN_0", "EYE", "EYE_0", "CLAW", "WEAPON",
            ]
            .contains(&key.as_str())
                || !value.is_finite()
                || !(0.5..=2.2).contains(value)
        })
        || spec.token_evidence.iter().any(|e| {
            e.label.is_empty()
                || e.start >= e.end
                || e.end > prompt.len()
                || e.end > u32::MAX as usize
                || !e.confidence.is_finite()
                || !(0.0..=1.0).contains(&e.confidence)
        })
        || spec.materials.is_empty()
        || spec.heads == 0
        || spec.heads > 5
        || spec.limb_count > 12
        || spec.arm_count > 8
        || spec.secondary_affinities.len() > 2
        || spec
            .secondary_affinities
            .iter()
            .any(|affinity| affinity.is_empty())
        || spec.spawn.as_ref().is_some_and(|spawn| {
            spawn.target.is_empty()
                || spawn.count == 0
                || spawn.count > 3
                || spawn.max_active < spawn.count
                || spawn.max_active > 8
        })
        || spec.mount.as_ref().is_some_and(|mount| {
            mount.affinity.is_empty()
                || !["RIDER", "MOUNT", "EITHER"].contains(&mount.survivor.as_str())
                || ![
                    "QUADRUPED",
                    "THEROPOD",
                    "WINGED",
                    "ARACHNID",
                    "INSECT",
                    "SERPENT",
                    "GASTROPOD",
                ]
                .contains(&mount.body_plan.as_str())
        })
        || !["MELEE", "PROJECTILE"].contains(&spec.attack.delivery.as_str())
        || ![
            "ARACHNID",
            "INSECT",
            "QUADRUPED",
            "THEROPOD",
            "SERPENT",
            "GASTROPOD",
            "WINGED",
            "FLOATING",
            "AMORPHOUS",
            "HUMANOID",
            "FUNGUS",
            "TANK",
        ]
        .contains(&spec.body_plan.as_str())
    {
        return Err(Error::InvalidMonster("invalid MonsterSpec".into()));
    }
    if stages.id().trim().is_empty() || stages.version() == 0 {
        return Err(Error::InvalidMonster(
            "stage pipeline needs a stable ID and version".into(),
        ));
    }
    let mut recipe_ids = stages.registry().versions(&spec.body_plan);
    if options.randomize_unknown && spec.affinity == "UNKNOWN" {
        recipe_ids.push("morphology.seeded_mystery@1".into());
    }
    if render_style == RenderStyle::Mesh3d {
        recipe_ids.extend(recipes::mesh_registry().versions(&spec.body_plan));
    }
    let mut recipe_digest = Sha256::new();
    for id in &recipe_ids {
        recipe_digest.update(id.as_bytes());
        recipe_digest.update([0]);
    }
    let recipe_version = format!(
        "{recipe_version}-{}@{}-{}",
        stages.id(),
        stages.version(),
        hex_prefix(&recipe_digest.finalize(), 16)
    );
    let recipe_version = if render_style == RenderStyle::Mesh3d {
        format!(
            "{recipe_version}-mesh3d@5-{}",
            render3d::recipe_fingerprint()
        )
    } else {
        recipe_version
    };
    let mut hasher = Sha256::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(recipe_version.as_bytes());
    hasher.update(prompt.as_bytes());
    hasher.update(seed.to_le_bytes());
    let hash = hasher.finalize();
    let mut rng = ChaCha8Rng::from_seed(hash.into());
    if options.randomize_unknown {
        mystery::fill_unknown(&mut spec, &mut rng);
    }
    if allow_auto_spawn
        && spec.spawn.is_none()
        && ["ARACHNID", "INSECT"].contains(&spec.body_plan.as_str())
        && spec.size <= 0.32
        && spec.bulk <= 0.65
        && 20.0 + spec.size * 90.0 + spec.bulk * 22.0 + 14.0 <= 75.0
        && rng.random_bool(0.38)
    {
        spec.spawn = Some(parser::SpawnIntent {
            target: "SELF".into(),
            count: 1,
            max_active: 3,
        });
    }
    if let Some(mount) = spec.mount.as_mut() {
        if mount.survivor == "EITHER" {
            mount.survivor = if rng.random_bool(0.5) {
                "RIDER"
            } else {
                "MOUNT"
            }
            .into();
        }
    }
    let mut body = stages.anatomy(&spec, &mut rng);
    validate_body(&body)?;
    stages.repair(&mut body);
    validate_body(&body)?;
    let (animations, movement_modes, mut attacks, poses) = stages.animate(&spec, &body);
    if movement_modes.len() < 2 || attacks.len() < 2 || poses.is_empty() {
        return Err(Error::InvalidMonster(
            "animation stage omitted required movement, attacks, or poses".into(),
        ));
    }
    let (projectiles, projectile_sheet) = projectile::bake(&spec, &mut attacks, seed);
    let base_size = if spec.size > 0.8 {
        96
    } else if spec.size < 0.35 {
        48
    } else {
        64
    };
    let size = if render_style == RenderStyle::Mesh3d
        && spec
            .mount
            .as_ref()
            .and_then(|mount| mount.spec.as_ref())
            .is_some_and(|child| child.mount.is_some())
    {
        192
    } else if render_style == RenderStyle::Mesh3d
        && matches!(
            spec.affinity.as_str(),
            "SAND_WORM" | "KRAKEN" | "LONGNECK_DINO" | "MAMMOTH" | "ORCA" | "TREE_MONSTER"
        )
    {
        160
    } else if render_style == RenderStyle::Mesh3d
        && (spec.mount.is_some() || matches!(spec.affinity.as_str(), "CENTIPEDE" | "JAR_JAR"))
    {
        128
    } else if render_style == RenderStyle::Mesh3d {
        match base_size {
            48 => 72,
            64 => 96,
            _ => 128,
        }
    } else {
        base_size
    };
    let (sheet, columns, palette_size) = if render_style == RenderStyle::Mesh3d {
        render3d::sheet(&spec, &body, &poses, size, seed)
    } else {
        stages.render(&body, &poses, size)
    };
    if palette_size >= 256 {
        return Err(Error::InvalidMonster(
            "sprite sheet exceeds 255 RGBA colors".into(),
        ));
    }
    let size_info = if render_style == RenderStyle::Mesh3d {
        let (visible_width_px, visible_height_px) =
            render3d::visible_extent(&sheet, size, columns, poses.len() as u32);
        Some(proto::SizeInfo {
            normalized_size: spec.size,
            size_class: if spec.size < 0.35 {
                "SMALL"
            } else if spec.size > 0.8 {
                "GIANT"
            } else {
                "STANDARD"
            }
            .into(),
            morphology_scale: anatomy::morphology_scale(spec.size),
            pixels_per_unit: render3d::PIXELS_PER_UNIT,
            visible_width_px,
            visible_height_px,
            anchor_x_px: size as f32 * 0.5,
            anchor_y_px: size as f32 * 0.57,
        })
    } else {
        None
    };
    let colliders = body
        .nodes
        .iter()
        .map(|n| proto::Collider {
            id: format!("hurt_{}", n.id),
            node_id: n.id.clone(),
            x: n.x,
            y: n.y,
            radius: n.rx.min(n.ry).max(0.5),
            hurtbox: !["EYE", "MOUTH", "WING", "FIN", "VORTEX", "WEAPON"]
                .contains(&n.kind.as_str()),
            views: if render_style == RenderStyle::Mesh3d {
                render3d::collider_views(n, size)
            } else {
                Vec::new()
            },
        })
        .collect();
    let id = format!("monster-{}", hex_prefix(&hash, 8));
    let (display_name, description) = stages.describe_seeded(&spec, &body, &attacks, seed);
    let mut side_effects: Vec<String> = body
        .repairs
        .iter()
        .filter_map(|repair| match repair.as_str() {
            "GRAVITY_REDUCTION" | "MAGICAL_LIFT" => Some("INCREASED_KNOCKBACK"),
            "TELEKINETIC_BALANCE" => Some("DELAYED_BALANCE_CORRECTION"),
            "JOINT_REINFORCEMENT" => Some("STIFF_MOTION"),
            "RECOIL_DAMPING" => Some("SLOWER_ATTACK_RECOVERY"),
            "SPECTRAL_COLLISION" => Some("PHASED_APPENDAGES"),
            _ => None,
        })
        .map(str::to_string)
        .collect();
    side_effects.sort();
    side_effects.dedup();
    let ranged = attacks
        .iter()
        .any(|a| a.delivery == "PROJECTILE" || a.delivery == "FIELD");
    let burrows = movement_modes.iter().any(|m| m.id == "BURROW");
    let preference_weight: Vec<f32> = attacks
        .iter()
        .map(|attack| match attack.id.as_str() {
            "primary" => 1.0,
            "secondary" => 0.65,
            "special" => 0.45,
            "summon" => 0.32,
            _ => 0.6,
        })
        .collect();
    let preference_total: f32 = preference_weight.iter().sum();
    let behavior = proto::BehaviorInfo {
        style: if burrows {
            "AMBUSH"
        } else if ranged {
            "SKIRMISH"
        } else {
            "AGGRESSIVE"
        }
        .into(),
        aggro_range: if ranged { 220.0 } else { 140.0 },
        preferred_range: if ranged { 105.0 } else { 22.0 },
        aggression: (0.35 + spec.size * 0.35 + spec.bulk * 0.2).clamp(0.0, 1.0),
        retreat_health_fraction: if ranged { 0.2 } else { 0.05 },
        approach_mode: movement_modes[0].id.clone(),
        escape_mode: movement_modes[1].id.clone(),
        attack_preferences: attacks
            .iter()
            .enumerate()
            .map(|(i, a)| proto::AttackPreference {
                attack_id: a.id.clone(),
                weight: preference_weight[i] / preference_total,
            })
            .collect(),
    };
    let mount_info = if let Some(mount) = &spec.mount {
        let mut rider_spec = spec.clone();
        rider_spec.mount = None;
        rider_spec.spawn = None;
        rider_spec.secondary_affinities.clear();
        rider_spec.size = if ["GOBLIN", "TOKOLOSHE"].contains(&spec.affinity.as_str()) {
            0.26
        } else {
            (spec.size * 0.72).clamp(0.25, 0.65)
        };
        let rider_source = serde_json::to_string(&rider_spec)
            .map_err(|error| Error::InvalidMonster(error.to_string()))?;
        let rider = generate_spec_internal(
            rider_spec,
            &rider_source,
            seed ^ 0x52_49_44_45_52,
            &output_dir.join("companions/rider"),
            recipe_version.clone(),
            stages,
            sink,
            GenerationOptions {
                render_style,
                allow_auto_spawn: false,
                randomize_unknown: false,
            },
        )?;
        let mount_spec = parser::mount_spec(&spec, mount);
        let mount_source = serde_json::to_string(&mount_spec)
            .map_err(|error| Error::InvalidMonster(error.to_string()))?;
        let mount_child = generate_spec_internal(
            mount_spec,
            &mount_source,
            seed ^ 0x4d_4f_55_4e_54,
            &output_dir.join("companions/mount"),
            recipe_version.clone(),
            stages,
            sink,
            GenerationOptions {
                render_style,
                allow_auto_spawn: false,
                randomize_unknown: false,
            },
        )?;
        let _ = (rider, mount_child);
        Some(proto::MountInfo {
            rider_package: "companions/rider".into(),
            mount_package: "companions/mount".into(),
            survivor: mount.survivor.clone(),
        })
    } else {
        None
    };
    if let Some(intent) = &spec.spawn {
        let mut minion_spec = if intent.target == "SELF" {
            spec.clone()
        } else {
            parser::parse_vocabulary(&intent.target.to_lowercase().replace('_', " "))
        };
        minion_spec.spawn = None;
        minion_spec.mount = None;
        minion_spec.secondary_affinities.clear();
        minion_spec.size = if intent.target == "SELF" {
            (spec.size * 0.45).clamp(0.12, 0.28)
        } else {
            0.22
        };
        minion_spec.bulk = minion_spec.bulk.min(0.55);
        let minion_source = serde_json::to_string(&minion_spec)
            .map_err(|error| Error::InvalidMonster(error.to_string()))?;
        let minion = generate_spec_internal(
            minion_spec,
            &minion_source,
            seed ^ 0x4d_49_4e_49_4f_4e,
            &output_dir.join("companions/minion"),
            recipe_version.clone(),
            stages,
            sink,
            GenerationOptions {
                render_style,
                allow_auto_spawn: false,
                randomize_unknown: false,
            },
        )?;
        if let Some(summon) = attacks.iter_mut().find(|attack| attack.id == "summon") {
            summon.spawn = Some(proto::SpawnInfo {
                package_path: "companions/minion".into(),
                monster_id: minion.id,
                count: intent.count as u32,
                max_active: intent.max_active as u32,
            });
        }
    }
    let size_health = (spec.size * 90.0).round() as u32;
    let armor_material = spec.materials.iter().any(|material| {
        ["CHITIN", "SCALES", "STONE", "METAL", "BONE"].contains(&material.as_str())
    });
    let armor_health = (spec.bulk * 22.0).round() as u32 + if armor_material { 14 } else { 0 };
    let magic_health = (if spec.attack.element != "PHYSICAL" {
        12
    } else {
        0
    }) + (if spec
        .materials
        .iter()
        .any(|material| ["SPECTRAL", "ENERGY", "FIRE"].contains(&material.as_str()))
    {
        16
    } else {
        0
    }) + (body.repairs.len() as u32 * 4).min(16);
    let monster = proto::Monster {
        format_version: if render_style == RenderStyle::Mesh3d {
            7
        } else {
            2
        },
        id,
        display_name,
        generation: Some(proto::GenerationInfo {
            generator_version: env!("CARGO_PKG_VERSION").into(),
            recipe_version,
            prompt: prompt.into(),
            seed,
            parser: spec.parser.clone(),
            parser_confidence: spec.confidence,
            repairs: body.repairs.clone(),
            recipe_ids,
            confidences: spec
                .confidences
                .iter()
                .map(|(key, confidence)| proto::SemanticConfidence {
                    key: key.clone(),
                    confidence: *confidence,
                })
                .collect(),
            token_evidence: spec
                .token_evidence
                .iter()
                .map(|e| proto::TokenEvidence {
                    label: e.label.clone(),
                    start: e.start as u32,
                    end: e.end as u32,
                    confidence: e.confidence,
                })
                .collect(),
        }),
        sprites: Some(proto::SpriteSet {
            image_file: "sprites.png".into(),
            frame_width: size,
            frame_height: size,
            columns,
            frame_count: poses.len() as u32
                * if render_style == RenderStyle::Mesh3d {
                    render3d::ANGLES_DEG.len() as u32
                } else {
                    1
                },
            palette_size,
            emission_file: "emission.png".into(),
            direction_angles_deg: if render_style == RenderStyle::Mesh3d {
                render3d::ANGLES_DEG.to_vec()
            } else {
                vec![0.0]
            },
            direction_stride: poses.len() as u32,
            render_mode: if render_style == RenderStyle::Mesh3d {
                "MESH_3D"
            } else {
                "SDF_2D"
            }
            .into(),
        }),
        physics: Some(proto::PhysicsInfo {
            mass: body.mass,
            gravity_scale: body.gravity,
            speed: movement_modes[0].speed,
            knockback_scale: ((400.0 / body.mass).clamp(0.1, 2.0)
                * if body.gravity < 1.0 { 1.25 } else { 1.0 })
            .min(2.5),
            center_of_mass_x: body.com.0,
            center_of_mass_y: body.com.1,
            side_effects,
        }),
        gameplay: Some(proto::GameplayInfo {
            health: 20 + size_health + armor_health + magic_health,
            defense: (spec.bulk * 6.0) as u32 + if armor_material { 3 } else { 0 },
            threat: (spec.size
                + spec.bulk
                + if spec.attack.delivery == "PROJECTILE" {
                    0.4
                } else {
                    0.0
                })
                / 2.0,
            health_size_bonus: size_health,
            health_armor_bonus: armor_health,
            health_magic_bonus: magic_health,
        }),
        animations,
        attacks,
        colliders,
        movement_modes,
        tags: std::iter::once(spec.body_plan.clone())
            .chain(std::iter::once(spec.affinity.clone()))
            .chain(spec.secondary_affinities.iter().cloned())
            .chain(spec.features.iter().cloned())
            .chain(std::iter::once(spec.materials[0].clone()))
            .chain(std::iter::once(spec.attack.element.clone()))
            .chain(spec.mount.as_ref().map(|_| "MOUNTED".into()))
            .chain(spec.spawn.as_ref().map(|_| "SUMMONER".into()))
            .collect(),
        description,
        behavior: Some(behavior),
        projectiles,
        size: size_info,
        mount: mount_info,
    };
    validate(&monster, &sheet)?;
    if let Some(atlas) = &projectile_sheet {
        if !projectile::valid_atlas(&monster.projectiles, atlas) {
            return Err(Error::InvalidMonster("invalid projectile atlas".into()));
        }
    }
    sink.store(
        output_dir,
        &monster,
        &sheet,
        projectile_sheet.as_ref(),
        (render_style == RenderStyle::Mesh3d).then_some((size, columns, poses.len() as u32)),
    )?;
    Ok(monster)
}

pub fn export_handcrafted(
    monster: &proto::Monster,
    sheet: &image::RgbaImage,
    output_dir: &Path,
) -> Result<(), Error> {
    export_handcrafted_with_projectiles(monster, sheet, None, output_dir)
}

pub fn export_handcrafted_with_projectiles(
    monster: &proto::Monster,
    sheet: &image::RgbaImage,
    projectile_sheet: Option<&image::RgbaImage>,
    output_dir: &Path,
) -> Result<(), Error> {
    validate(monster, sheet)?;
    if !monster.projectiles.is_empty()
        && !projectile_sheet
            .is_some_and(|atlas| projectile::valid_atlas(&monster.projectiles, atlas))
    {
        return Err(Error::InvalidMonster(
            "missing or invalid projectile atlas".into(),
        ));
    }
    std::fs::create_dir_all(output_dir)?;
    sheet.save(output_dir.join("sprites.png"))?;
    if let Some(atlas) = projectile_sheet {
        atlas.save(output_dir.join("projectiles.png"))?;
    }
    if monster
        .sprites
        .as_ref()
        .is_some_and(|s| !s.emission_file.is_empty())
    {
        render::emission_sheet(sheet).save(output_dir.join("emission.png"))?;
    }
    std::fs::write(output_dir.join("monster.pb"), monster.encode_to_vec())?;
    Ok(())
}

pub fn load_package(output_dir: &Path) -> Result<proto::Monster, Error> {
    load_package_inner(output_dir, 0)
}

fn load_package_inner(output_dir: &Path, depth: u8) -> Result<proto::Monster, Error> {
    let bytes = std::fs::read(output_dir.join("monster.pb"))?;
    let monster = proto::Monster::decode(bytes.as_slice())
        .map_err(|e| Error::InvalidMonster(format!("invalid protobuf: {e}")))?;
    let sprites = monster
        .sprites
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing sprites".into()))?;
    let sheet = image::open(output_dir.join(&sprites.image_file))?.into_rgba8();
    validate(&monster, &sheet)?;
    if !sprites.emission_file.is_empty() {
        let emission = image::open(output_dir.join(&sprites.emission_file))?.into_rgba8();
        if emission.dimensions() != sheet.dimensions() {
            return Err(Error::InvalidMonster(
                "emission atlas dimensions differ".into(),
            ));
        }
    }
    if !monster.projectiles.is_empty() {
        let atlas = image::open(output_dir.join("projectiles.png"))?.into_rgba8();
        if !projectile::valid_atlas(&monster.projectiles, &atlas) {
            return Err(Error::InvalidMonster("invalid projectile atlas".into()));
        }
    }
    if depth < 4 {
        for attack in &monster.attacks {
            if let Some(spawn) = &attack.spawn {
                let child = load_package_inner(&output_dir.join(&spawn.package_path), depth + 1)?;
                if child.id != spawn.monster_id {
                    return Err(Error::InvalidMonster("spawned package ID mismatch".into()));
                }
            }
        }
        if let Some(mount) = &monster.mount {
            load_package_inner(&output_dir.join(&mount.rider_package), depth + 1)?;
            load_package_inner(&output_dir.join(&mount.mount_package), depth + 1)?;
        }
    } else if monster.mount.is_some() || monster.attacks.iter().any(|attack| attack.spawn.is_some())
    {
        return Err(Error::InvalidMonster(
            "companion nesting exceeds four levels".into(),
        ));
    }
    Ok(monster)
}

fn hex_prefix(bytes: &[u8], len: usize) -> String {
    bytes
        .iter()
        .take(len / 2)
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn validate_body(body: &anatomy::Body) -> Result<(), Error> {
    if body.nodes.is_empty()
        || body.nodes.len() > 512
        || body.nodes[0].kind != "TORSO"
        || body.nodes[0].parent.is_some()
        || ![body.mass, body.com.0, body.com.1, body.gravity]
            .iter()
            .all(|v| v.is_finite())
        || body.mass <= 0.0
        || !(0.0..=2.0).contains(&body.gravity)
    {
        return Err(Error::InvalidMonster("invalid anatomy body".into()));
    }
    let mut ids = std::collections::HashSet::new();
    for (i, node) in body.nodes.iter().enumerate() {
        if node.id.is_empty()
            || node.kind.is_empty()
            || node.material.is_empty()
            || !ids.insert(node.id.as_str())
            || ![node.x, node.y, node.rx, node.ry, node.angle, node.density]
                .iter()
                .all(|v| v.is_finite())
            || node.rx <= 0.0
            || node.ry <= 0.0
            || node.density <= 0.0
            || (i > 0 && node.parent.is_none())
            || node.parent.is_some_and(|parent| {
                parent >= i || matches!(body.nodes[parent].kind.as_str(), "EYE" | "MOUTH" | "HORN")
            })
        {
            return Err(Error::InvalidMonster(format!(
                "invalid anatomy node {}",
                node.id
            )));
        }
    }
    Ok(())
}

pub fn validate(monster: &proto::Monster, sheet: &image::RgbaImage) -> Result<(), Error> {
    if ![2, 3, 4, 5, 6, 7].contains(&monster.format_version) {
        return Err(Error::InvalidMonster("unsupported format_version".into()));
    }
    if monster.id.trim().is_empty()
        || monster.display_name.trim().is_empty()
        || monster.description.trim().is_empty()
    {
        return Err(Error::InvalidMonster("missing monster identity".into()));
    }
    let generation = monster
        .generation
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing generation info".into()))?;
    if generation.generator_version.is_empty()
        || generation.recipe_version.is_empty()
        || generation.parser.is_empty()
        || !generation.parser_confidence.is_finite()
        || !(0.0..=1.0).contains(&generation.parser_confidence)
        || generation.confidences.iter().any(|c| {
            c.key.is_empty() || !c.confidence.is_finite() || !(0.0..=1.0).contains(&c.confidence)
        })
        || generation.token_evidence.iter().any(|e| {
            e.label.is_empty()
                || e.start >= e.end
                || e.end as usize > generation.prompt.len()
                || !e.confidence.is_finite()
                || !(0.0..=1.0).contains(&e.confidence)
        })
    {
        return Err(Error::InvalidMonster("invalid generation info".into()));
    }
    let gameplay = monster
        .gameplay
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing gameplay info".into()))?;
    if gameplay.health == 0 || !gameplay.threat.is_finite() || gameplay.threat < 0.0 {
        return Err(Error::InvalidMonster("invalid gameplay info".into()));
    }
    fn unique_nonempty<'a>(values: impl Iterator<Item = &'a str>) -> bool {
        let mut ids = std::collections::HashSet::new();
        values
            .into_iter()
            .all(|id| !id.is_empty() && ids.insert(id))
    }
    if !unique_nonempty(monster.animations.iter().map(|a| a.id.as_str()))
        || !unique_nonempty(monster.attacks.iter().map(|a| a.id.as_str()))
        || !unique_nonempty(monster.projectiles.iter().map(|p| p.id.as_str()))
        || !unique_nonempty(monster.colliders.iter().map(|c| c.id.as_str()))
        || !unique_nonempty(monster.movement_modes.iter().map(|m| m.id.as_str()))
        || monster.colliders.is_empty()
        || monster.attacks.is_empty()
        || monster.movement_modes.is_empty()
    {
        return Err(Error::InvalidMonster(
            "invalid or duplicate runtime IDs".into(),
        ));
    }
    let sprites = monster
        .sprites
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing sprites".into()))?;
    if sprites.image_file != "sprites.png"
        || (!sprites.emission_file.is_empty() && sprites.emission_file != "emission.png")
    {
        return Err(Error::InvalidMonster(
            "invalid sprite file reference".into(),
        ));
    }
    if sprites.frame_count == 0
        || sprites.columns == 0
        || sprites.frame_width == 0
        || sprites.frame_height == 0
        || sprites.columns.checked_mul(sprites.frame_width) != Some(sheet.width())
        || sprites
            .frame_height
            .checked_mul(sprites.frame_count.div_ceil(sprites.columns))
            .is_none_or(|required| sheet.height() != required)
    {
        return Err(Error::InvalidMonster("invalid sprite dimensions".into()));
    }
    if monster.format_version >= 5 {
        let size = monster
            .size
            .as_ref()
            .ok_or_else(|| Error::InvalidMonster("missing size metadata".into()))?;
        if !size.normalized_size.is_finite()
            || !(0.0..=1.0).contains(&size.normalized_size)
            || !["SMALL", "STANDARD", "GIANT"].contains(&size.size_class.as_str())
            || !size.morphology_scale.is_finite()
            || size.morphology_scale <= 0.0
            || !size.pixels_per_unit.is_finite()
            || size.pixels_per_unit <= 0.0
            || size.visible_width_px == 0
            || size.visible_width_px > sprites.frame_width
            || size.visible_height_px == 0
            || size.visible_height_px > sprites.frame_height
            || !size.anchor_x_px.is_finite()
            || !(0.0..sprites.frame_width as f32).contains(&size.anchor_x_px)
            || !size.anchor_y_px.is_finite()
            || !(0.0..sprites.frame_height as f32).contains(&size.anchor_y_px)
        {
            return Err(Error::InvalidMonster("invalid size metadata".into()));
        }
    }
    if let Some(mount) = &monster.mount {
        if mount.rider_package != "companions/rider"
            || mount.mount_package != "companions/mount"
            || !["RIDER", "MOUNT"].contains(&mount.survivor.as_str())
        {
            return Err(Error::InvalidMonster("invalid mount metadata".into()));
        }
    }
    let directions = if monster.format_version >= 7 { 4 } else { 8 };
    if monster.format_version >= 3 {
        if sprites.render_mode != "MESH_3D"
            || sprites.direction_angles_deg.len() != directions
            || sprites.direction_stride == 0
            || sprites.direction_stride.checked_mul(directions as u32) != Some(sprites.frame_count)
            || sprites
                .direction_angles_deg
                .iter()
                .any(|v| !v.is_finite() || !(0.0..360.0).contains(v))
        {
            return Err(Error::InvalidMonster("invalid directional atlas".into()));
        }
    } else if !sprites.direction_angles_deg.is_empty()
        && (sprites.direction_angles_deg != [0.0]
            || sprites.direction_stride != sprites.frame_count)
    {
        return Err(Error::InvalidMonster("invalid 2D atlas metadata".into()));
    }
    if !sheet.pixels().any(|p| p[3] > 0) {
        return Err(Error::InvalidMonster("sprite is invisible".into()));
    }
    if monster.format_version >= 7 {
        let actual = render::color_count(sheet) as u32;
        if actual >= 256 || sprites.palette_size != actual {
            return Err(Error::InvalidMonster(
                "sprite sheet exceeds color budget or has incorrect palette metadata".into(),
            ));
        }
    }
    let required = [
        "IDLE",
        "MOVE",
        "FAST_MOVE",
        "TURN",
        "ATTACK",
        "IMPACT_LIGHT",
        "IMPACT_HEAVY",
        "KNOCKBACK",
        "STUN",
        "DEATH",
    ];
    for state in required {
        if !monster.animations.iter().any(|a| a.semantic_state == state) {
            return Err(Error::InvalidMonster(format!("missing {state}")));
        }
    }
    for id in [
        "impact_light_front",
        "impact_light_back",
        "impact_heavy_front",
        "impact_heavy_back",
        "knockback",
        "stun",
        "death",
    ] {
        if !monster.animations.iter().any(|a| a.id == id) {
            return Err(Error::InvalidMonster(format!("missing reaction clip {id}")));
        }
    }
    for mode in &monster.movement_modes {
        let transitions: &[&str] = match mode.id.as_str() {
            "FLY" => &["takeoff", "fly", "fly_turn", "land_from_flight"],
            "BURROW" => &["burrow", "burrowed_move", "emerge"],
            _ => &[],
        };
        for id in transitions {
            if !monster.animations.iter().any(|a| a.id == *id) {
                return Err(Error::InvalidMonster(format!("missing transition {id}")));
            }
        }
    }
    for a in &monster.animations {
        if a.frames.is_empty()
            || a.semantic_state.is_empty()
            || a.frames.iter().any(|f| {
                f.duration_ms == 0
                    || f.sprite_frame_id
                        >= if monster.format_version >= 3 {
                            sprites.direction_stride
                        } else {
                            sprites.frame_count
                        }
                    || !f.root_dx.is_finite()
                    || !f.root_dy.is_finite()
            })
        {
            return Err(Error::InvalidMonster(format!("invalid frames in {}", a.id)));
        }
        let total: u32 = a
            .frames
            .iter()
            .fold(0u32, |v, f| v.saturating_add(f.duration_ms));
        if a.events.iter().any(|e| e.time_ms > total) {
            return Err(Error::InvalidMonster(format!("invalid event in {}", a.id)));
        }
    }
    if !monster.animations.iter().any(|a| {
        a.id == "death" && !a.looped && a.events.iter().any(|e| e.kind == "DEATH_COMPLETE")
    }) {
        return Err(Error::InvalidMonster(
            "death has no completion event".into(),
        ));
    }
    if monster.colliders.iter().any(|c| {
        ![c.x, c.y, c.radius].iter().all(|v| v.is_finite())
            || c.radius <= 0.0
            || (monster.format_version >= 3
                && (c.views.len() != directions
                    || (0..directions as u32).any(|direction| {
                        c.views
                            .iter()
                            .filter(|view| view.direction_index == direction)
                            .count()
                            != 1
                    })))
            || c.views.iter().any(|view| {
                view.direction_index >= directions as u32
                    || ![view.x, view.y, view.radius]
                        .iter()
                        .all(|value| value.is_finite())
                    || view.radius <= 0.0
            })
    }) {
        return Err(Error::InvalidMonster("invalid collider".into()));
    }
    for m in &monster.movement_modes {
        if !monster
            .animations
            .iter()
            .any(|a| a.id == m.animation_id && a.movement_mode == m.id)
            || !m.speed.is_finite()
            || m.speed < 0.0
            || (m.speed == 0.0 && !monster.tags.iter().any(|tag| tag == "ANCHORED"))
        {
            return Err(Error::InvalidMonster(format!("invalid movement {}", m.id)));
        }
    }
    for a in &monster.attacks {
        if (a.delivery == "SUMMON") != a.spawn.is_some() {
            return Err(Error::InvalidMonster(format!(
                "invalid summon reference for {}",
                a.id
            )));
        }
        if let Some(spawn) = &a.spawn {
            if spawn.package_path != "companions/minion"
                || spawn.monster_id.is_empty()
                || spawn.count == 0
                || spawn.count > 3
                || spawn.max_active < spawn.count
                || spawn.max_active > 8
                || a.cooldown_ms < 1800
            {
                return Err(Error::InvalidMonster(format!(
                    "invalid summon metadata for {}",
                    a.id
                )));
            }
        }
        if (a.delivery == "PROJECTILE" && monster.format_version >= 4 && a.projectile_id.is_empty())
            || (!a.projectile_id.is_empty()
                && (a.delivery != "PROJECTILE"
                    || !monster.projectiles.iter().any(|p| p.id == a.projectile_id)))
        {
            return Err(Error::InvalidMonster(format!(
                "invalid projectile reference for {}",
                a.id
            )));
        }
        if !monster.colliders.iter().any(|c| c.node_id == a.origin_node) {
            return Err(Error::InvalidMonster(format!(
                "invalid attack origin {}",
                a.origin_node
            )));
        }
        let clip = monster
            .animations
            .iter()
            .find(|clip| clip.id == a.animation_id);
        let clip_duration = monster
            .animations
            .iter()
            .find(|clip| clip.id == a.animation_id)
            .map(|clip| {
                clip.frames
                    .iter()
                    .fold(0u32, |total, f| total.saturating_add(f.duration_ms))
            })
            .unwrap_or(0);
        if !monster.animations.iter().any(|clip| {
            clip.id == a.animation_id
                && clip.events.iter().any(|e| {
                    e.kind == "ATTACK" && e.reference_id == a.id && e.time_ms == a.telegraph_ms
                })
                && clip
                    .events
                    .iter()
                    .any(|e| e.kind == "TELEGRAPH" && e.reference_id == a.id && e.time_ms == 0)
                && (a.delivery != "PROJECTILE"
                    || clip.events.iter().any(|e| {
                        e.kind == "PROJECTILE_SPAWN"
                            && e.reference_id == a.id
                            && e.time_ms == a.telegraph_ms
                    }))
                && (a.delivery != "FIELD"
                    || clip.events.iter().any(|e| {
                        e.kind == "FIELD_SPAWN"
                            && e.reference_id == a.id
                            && e.time_ms == a.telegraph_ms
                    }))
                && (a.delivery != "SUMMON"
                    || clip.events.iter().any(|e| {
                        e.kind == "MINION_SPAWN"
                            && e.reference_id == a.id
                            && e.time_ms == a.telegraph_ms
                    }))
                && (a.steps.iter().all(|step| {
                    !["DASH", "LEAP", "SWOOP", "BURROW", "TELEPORT", "DODGE"]
                        .contains(&step.primitive.as_str())
                }) || a.steps.iter().any(|step| {
                    ["SWIPE", "BITE", "THRUST", "SLAM"].contains(&step.primitive.as_str())
                        && clip.events.iter().any(|event| {
                            event.kind == "ATTACK_HIT"
                                && event.reference_id == a.id
                                && event.time_ms == step.at_ms
                        })
                }))
        }) || (a.damage == 0 && a.delivery != "SUMMON")
            || a.damage > 500
            || a.cooldown_ms < 300
            || a.cooldown_ms > 30000
            || a.telegraph_ms < 120
            || a.telegraph_ms > 5000
            || !a.range.is_finite()
            || a.range <= 0.0
            || a.range > 1000.0
            || a.steps.is_empty()
            || a.steps.len() > 32
            || a.steps.windows(2).any(|pair| pair[0].at_ms > pair[1].at_ms)
            || !a.steps.iter().any(|s| {
                [
                    "SPAWN_PROJECTILE",
                    "SPAWN_FIELD",
                    "SWIPE",
                    "BITE",
                    "THRUST",
                    "SLAM",
                    "SPAWN_MINION",
                ]
                .contains(&s.primitive.as_str())
            })
            || a.steps.iter().any(|s| {
                s.at_ms > clip_duration
                    || !s.value.is_finite()
                    || !s.spread_deg.is_finite()
                    || s.spread_deg.abs() > 120.0
                    || !s.speed.is_finite()
                    || s.speed < 0.0
                    || s.speed > 500.0
                    || !s.radius.is_finite()
                    || s.radius < 0.0
                    || s.radius > 100.0
                    || s.count > 16
                    || s.duration_ms > 10000
                    || (s.primitive == "SPAWN_PROJECTILE" && (s.count == 0 || s.speed <= 0.0))
                    || (s.primitive == "SPAWN_MINION"
                        && (a.delivery != "SUMMON"
                            || s.count == 0
                            || a.spawn.as_ref().is_none_or(|spawn| s.count != spawn.count)))
                    || ![
                        "TELEGRAPH",
                        "SPAWN_PROJECTILE",
                        "SPAWN_FIELD",
                        "SWIPE",
                        "BITE",
                        "THRUST",
                        "SLAM",
                        "SPAWN_MINION",
                        "DASH",
                        "LEAP",
                        "SWOOP",
                        "BURROW",
                        "TELEPORT",
                        "DODGE",
                        "COOLDOWN",
                    ]
                    .contains(&s.primitive.as_str())
            })
        {
            return Err(Error::InvalidMonster(format!("invalid attack {}", a.id)));
        }
        if let Some(frame) = clip.and_then(|c| c.frames.first()) {
            let sx = frame.sprite_frame_id % sprites.columns * sprites.frame_width;
            let sy = frame.sprite_frame_id / sprites.columns * sprites.frame_height;
            let visible = (sy..sy + sprites.frame_height)
                .any(|y| (sx..sx + sprites.frame_width).any(|x| sheet.get_pixel(x, y)[3] > 0));
            if !visible {
                return Err(Error::InvalidMonster(format!(
                    "invisible telegraph for {}",
                    a.id
                )));
            }
        }
    }
    if monster.projectiles.iter().any(|p| {
        p.kind.is_empty()
            || p.image_file != "projectiles.png"
            || p.frame_width == 0
            || p.frame_height == 0
            || p.columns == 0
            || p.frame_count == 0
            || p.frame_count > 32
            || p.frame_duration_ms == 0
            || !p.collision_radius.is_finite()
            || p.collision_radius <= 0.0
            || !["ALIGN_VELOCITY", "BILLBOARD"].contains(&p.orientation_mode.as_str())
            || p.first_frame.checked_add(p.frame_count).is_none()
    }) {
        return Err(Error::InvalidMonster("invalid projectile metadata".into()));
    }
    let behavior = monster
        .behavior
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing behavior".into()))?;
    let total_weight: f32 = behavior.attack_preferences.iter().map(|p| p.weight).sum();
    if ![
        behavior.aggro_range,
        behavior.preferred_range,
        behavior.aggression,
        behavior.retreat_health_fraction,
        total_weight,
    ]
    .iter()
    .all(|v| v.is_finite())
        || behavior.aggro_range <= 0.0
        || behavior.aggro_range > 2000.0
        || behavior.preferred_range < 0.0
        || behavior.preferred_range > behavior.aggro_range
        || !(0.0..=1.0).contains(&behavior.aggression)
        || !(0.0..=1.0).contains(&behavior.retreat_health_fraction)
        || (total_weight - 1.0).abs() > 0.01
        || !monster
            .movement_modes
            .iter()
            .any(|m| m.id == behavior.approach_mode)
        || !monster
            .movement_modes
            .iter()
            .any(|m| m.id == behavior.escape_mode)
        || behavior.attack_preferences.iter().any(|p| {
            !p.weight.is_finite()
                || p.weight <= 0.0
                || !monster.attacks.iter().any(|a| a.id == p.attack_id)
        })
    {
        return Err(Error::InvalidMonster("invalid behavior".into()));
    }
    let p = monster
        .physics
        .as_ref()
        .ok_or_else(|| Error::InvalidMonster("missing physics".into()))?;
    if ![
        p.mass,
        p.gravity_scale,
        p.speed,
        p.knockback_scale,
        p.center_of_mass_x,
        p.center_of_mass_y,
    ]
    .iter()
    .all(|x| x.is_finite())
        || p.mass <= 0.0
        || p.gravity_scale < 0.0
        || p.speed < 0.0
        || (p.speed == 0.0 && !monster.tags.iter().any(|tag| tag == "ANCHORED"))
        || p.knockback_scale < 0.0
    {
        return Err(Error::InvalidMonster("invalid physics".into()));
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn infernal_abi_version() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn infernal_cpu_kernel_name() -> *const c_char {
    match render3d::cpu_kernel_name() {
        "x64_v3" => c"x64_v3".as_ptr(),
        "x64_v2" => c"x64_v2".as_ptr(),
        "x64" => c"x64".as_ptr(),
        "arm64" => c"arm64".as_ptr(),
        _ => c"x86".as_ptr(),
    }
}

#[no_mangle]
/// # Safety
/// String pointers must be valid NUL-terminated UTF-8. `error_out`, when non-null,
/// must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_generate(
    prompt: *const c_char,
    seed: u64,
    output_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    infernal_generate_with_model(prompt, seed, output_dir, std::ptr::null(), error_out)
}

#[no_mangle]
/// # Safety
/// String pointers must be valid NUL-terminated UTF-8. `error_out`, when non-null,
/// must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_generate_2d(
    prompt: *const c_char,
    seed: u64,
    output_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if prompt.is_null() || output_dir.is_null() {
            return Err("null prompt or output path".to_string());
        }
        let prompt = CStr::from_ptr(prompt).to_str().map_err(|e| e.to_string())?;
        let output = CStr::from_ptr(output_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        generate_2d(prompt, seed, Path::new(output), None)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(error)) => error,
                Err(_) => "panic during 2D generation".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[no_mangle]
/// # Safety
/// String pointers must be valid NUL-terminated UTF-8. `error_out`, when non-null,
/// must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_generate_3d(
    prompt: *const c_char,
    seed: u64,
    output_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if prompt.is_null() || output_dir.is_null() {
            return Err("null prompt or output path".to_string());
        }
        let prompt = CStr::from_ptr(prompt).to_str().map_err(|e| e.to_string())?;
        let output = CStr::from_ptr(output_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        generate_3d(prompt, seed, Path::new(output), None)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(error)) => error,
                Err(_) => "panic during 3D generation".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[no_mangle]
/// # Safety
/// String pointers must be valid NUL-terminated UTF-8. `model_dir` may be null;
/// `error_out`, when non-null, must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_generate_with_model(
    prompt: *const c_char,
    seed: u64,
    output_dir: *const c_char,
    model_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if prompt.is_null() || output_dir.is_null() {
            return Err("null prompt or output path".to_string());
        }
        let prompt = CStr::from_ptr(prompt).to_str().map_err(|e| e.to_string())?;
        let output = CStr::from_ptr(output_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        let model = if model_dir.is_null() {
            None
        } else {
            Some(
                CStr::from_ptr(model_dir)
                    .to_str()
                    .map_err(|e| e.to_string())?,
            )
        };
        generate(prompt, seed, Path::new(output), model.map(Path::new))
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(e)) => e,
                Err(_) => "panic during generation".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[no_mangle]
/// # Safety
/// String pointers must be valid NUL-terminated UTF-8. `error_out`, when non-null,
/// must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_generate_spec_json(
    spec_json: *const c_char,
    seed: u64,
    output_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if spec_json.is_null() || output_dir.is_null() {
            return Err("null spec or output path".to_string());
        }
        let json = CStr::from_ptr(spec_json)
            .to_str()
            .map_err(|e| e.to_string())?;
        let path = CStr::from_ptr(output_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        let spec: parser::MonsterSpec = serde_json::from_str(json).map_err(|e| e.to_string())?;
        generate_from_spec(spec, seed, Path::new(path))
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(e)) => e,
                Err(_) => "panic during generation".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[no_mangle]
/// # Safety
/// `package_dir` must point to valid NUL-terminated UTF-8. `error_out`, when
/// non-null, must point to writable storage for one pointer.
pub unsafe extern "C" fn infernal_validate_package(
    package_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if package_dir.is_null() {
            return Err("null package path".to_string());
        }
        let path = CStr::from_ptr(package_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        load_package(Path::new(path))
            .map(|_| ())
            .map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(e)) => e,
                Err(_) => "panic while validating package".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[no_mangle]
/// # Safety
/// `value` must be null or a pointer returned by this library as an error string,
/// and must be freed only once.
pub unsafe extern "C" fn infernal_free_string(value: *mut c_char) {
    if !value.is_null() {
        drop(CString::from_raw(value));
    }
}

/// Opaque in-memory package; returned data remains valid until this handle is freed.
#[no_mangle]
/// # Safety
/// `prompt` must be a valid NUL-terminated UTF-8 string and `error_out` must
/// point to writable pointer storage when non-null.
pub unsafe extern "C" fn infernal_generate_object(
    prompt: *const c_char,
    seed: u64,
    error_out: *mut *mut c_char,
) -> *mut GeneratedMonster {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let result = std::panic::catch_unwind(|| {
        if prompt.is_null() {
            return Err("null prompt".to_string());
        }
        let text = CStr::from_ptr(prompt).to_str().map_err(|e| e.to_string())?;
        generate_in_memory(text, seed).map_err(|e| e.to_string())
    });
    match result {
        Ok(Ok(value)) => Box::into_raw(Box::new(value)),
        other => {
            let message = match other {
                Ok(Err(e)) => e,
                Err(_) => "panic during generation".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
/// # Safety
/// `handle` must be null or returned by `infernal_generate_object`, and freed once.
pub unsafe extern "C" fn infernal_object_free(handle: *mut GeneratedMonster) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

#[no_mangle]
/// # Safety
/// `handle` must be a live `infernal_generate_object` result.
pub unsafe extern "C" fn infernal_object_count(handle: *const GeneratedMonster) -> u32 {
    if handle.is_null() {
        0
    } else {
        (*handle).packages.len() as u32
    }
}

#[no_mangle]
/// # Safety
/// `handle` must be live. The returned string must be freed with `infernal_free_string`.
pub unsafe extern "C" fn infernal_object_package_path(
    handle: *const GeneratedMonster,
    index: u32,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    (&*handle)
        .packages
        .get(index as usize)
        .and_then(|p| CString::new(p.path.to_string_lossy().as_bytes()).ok())
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
/// # Safety
/// `handle` and all output pointers must be valid. The returned pixel pointer
/// is borrowed from the handle and must be copied before freeing it.
pub unsafe extern "C" fn infernal_object_pixels(
    handle: *const GeneratedMonster,
    index: u32,
    kind: u32,
    pixels_out: *mut *const u8,
    len_out: *mut usize,
    width_out: *mut u32,
    height_out: *mut u32,
) -> i32 {
    if handle.is_null()
        || pixels_out.is_null()
        || len_out.is_null()
        || width_out.is_null()
        || height_out.is_null()
    {
        return -1;
    }
    let Some(package) = (&*handle).packages.get(index as usize) else {
        return -1;
    };
    let image = match kind {
        0 => Some(&package.sprites),
        1 => Some(&package.emission),
        2 => package.projectiles.as_ref(),
        _ => None,
    };
    let Some(image) = image else {
        return -1;
    };
    *pixels_out = image.as_raw().as_ptr();
    *len_out = image.as_raw().len();
    *width_out = image.width();
    *height_out = image.height();
    0
}

/// Event codes: 0 null, 1 object start, 2 object end, 3 array start,
/// 4 array end, 5 string, 6 integer, 7 float, 8 boolean.
pub type ObjectVisitor =
    unsafe extern "C" fn(*mut std::ffi::c_void, u32, *const c_char, *const c_char, i64, f64);

fn visit_value(
    value: &serde_json::Value,
    name: Option<&str>,
    context: *mut std::ffi::c_void,
    callback: ObjectVisitor,
) {
    use serde_json::Value;
    let key = name.and_then(|name| CString::new(name).ok());
    let key_ptr = key
        .as_ref()
        .map_or(std::ptr::null(), |value| value.as_ptr());
    let emit = |kind, string: *const c_char, integer, real| unsafe {
        callback(context, kind, key_ptr, string, integer, real)
    };
    match value {
        Value::Null => emit(0, std::ptr::null(), 0, 0.0),
        Value::Object(fields) => {
            emit(1, std::ptr::null(), 0, 0.0);
            for (key, child) in fields {
                visit_value(child, Some(key), context, callback);
            }
            emit(2, std::ptr::null(), 0, 0.0);
        }
        Value::Array(items) => {
            emit(3, std::ptr::null(), 0, 0.0);
            for child in items {
                visit_value(child, None, context, callback);
            }
            emit(4, std::ptr::null(), 0, 0.0);
        }
        Value::String(text) => {
            if let Ok(text) = CString::new(text.as_str()) {
                emit(5, text.as_ptr(), 0, 0.0);
            }
        }
        Value::Number(number) => {
            if let Some(value) = number.as_i64() {
                emit(6, std::ptr::null(), value, 0.0);
            } else if let Some(value) = number.as_u64() {
                emit(6, std::ptr::null(), value as i64, 0.0);
            } else if let Some(value) = number.as_f64() {
                emit(7, std::ptr::null(), 0, value);
            }
        }
        Value::Bool(value) => emit(8, std::ptr::null(), i64::from(*value), 0.0),
    }
}

#[no_mangle]
/// # Safety
/// `handle` must be live. `context` must be valid for `callback` for the
/// duration of the call. Callback pointers are borrowed only for that call.
pub unsafe extern "C" fn infernal_object_visit(
    handle: *const GeneratedMonster,
    index: u32,
    context: *mut std::ffi::c_void,
    callback: Option<ObjectVisitor>,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let Some(package) = (&*handle).packages.get(index as usize) else {
        return -1;
    };
    let Some(callback) = callback else {
        return -1;
    };
    let Ok(value) = serde_json::to_value(&package.monster) else {
        return -1;
    };
    visit_value(&value, None, context, callback);
    0
}

#[no_mangle]
/// # Safety
/// `handle` must be live; `output_dir` must be a valid NUL-terminated UTF-8 path.
/// `error_out` must point to writable pointer storage when non-null.
pub unsafe extern "C" fn infernal_object_save(
    handle: *const GeneratedMonster,
    output_dir: *const c_char,
    error_out: *mut *mut c_char,
) -> i32 {
    if !error_out.is_null() {
        *error_out = std::ptr::null_mut();
    }
    let outcome = std::panic::catch_unwind(|| {
        if handle.is_null() || output_dir.is_null() {
            return Err("null object or path".to_string());
        }
        let path = CStr::from_ptr(output_dir)
            .to_str()
            .map_err(|e| e.to_string())?;
        save_generated(&*handle, Path::new(path)).map_err(|e| e.to_string())
    });
    match outcome {
        Ok(Ok(())) => 0,
        other => {
            let message = match other {
                Ok(Err(e)) => e,
                Err(_) => "panic while saving object".into(),
                _ => unreachable!(),
            };
            if !error_out.is_null() {
                *error_out = CString::new(message).unwrap_or_default().into_raw();
            }
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_object_round_trip() {
        unsafe extern "C" fn count_fields(
            context: *mut std::ffi::c_void,
            _kind: u32,
            key: *const c_char,
            _string: *const c_char,
            _integer: i64,
            _real: f64,
        ) {
            if !key.is_null() && CStr::from_ptr(key).to_bytes() == b"display_name" {
                *(context as *mut usize) += 1;
            }
        }
        let prompt = CString::new("wolf").unwrap();
        let mut error = std::ptr::null_mut();
        let object = unsafe { infernal_generate_object(prompt.as_ptr(), 42, &mut error) };
        assert!(error.is_null());
        assert!(!object.is_null());
        assert_eq!(unsafe { infernal_object_count(object) }, 1);
        let mut fields = 0usize;
        assert_eq!(
            unsafe {
                infernal_object_visit(
                    object,
                    0,
                    &mut fields as *mut _ as *mut _,
                    Some(count_fields),
                )
            },
            0
        );
        assert_eq!(fields, 1);
        let (mut pixels, mut len, mut width, mut height) = (std::ptr::null(), 0, 0, 0);
        assert_eq!(
            unsafe {
                infernal_object_pixels(object, 0, 0, &mut pixels, &mut len, &mut width, &mut height)
            },
            0
        );
        assert_eq!(len, width as usize * height as usize * 4);
        assert!(!pixels.is_null());
        let saved = tempfile::tempdir().unwrap();
        let destination = CString::new(saved.path().to_str().unwrap()).unwrap();
        assert_eq!(
            unsafe { infernal_object_save(object, destination.as_ptr(), &mut error) },
            0
        );
        assert!(error.is_null());
        assert!(!load_package(saved.path()).unwrap().id.is_empty());
        unsafe { infernal_object_free(object) };
    }

    #[test]
    fn deterministic_complete_monster() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let one = generate("giant spider that shoots fire", 42, a.path(), None).unwrap();
        let two = generate("giant spider that shoots fire", 42, b.path(), None).unwrap();
        assert_eq!(one, two);
        assert_eq!(
            std::fs::read(a.path().join("sprites.png")).unwrap(),
            std::fs::read(b.path().join("sprites.png")).unwrap()
        );
        assert_eq!(
            std::fs::read(a.path().join("emission.png")).unwrap(),
            std::fs::read(b.path().join("emission.png")).unwrap()
        );
        let emission = image::open(a.path().join("emission.png"))
            .unwrap()
            .into_rgba8();
        assert_eq!(
            emission.width(),
            image::open(a.path().join("sprites.png")).unwrap().width()
        );
        assert_eq!(one.attacks[0].origin_node, "mouth_0");
        let decoded = proto::Monster::decode(
            std::fs::read(a.path().join("monster.pb"))
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        assert_eq!(decoded, one);
        assert_eq!(load_package(a.path()).unwrap(), one);
        #[cfg(feature = "bert")]
        assert!(one
            .generation
            .as_ref()
            .unwrap()
            .parser
            .starts_with("candle-bert"));
        let sprites = one.sprites.as_ref().unwrap();
        assert!(sprites.palette_size < 256);
        let sheet = image::open(a.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        assert_eq!(one.format_version, 7);
        assert_eq!(sprites.frame_width, 128);
        assert!(sheet.pixels().any(|p| p[3] > 0));
        assert_eq!(one.projectiles[0].kind, "FIREBALL");
        assert_eq!(one.attacks[0].projectile_id, one.projectiles[0].id);
        let projectile_sheet = image::open(a.path().join("projectiles.png"))
            .unwrap()
            .into_rgba8();
        assert!(projectile_sheet
            .pixels()
            .any(|p| p[0] > 240 && p[1] > 200 && p[2] < 150 && p[3] > 0));
        std::fs::write(a.path().join("projectiles.png"), b"corrupt").unwrap();
        assert!(load_package(a.path()).is_err());
    }
    #[test]
    fn modern_vocabulary_requires_explicit_prompt() {
        let a = parser::parse_vocabulary("skeletal jackal with a shotgun");
        let b = parser::parse_vocabulary("skeletal jackal");
        assert_eq!(a.attack.concept, "SHOTGUN");
        assert_eq!(b.attack.concept, "MELEE");
    }
    #[test]
    fn archetype_recipes_bind_anatomy_and_movement() {
        use rand::SeedableRng;
        for (prompt, movement, part, moving_part) in [
            (
                "six armed cyclops with a sword",
                "BIPED_WALK",
                "hand_5",
                "hand_5",
            ),
            (
                "three headed hydra",
                "SERPENTINE_SLITHER",
                "head_2",
                "tail_3",
            ),
            ("giant dragon", "QUADRUPED_WALK", "wing_1", "wing_1"),
            ("snail", "SNAIL_CRAWL", "shell", "sole"),
            ("slug", "GLIDE", "sole", "sole"),
            ("slime blob", "OOZE_CRAWL", "lobe_4", "lobe_4"),
        ] {
            let spec = parser::parse_vocabulary(prompt);
            let mut rng = ChaCha8Rng::seed_from_u64(7);
            let body = anatomy::build(&spec, &mut rng);
            validate_body(&body).unwrap();
            assert!(body.nodes.iter().any(|node| node.id == part), "{prompt}");
            let (_, modes, _, poses) = animation::make(&spec, &body);
            assert_eq!(modes[0].id, movement, "{prompt}");
            let index = body
                .nodes
                .iter()
                .position(|node| node.id == moving_part)
                .unwrap();
            let moving: Vec<_> = poses.iter().filter(|pose| pose.clip_id == "move").collect();
            let first = render::transform(&body.nodes[index], index, moving[1], body.gravity);
            let later = render::transform(&body.nodes[index], index, moving[3], body.gravity);
            assert!(
                (first.0 - later.0).abs() + (first.1 - later.1).abs() + (first.2 - later.2).abs()
                    > 0.1,
                "{prompt}: {moving_part} did not move"
            );
        }
    }
    #[test]
    fn projectile_mapper_bakes_distinct_art() {
        for (prompt, expected) in [
            ("spider that shoots fire", "FIREBALL"),
            ("blob that spits acid", "SLUDGE"),
            ("archer with a bow", "ARROW"),
            ("cyclops with a shotgun", "BULLET"),
        ] {
            let spec = parser::parse_vocabulary(prompt);
            let mut rng = ChaCha8Rng::seed_from_u64(9);
            let body = anatomy::build(&spec, &mut rng);
            let (_, _, mut attacks, _) = animation::make(&spec, &body);
            let (assets, atlas) = projectile::bake(&spec, &mut attacks, 9);
            assert_eq!(assets[0].kind, expected, "{prompt}");
            assert_eq!(attacks[0].projectile_id, assets[0].id);
            assert!(projectile::valid_atlas(&assets, &atlas.unwrap()));
        }
    }
    #[test]
    fn explicit_arms_do_not_replace_walking_limbs() {
        let spec = parser::parse_vocabulary("translucent crocodile demon with six arms");
        assert_eq!(spec.limb_count, 4);
        assert_eq!(spec.arm_count, 6);
        let dir = tempfile::tempdir().unwrap();
        let monster = generate(
            "translucent crocodile demon with six arms",
            4,
            dir.path(),
            None,
        )
        .unwrap();
        assert!(monster.colliders.iter().any(|c| c.node_id == "hand_5"));
        assert!(monster.colliders.iter().any(|c| c.node_id == "foot_3"));
    }
    #[test]
    fn laser_eyes_use_generated_eye() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("sphinx with laser eyes", 9, dir.path(), None).unwrap();
        assert_eq!(monster.attacks[0].origin_node, "eye_0");
        assert!(!monster.colliders.iter().any(|c| c.node_id == "weapon"));
    }
    #[test]
    fn typed_spec_and_handcrafted_export_share_runtime_format() {
        let spec = parser::parse_vocabulary("cyclops with a shotgun");
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let monster = generate_from_spec(spec, 13, a.path()).unwrap();
        assert_eq!(monster.format_version, 7);
        assert_eq!(monster.sprites.as_ref().unwrap().frame_width, 96);
        let sheet = image::open(a.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        let projectile_sheet = image::open(a.path().join("projectiles.png"))
            .unwrap()
            .into_rgba8();
        export_handcrafted_with_projectiles(&monster, &sheet, Some(&projectile_sheet), b.path())
            .unwrap();
        assert_eq!(
            std::fs::read(a.path().join("monster.pb")).unwrap(),
            std::fs::read(b.path().join("monster.pb")).unwrap()
        );
    }
    #[test]
    fn spectral_alpha_survives_sprite_baking() {
        let dir = tempfile::tempdir().unwrap();
        generate("spectral ghost", 5, dir.path(), None).unwrap();
        let sheet = image::open(dir.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        assert!(sheet.pixels().any(|p| p[3] > 0 && p[3] < 255));
    }
    #[test]
    fn movement_modes_have_matching_clips() {
        for prompt in ["winged demon", "burrowing scarab", "giant spider"] {
            let dir = tempfile::tempdir().unwrap();
            let monster = generate(prompt, 17, dir.path(), None).unwrap();
            for mode in &monster.movement_modes {
                assert!(
                    monster
                        .animations
                        .iter()
                        .any(|clip| clip.id == mode.animation_id && clip.movement_mode == mode.id),
                    "{prompt}: {}",
                    mode.id
                );
            }
            if prompt == "winged demon" {
                assert!(monster.movement_modes.iter().any(|m| m.id == "FLY"));
            }
            if prompt == "burrowing scarab" {
                assert!(monster
                    .animations
                    .iter()
                    .any(|a| a.semantic_state == "EMERGE"));
            }
        }
    }
    #[test]
    fn scorpion_has_anatomy_linked_poison_sting() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("scorpion", 21, dir.path(), None).unwrap();
        let secondary = monster
            .attacks
            .iter()
            .find(|a| a.id == "secondary")
            .unwrap();
        assert_eq!(secondary.origin_node, "stinger");
        assert_eq!(secondary.element, "POISON");
        assert!(secondary.steps.iter().any(|s| s.primitive == "THRUST"));
    }
    #[test]
    fn elemental_special_has_bounded_field_and_clip() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("giant spider that shoots fire", 42, dir.path(), None).unwrap();
        let special = monster.attacks.iter().find(|a| a.id == "special").unwrap();
        assert!(special
            .steps
            .iter()
            .any(|s| s.primitive == "SPAWN_FIELD" && s.radius <= 100.0));
        assert!(monster
            .animations
            .iter()
            .any(|a| a.id == special.animation_id && a.semantic_state == "SPECIAL_ATTACK"));
    }
    #[test]
    fn invalid_attack_origin_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut m = generate("cyclops with a shotgun", 19, dir.path(), None).unwrap();
        m.attacks[0].origin_node = "missing_weapon".into();
        let sheet = image::open(dir.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        assert!(validate(&m, &sheet).is_err());
        m.attacks[0].origin_node = "weapon".into();
        m.attacks[0].projectile_id = "missing_projectile".into();
        assert!(validate(&m, &sheet).is_err());
    }
    #[test]
    fn invalid_behavior_reference_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let mut monster = generate("spider", 6, dir.path(), None).unwrap();
        monster.behavior.as_mut().unwrap().attack_preferences[0].attack_id = "missing".into();
        let sheet = image::open(dir.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        assert!(validate(&monster, &sheet).is_err());
    }
    #[test]
    fn corrupt_runtime_metadata_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("winged demon", 11, dir.path(), None).unwrap();
        let sheet = image::open(dir.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        let mut duplicate = monster.clone();
        duplicate.animations[1].id = duplicate.animations[0].id.clone();
        assert!(validate(&duplicate, &sheet).is_err());
        let mut nan = monster.clone();
        nan.generation.as_mut().unwrap().parser_confidence = f32::NAN;
        assert!(validate(&nan, &sheet).is_err());
        let mut missing = monster.clone();
        missing
            .animations
            .iter_mut()
            .find(|a| a.id == "death")
            .unwrap()
            .events
            .clear();
        assert!(validate(&missing, &sheet).is_err());
    }
    #[test]
    fn three_dimensional_bake_has_distinct_valid_directions() {
        let dir = tempfile::tempdir().unwrap();
        let spec = parser::parse_vocabulary("giant spider that shoots fire");
        let duplicate_dir = tempfile::tempdir().unwrap();
        let monster = generate_from_spec_3d(spec.clone(), 7, dir.path()).unwrap();
        let duplicate = generate_from_spec_3d(spec, 7, duplicate_dir.path()).unwrap();
        assert_eq!(monster, duplicate);
        assert_eq!(
            std::fs::read(dir.path().join("sprites.png")).unwrap(),
            std::fs::read(duplicate_dir.path().join("sprites.png")).unwrap()
        );
        assert_eq!(monster.format_version, 7);
        let sprites = monster.sprites.as_ref().unwrap();
        assert_eq!(sprites.frame_width, 128);
        assert_eq!(sprites.frame_height, 128);
        assert_eq!(sprites.direction_angles_deg.len(), 4);
        assert_eq!(sprites.frame_count, sprites.direction_stride * 4);
        assert!(monster.colliders.iter().all(|c| c.views.len() == 4));
        assert!(dir.path().join("preview.png").exists());
        assert_eq!(load_package(dir.path()).unwrap(), monster);
        let sheet = image::open(dir.path().join("sprites.png"))
            .unwrap()
            .into_rgba8();
        let frame_pixels = |frame_id: u32| {
            let ox = frame_id % sprites.columns * sprites.frame_width;
            let oy = frame_id / sprites.columns * sprites.frame_height;
            let mut pixels = Vec::new();
            for y in 0..sprites.frame_height {
                for x in 0..sprites.frame_width {
                    pixels.push(sheet.get_pixel(ox + x, oy + y).0);
                }
            }
            pixels
        };
        assert_ne!(frame_pixels(0), frame_pixels(sprites.direction_stride));
        let mut invalid = monster.clone();
        invalid.sprites.as_mut().unwrap().direction_stride = 1;
        assert!(validate(&invalid, &sheet).is_err());
    }

    #[test]
    fn many_attack_patterns_export_normalized_behavior() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate_from_spec_3d(
            parser::parse_vocabulary("dragon that shoots fire"),
            51,
            dir.path(),
        )
        .unwrap();
        assert!(monster.attacks.len() >= 5);
        assert!(monster
            .attacks
            .iter()
            .any(|attack| attack.steps.iter().any(|step| step.primitive == "SWOOP")));
        let preferences = &monster.behavior.as_ref().unwrap().attack_preferences;
        assert_eq!(preferences.len(), monster.attacks.len());
        let weight: f32 = preferences.iter().map(|preference| preference.weight).sum();
        assert!((weight - 1.0).abs() < 0.001);
        assert_eq!(load_package(dir.path()).unwrap(), monster);
    }

    #[test]
    fn mounted_monster_exports_playable_survivor() {
        let dir = tempfile::tempdir().unwrap();
        let monster =
            generate("goblin riding a wolf; mount survives", 17, dir.path(), None).unwrap();
        assert_eq!(monster.mount.as_ref().unwrap().survivor, "MOUNT");
        assert_eq!(monster.format_version, 7);
        assert!(monster.tags.contains(&"MOUNTED".into()));
        let mount = load_package(&dir.path().join("companions/mount")).unwrap();
        assert!(mount.gameplay.as_ref().unwrap().health > 0);
        assert!(mount.mount.is_none());
        assert_eq!(load_package(dir.path()).unwrap(), monster);
    }

    #[test]
    fn unspecified_mounted_survivor_is_seeded_and_described() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("orc riding a bear", 12, dir.path(), None).unwrap();
        let survivor = monster.mount.as_ref().unwrap().survivor.to_lowercase();
        assert!(["rider", "mount"].contains(&survivor.as_str()));
        assert!(monster
            .description
            .contains(&format!("the {survivor} continues fighting")));
        assert_eq!(load_package(dir.path()).unwrap(), monster);
    }

    #[test]
    fn spore_summoner_exports_minions_and_projectile_art() {
        let dir = tempfile::tempdir().unwrap();
        let monster = generate("mushroom summons hornets", 18, dir.path(), None).unwrap();
        assert!(monster.tags.contains(&"SUMMONER".into()));
        let summon = monster
            .attacks
            .iter()
            .find(|a| a.delivery == "SUMMON")
            .unwrap();
        let child = load_package(&dir.path().join("companions/minion")).unwrap();
        assert_eq!(summon.spawn.as_ref().unwrap().monster_id, child.id);
        assert!(monster.projectiles.iter().any(|p| p.kind == "SPORE_CLOUD"));
        assert_eq!(load_package(dir.path()).unwrap(), monster);
    }
    #[test]
    fn sharknado_exports_whirlwind_and_playable_shark_minions() {
        let dir = tempfile::tempdir().unwrap();
        let spec = parser::parse_vocabulary("sharknado");
        let mut rng = ChaCha8Rng::seed_from_u64(17);
        let body = anatomy::build(&spec, &mut rng);
        assert!(body.nodes.iter().any(|node| node.kind == "VORTEX"));
        assert!(body.nodes.iter().any(|node| node.kind == "FIN"));
        let monster = generate_from_spec_3d(spec, 17, dir.path()).unwrap();
        assert!(monster.attacks.iter().any(|attack| attack.spawn.is_some()));
        let child = load_package(&dir.path().join("companions/minion")).unwrap();
        assert!(child.attacks.len() >= 2);
        assert!(monster.sprites.as_ref().unwrap().palette_size < 256);
        assert_eq!(load_package(dir.path()).unwrap(), monster);
    }
    #[test]
    fn corrupt_package_is_rejected_on_load() {
        let dir = tempfile::tempdir().unwrap();
        generate("skeletal jackal", 3, dir.path(), None).unwrap();
        std::fs::write(dir.path().join("monster.pb"), b"broken").unwrap();
        assert!(load_package(dir.path()).is_err());
    }
}
