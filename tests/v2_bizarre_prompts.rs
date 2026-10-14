//! Exact public prompt-path regressions. Run with default features (embedded
//! BERT) and --no-default-features (vocabulary); no hand-authored spec substitute.
use infernal_diffusion::{generate_with_profile, load_package, GenerationProfile, PackageFormat};
use serde_json::json;

#[test]
fn exact_bizarre_prompts_preserve_parts_materials_and_mounts() {
    let temporary = tempfile::tempdir().unwrap();
    let root = std::env::var_os("INFERNAL_BIZARRE_EVIDENCE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| temporary.path().to_owned());
    let configuration = if cfg!(feature = "bert") {
        "embedded_bert"
    } else {
        "vocabulary"
    };
    let root = root.join(configuration);
    std::fs::create_dir_all(&root).unwrap();
    let mut records = Vec::new();
    for (name, prompt, seed) in [
        (
            "skeleton_mermaid_eight_arms",
            "skeleton mermaid with eight arms",
            42,
        ),
        (
            "ghost_lion_orc_rider",
            "Ghost lion with orc rider with huge axe",
            17,
        ),
    ] {
        let output = root.join(name);
        let generated = generate_with_profile(
            prompt,
            seed,
            &output,
            None,
            PackageFormat::Protobuf,
            GenerationProfile::V2,
        );
        match generated {
            Err(error)=>records.push(json!({"case":name,"prompt":prompt,"seed":seed,"configuration":configuration,"supported":false,"pass":false,"error":error.to_string()})),
            Ok(monster)=>{
                let source_preserved=monster.generation.as_ref().is_some_and(|g|g.prompt==prompt && g.seed==seed);
                let ids:Vec<_>=monster.colliders.iter().map(|c|c.node_id.as_str()).collect();
                let (checks,companions)=if name=="skeleton_mermaid_eight_arms" {
                    (json!({"source_preserved":source_preserved,"eight_hands":ids.iter().filter(|id|id.starts_with("hand_")).count()==8,
                        "mermaid_identity":monster.tags.iter().any(|t|t=="MERMAID"),"bone_material":monster.tags.iter().any(|t|t=="BONE"),
                        "fish_tail":ids.iter().any(|id|id.starts_with("v2_fish_tail_")),"no_humanoid_feet":!ids.iter().any(|id|id.starts_with("foot_")),
                        "no_unrequested_hybrid_head":!ids.iter().any(|id|id.starts_with("hybrid_head_"))}),json!(null))
                } else {
                    let rider=monster.mount.as_ref().and_then(|m|load_package(&output.join(&m.rider_package)).ok());
                    let mount=monster.mount.as_ref().and_then(|m|load_package(&output.join(&m.mount_package)).ok());
                    (json!({"source_preserved":source_preserved,"mounted":monster.mount.is_some(),
                        "distinct_torsos":ids.contains(&"torso")&&ids.contains(&"rider_torso"),
                        "rider_weapon":ids.contains(&"rider_weapon"),"rider_weapon_attack_origin":monster.attacks.iter().any(|a|a.origin_node=="rider_weapon"),
                        "orc_rider":rider.as_ref().is_some_and(|m|m.tags.iter().any(|t|t=="ORC")),
                        "rider_not_ghost":rider.as_ref().is_some_and(|m|!m.tags.iter().any(|t|t=="GHOST")),
                        "companion_rider_arm_count_matches":rider.as_ref().is_some_and(|m|m.colliders.iter().filter(|c|c.node_id.starts_with("hand_")).count()==ids.iter().filter(|id|id.starts_with("rider_hand_")).count()),
                        "huge_axe_intent":infernal_diffusion::v2::prompt_normalization::normalize_mount(prompt).is_some_and(|s|s.attack.concept=="AXE" && s.part_scales.get("WEAPON")==Some(&1.8)),
                        "lion_mount":mount.as_ref().is_some_and(|m|m.tags.iter().any(|t|t=="LION")),
                        "spectral_mount":mount.as_ref().is_some_and(|m|m.tags.iter().any(|t|t=="GHOST"||t=="SPECTRAL")),
                        "no_blended_orc_head":!ids.iter().any(|id|id.starts_with("hybrid_head_"))}),
                    json!({"rider_tags":rider.as_ref().map(|m|&m.tags),"mount_tags":mount.as_ref().map(|m|&m.tags)}))
                };
                let pass=checks.as_object().unwrap().values().all(|v|v==true);
                std::fs::write(output.join("metadata.json"),serde_json::to_vec_pretty(&monster).unwrap()).unwrap();
                let record=json!({"case":name,"prompt":prompt,"seed":seed,"configuration":configuration,
                    "actual_parser":monster.generation.as_ref().map(|g|&g.parser),"supported":pass,"pass":pass,
                    "checks":checks,"tags":monster.tags,"collider_ids":ids,"companions":companions,
                    "package":output,"preview":output.join("preview.png"),"sprites":output.join("sprites.png")});
                std::fs::write(output.join("evidence.json"),serde_json::to_vec_pretty(&record).unwrap()).unwrap();
                records.push(record);
            }
        }
    }
    std::fs::write(
        root.join("results.json"),
        serde_json::to_vec_pretty(&records).unwrap(),
    )
    .unwrap();
    assert!(
        records.iter().all(|r| r["pass"] == true),
        "exact prompt failures: {}",
        serde_json::to_string_pretty(&records).unwrap()
    );
}

#[test]
fn contradictory_modifiers_and_impossible_composition_counts_are_rejected() {
    let output = tempfile::tempdir().unwrap();
    let mut evidence = Vec::new();
    for (index, prompt) in [
        "shade ghost lion",
        "13-legged spider-centaur",
        "stormtrooper top on mermaid bottom with two legs",
        "spider-centaur with nine arms",
    ]
    .into_iter()
    .enumerate()
    {
        let result = generate_with_profile(
            prompt,
            73,
            &output.path().join(index.to_string()),
            None,
            PackageFormat::Protobuf,
            GenerationProfile::V2,
        );
        evidence.push(json!({"prompt":prompt,"seed":73,"expected":"rejected","pass":result.is_err(),"error":result.err().map(|e|e.to_string())}));
    }
    if let Some(root) = std::env::var_os("INFERNAL_BIZARRE_EVIDENCE") {
        let root = std::path::PathBuf::from(root).join(if cfg!(feature = "bert") {
            "embedded_bert"
        } else {
            "vocabulary"
        });
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("rejection_results.json"),
            serde_json::to_vec_pretty(&evidence).unwrap(),
        )
        .unwrap();
    }
    assert!(
        evidence.iter().all(|r| r["pass"] == true),
        "{}",
        serde_json::to_string_pretty(&evidence).unwrap()
    );
}
