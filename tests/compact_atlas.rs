use image::RgbaImage;
use infernal_diffusion::{atlas, generate_in_memory_with_profile, GenerationProfile};

fn expand(p: &infernal_diffusion::GeneratedPackage, emission: bool) -> RgbaImage {
    let s = p.monster.sprites.as_ref().unwrap();
    let mut output = RgbaImage::new(
        s.columns * s.frame_width,
        s.frame_count.div_ceil(s.columns) * s.frame_height,
    );
    if emission && s.emission_file.is_empty() {
        return output;
    }
    let source = if emission { &p.emission } else { &p.sprites };
    for (id, r) in s.frame_regions.iter().enumerate() {
        let frame = image::imageops::crop_imm(source, r.x, r.y, r.width, r.height).to_image();
        image::imageops::replace(
            &mut output,
            &frame,
            i64::from(id as u32 % s.columns * s.frame_width + r.offset_x),
            i64::from(id as u32 / s.columns * s.frame_height + r.offset_y),
        );
    }
    output
}

#[test]
fn packed_frames_preserve_every_pixel_and_special_attack() {
    for profile in [GenerationProfile::V1, GenerationProfile::V2] {
        for prompt in [
            "small ice slime",
            "shadow wraith",
            "orc with a sword that shoots fire",
        ] {
            let mut generated = generate_in_memory_with_profile(prompt, 42, profile).unwrap();
            let originals: Vec<_> = generated
                .packages
                .iter()
                .map(|p| (p.monster.clone(), p.sprites.clone(), p.emission.clone()))
                .collect();
            atlas::compact(&mut generated).unwrap();
            for (package, (monster, sprites, emission)) in generated.packages.iter().zip(originals)
            {
                assert_eq!(
                    expand(package, false),
                    sprites,
                    "{prompt}: every logical color pixel"
                );
                assert_eq!(
                    expand(package, true),
                    emission,
                    "{prompt}: every logical emission pixel"
                );
                assert_eq!(
                    package.monster.animations, monster.animations,
                    "all clips, timings and animation events"
                );
                assert_eq!(
                    package.monster.attacks, monster.attacks,
                    "all special attacks and hit/launch events"
                );
                assert_eq!(package.monster.colliders, monster.colliders);
                assert_eq!(package.monster.size, monster.size);
                assert_eq!(
                    package.monster.v2_profile, monster.v2_profile,
                    "V2 poses, origins and hurtboxes"
                );
                let mut restored = package.monster.clone();
                restored.sprites = monster.sprites.clone();
                restored.format_version = monster.format_version;
                assert_eq!(restored, monster, "all non-storage metadata");
                assert!(package.sprites.as_raw().len() < sprites.as_raw().len());
                if package
                    .monster
                    .sprites
                    .as_ref()
                    .unwrap()
                    .emission_file
                    .is_empty()
                {
                    assert_eq!(package.emission.dimensions(), (1, 1));
                }
                infernal_diffusion::validate(&package.monster, &package.sprites).unwrap();
            }
            let before = generated.packages[0].sprites.clone();
            atlas::compact(&mut generated).unwrap();
            assert_eq!(
                before, generated.packages[0].sprites,
                "idempotent compaction"
            );
            let directory = tempfile::tempdir().unwrap();
            infernal_diffusion::save_generated(&generated, directory.path()).unwrap();
            infernal_diffusion::load_package(directory.path()).unwrap();
        }
    }
}

#[test]
fn packed_regions_reject_overflow_missing_and_out_of_frame() {
    let mut s = infernal_diffusion::proto::SpriteSet {
        frame_count: 1,
        frame_width: 8,
        frame_height: 8,
        frame_regions: vec![infernal_diffusion::proto::FrameRegion {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
            offset_x: 3,
            offset_y: 4,
        }],
        ..Default::default()
    };
    assert!(atlas::valid_regions(&s, 4, 4));
    s.frame_regions[0].x = u32::MAX;
    assert!(!atlas::valid_regions(&s, 4, 4));
    s.frame_regions[0].x = 1;
    s.frame_regions[0].offset_y = 7;
    assert!(!atlas::valid_regions(&s, 4, 4));
    s.frame_regions.clear();
    assert!(!atlas::valid_regions(&s, 4, 4));
}

#[test]
fn early_compaction_matches_postprocessing_and_hybrids_keep_attacks() {
    use infernal_diffusion::v2::hybrid::{HeadKind, HybridRecipe, LowerBody, UpperBody};
    let recipe = HybridRecipe {
        schema_version: 1,
        upper_body: UpperBody::Stormtrooper,
        lower_body: LowerBody::Spider,
        head: HeadKind::Fish,
        lower_limb_count: 8,
        upper_arm_count: 2,
    };
    let mut dense = infernal_diffusion::generate_hybrid_in_memory(&recipe, 42).unwrap();
    let sprites = dense.packages[0].sprites.clone();
    let before = dense.packages[0].monster.clone();
    atlas::compact(&mut dense).unwrap();
    let early = infernal_diffusion::generate_hybrid_in_memory_compact(&recipe, 42).unwrap();
    assert_eq!(early.packages[0].monster, dense.packages[0].monster);
    assert_eq!(early.packages[0].sprites, dense.packages[0].sprites);
    assert_eq!(expand(&early.packages[0], false), sprites);
    assert_eq!(early.packages[0].monster.attacks, before.attacks);
    assert_eq!(early.packages[0].monster.animations, before.animations);
    assert_eq!(early.packages[0].monster.v2_profile, before.v2_profile);
    let mut dense =
        generate_in_memory_with_profile("shade humanoid", 42, GenerationProfile::V2).unwrap();
    atlas::compact(&mut dense).unwrap();
    let early = infernal_diffusion::generate_in_memory_compact_with_profile(
        "shade humanoid",
        42,
        GenerationProfile::V2,
    )
    .unwrap();
    assert_eq!(early.packages[0].sprites, dense.packages[0].sprites);
    assert_eq!(early.packages[0].emission, dense.packages[0].emission);
    assert_eq!(early.packages[0].monster, dense.packages[0].monster);
}
