//! Static, Y-up imported meshes baked by the same CPU rasterizer as monsters.
//! No anatomy, animation, collision, navigation or terrain-seam inference.
pub mod ffi;
pub mod import;
use crate::{
    render,
    render3d::{self, Camera, Triangle, V3},
};
use image::RgbaImage;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read, path::Path};

#[derive(Debug, thiserror::Error)]
pub enum PropError {
    #[error("invalid prop: {0}")]
    Invalid(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropMesh {
    pub vertices: Vec<[f32; 3]>,
    pub faces: Vec<PropFace>,
    pub materials: Vec<PropMaterial>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropFace {
    pub indices: [usize; 3],
    pub material: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropMaterial {
    pub name: String,
    pub color: [u8; 4],
    pub surface: String,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Framing {
    #[default]
    Expand,
    Fixed,
    Fit,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PropOptions {
    pub tile_width: u32,
    pub tile_height: u32,
    pub pixels_per_unit: f32,
    pub world_scale: f32,
    /// Source-space pivot. None places the bottom-center of source bounds at the anchor.
    pub pivot: Option<[f32; 3]>,
    /// Fractional tile position of the pivot, measured from top-left.
    pub anchor: [f32; 2],
    pub elevation_deg: f32,
    pub angles_deg: Vec<f32>,
    pub columns: u32,
    pub framing: Framing,
    pub seed: u32,
}
impl Default for PropOptions {
    fn default() -> Self {
        Self {
            tile_width: 128,
            tile_height: 128,
            pixels_per_unit: 32.0,
            world_scale: 1.0,
            pivot: None,
            anchor: [0.5, 0.75],
            elevation_deg: 0.28_f32.to_degrees(),
            angles_deg: vec![0.0, 90.0, 180.0, 270.0],
            columns: 4,
            framing: Framing::Expand,
            seed: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropFrame {
    pub angle_deg: f32,
    pub atlas_rect: [u32; 4],
    pub pivot_pixels: [f32; 2],
    /// Nontransparent bounds relative to the frame: x, y, width, height.
    pub content_bounds: [u32; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropMetadata {
    pub format_version: u32,
    pub frame_width: u32,
    pub frame_height: u32,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub columns: u32,
    pub pivot_world: [f32; 3],
    pub pixels_per_unit: f32,
    pub world_scale: f32,
    pub elevation_deg: f32,
    pub frames: Vec<PropFrame>,
}
pub struct PropAtlas {
    pub sprites: RgbaImage,
    pub metadata: PropMetadata,
}

pub fn load_mesh(path: &Path) -> Result<PropMesh, PropError> {
    let mesh = match path.extension().and_then(|x| x.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("obj") => import::load_obj(path)?,
        Some("json") => {
            let mut bytes=Vec::new();
            std::fs::File::open(path)?.take(64*1024*1024+1).read_to_end(&mut bytes)?;
            if bytes.len()>64*1024*1024 { return Err(invalid("JSON mesh exceeds 64 MiB")); }
            serde_json::from_slice(&bytes)?
        },
        _ => return Err(PropError::Invalid("supported inputs are .obj (+ opaque Kd .mtl) or PropMesh .json; export Blender/glTF scenes to triangulated Y-up OBJ first".into())),
    };
    validate_mesh(&mesh)?;
    Ok(mesh)
}
pub fn save(atlas: &PropAtlas, directory: &Path) -> Result<(), PropError> {
    std::fs::create_dir_all(directory)?;
    atlas.sprites.save(directory.join("sprites.png"))?;
    std::fs::write(
        directory.join("prop.json"),
        serde_json::to_vec_pretty(&atlas.metadata)?,
    )?;
    Ok(())
}
fn invalid(message: &str) -> PropError {
    PropError::Invalid(message.into())
}
fn validate_mesh(mesh: &PropMesh) -> Result<(), PropError> {
    if mesh.vertices.is_empty() || mesh.faces.is_empty() || mesh.materials.is_empty() {
        return Err(invalid("mesh needs vertices, faces and materials"));
    }
    if mesh.vertices.len() > 1_000_000 || mesh.faces.len() > 1_000_000 || mesh.materials.len() > 256
    {
        return Err(invalid(
            "mesh exceeds limits (1M vertices/faces, 256 materials)",
        ));
    }
    if mesh
        .vertices
        .iter()
        .flatten()
        .any(|x| !x.is_finite() || x.abs() > 1e6)
    {
        return Err(invalid("positions must be finite and within +/-1,000,000"));
    }
    for m in &mesh.materials {
        if m.color[3] != 255 {
            return Err(invalid(
                "only opaque face materials are supported; output background is transparent",
            ));
        }
        if !["NONE", "STONE", "WOOD", "METAL", "BONE", "LEAF"].contains(&m.surface.as_str()) {
            return Err(invalid(
                "surface must be NONE, STONE, WOOD, METAL, BONE or LEAF",
            ));
        }
    }
    for f in &mesh.faces {
        if f.material >= mesh.materials.len() || f.indices.iter().any(|&i| i >= mesh.vertices.len())
        {
            return Err(invalid("face vertex/material index is out of range"));
        }
        let [a, b, c] = f.indices.map(|i| mesh.vertices[i]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        if cross.iter().all(|&x| x == 0.0) {
            return Err(invalid(
                "degenerate triangle; clean or triangulate the source mesh",
            ));
        }
    }
    Ok(())
}
fn camera(angle: f32, elevation: f32, scale: f32, origin: [f32; 2]) -> Camera {
    let (sine, cosine) = angle.to_radians().sin_cos();
    let (elevation_sine, elevation_cosine) = elevation.to_radians().sin_cos();
    Camera {
        sine,
        cosine,
        elevation_sine,
        elevation_cosine,
        scale,
        origin_x: origin[0],
        origin_y: origin[1],
    }
}
/// Bake all views with one shared scale/pivot. Expand preserves scale; Fit changes
/// it explicitly across every view; Fixed reports overflow instead of clipping.
pub fn bake(mesh: &PropMesh, options: &PropOptions) -> Result<PropAtlas, PropError> {
    validate_mesh(mesh)?;
    let o = options;
    if !(8..=2048).contains(&o.tile_width)
        || !(8..=2048).contains(&o.tile_height)
        || !o.pixels_per_unit.is_finite()
        || o.pixels_per_unit <= 0.0
        || o.pixels_per_unit > 4096.0
        || !o.world_scale.is_finite()
        || o.world_scale <= 0.0
        || o.world_scale > 1e6
        || !o.elevation_deg.is_finite()
        || !(-89.0..=89.0).contains(&o.elevation_deg)
        || o.anchor
            .iter()
            .any(|x| !x.is_finite() || !(0.01..=0.99).contains(x))
        || o.pivot
            .is_some_and(|p| p.iter().any(|v| !v.is_finite() || v.abs() > 1e6))
        || o.angles_deg.is_empty()
        || o.angles_deg.len() > 64
        || o.angles_deg
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 36000.0)
        || o.columns == 0
        || o.columns > 64
    {
        return Err(invalid(
            "invalid dimensions, finite scale, pivot, anchor, view angles or columns",
        ));
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for v in &mesh.vertices {
        for d in 0..3 {
            min[d] = min[d].min(v[d]);
            max[d] = max[d].max(v[d]);
        }
    }
    let pivot = o
        .pivot
        .unwrap_or([(min[0] + max[0]) * 0.5, min[1], (min[2] + max[2]) * 0.5]);
    let points: Vec<_> = mesh
        .vertices
        .iter()
        .map(|v| {
            V3::new(
                (v[0] - pivot[0]) * o.world_scale,
                (v[1] - pivot[1]) * o.world_scale,
                (v[2] - pivot[2]) * o.world_scale,
            )
        })
        .collect();
    let mut bounds = [0.0_f32; 4];
    for angle in &o.angles_deg {
        let cam = camera(*angle, o.elevation_deg, 1.0, [0.0, 0.0]);
        for p in &points {
            let q = render3d::project_with_camera(*p, cam);
            bounds[0] = bounds[0].min(q.x);
            bounds[1] = bounds[1].min(q.y);
            bounds[2] = bounds[2].max(q.x);
            bounds[3] = bounds[3].max(q.y);
        }
    }
    const MARGIN: f32 = 3.0;
    let mut width = o.tile_width;
    let mut height = o.tile_height;
    let mut scale = o.pixels_per_unit;
    let required = |scale: f32| {
        [
            ((-bounds[0] * scale + MARGIN) / o.anchor[0])
                .max((bounds[2] * scale + MARGIN) / (1.0 - o.anchor[0])),
            ((-bounds[1] * scale + MARGIN) / o.anchor[1])
                .max((bounds[3] * scale + MARGIN) / (1.0 - o.anchor[1])),
        ]
    };
    match o.framing {
        Framing::Expand => {
            let r = required(scale);
            if r.iter().any(|v| !v.is_finite() || *v > 2048.0) {
                return Err(invalid(
                    "expanded frame exceeds 2048px; reduce world_scale/pixels_per_unit or use fit",
                ));
            }
            width = width.max((r[0].ceil() as u32).div_ceil(8) * 8);
            height = height.max((r[1].ceil() as u32).div_ceil(8) * 8);
        }
        Framing::Fixed => {
            let r = required(scale);
            if r[0] > width as f32 || r[1] > height as f32 {
                return Err(invalid(
                    "mesh does not fit fixed tile; use expand/fit or lower scale",
                ));
            }
        }
        Framing::Fit => {
            for (extent, available) in [
                (-bounds[0], width as f32 * o.anchor[0] - MARGIN),
                (bounds[2], width as f32 * (1.0 - o.anchor[0]) - MARGIN),
                (-bounds[1], height as f32 * o.anchor[1] - MARGIN),
                (bounds[3], height as f32 * (1.0 - o.anchor[1]) - MARGIN),
            ] {
                if available <= 0.0 {
                    return Err(invalid("tile anchor leaves no space for margin"));
                }
                if extent > 0.0 {
                    scale = scale.min(available / extent);
                }
            }
        }
    }
    let columns = o.columns.min(o.angles_deg.len() as u32);
    let rows = (o.angles_deg.len() as u32).div_ceil(columns);
    let aw = width * columns;
    let ah = height * rows;
    if u64::from(aw) * u64::from(ah) > 16_777_216 {
        return Err(invalid(
            "atlas exceeds 16M pixels; reduce tile size or views",
        ));
    }
    let mut palette = Vec::new();
    for m in &mesh.materials {
        for factor in [0.49, 0.65, 0.82, 1.0, 1.15] {
            let mut c = m.color;
            for x in &mut c[..3] {
                *x = (*x as f32 * factor).min(255.0) as u8;
            }
            // Imported materials must not disappear behind the monster-only
            // 48-color cap. Up to 256 materials each contribute five shades.
            if !palette.contains(&c) {
                palette.push(c);
            }
        }
    }
    let triangles: Vec<_> = mesh
        .faces
        .iter()
        .map(|f| {
            let m = &mesh.materials[f.material];
            let [a, b, c] = f.indices.map(|i| points[i]);
            Triangle {
                a,
                b,
                c,
                color: m.color,
                local: [a, b, c],
                texture: render3d::texture_kind(&m.surface, "base"),
                seed: o.seed,
            }
        })
        .collect();
    let mut sprites = RgbaImage::new(aw, ah);
    let mut frames = Vec::new();
    let mut nearest = HashMap::new();
    for (index, angle) in o.angles_deg.iter().enumerate() {
        let mut high = RgbaImage::new(width * 2, height * 2);
        let mut depth = vec![f32::NEG_INFINITY; (width * height * 4) as usize];
        render3d::rasterize_mesh(
            &triangles,
            &mut high,
            &mut depth,
            camera(
                *angle,
                o.elevation_deg,
                scale * 2.0,
                [
                    width as f32 * o.anchor[0] * 2.0,
                    height as f32 * o.anchor[1] * 2.0,
                ],
            ),
        );
        let mut low = render3d::downsample(&high);
        render::quantize_cached(&mut low, &palette, &mut nearest);
        let mut b = [width, height, 0, 0];
        let mut visible = false;
        for (x, y, p) in low.enumerate_pixels() {
            if p[3] > 0 {
                visible = true;
                b[0] = b[0].min(x);
                b[1] = b[1].min(y);
                b[2] = b[2].max(x);
                b[3] = b[3].max(y);
            }
        }
        if !visible {
            return Err(invalid(
                "a view has no visible pixels; increase scale or use a volumetric mesh",
            ));
        }
        let ox = index as u32 % columns * width;
        let oy = index as u32 / columns * height;
        image::imageops::replace(&mut sprites, &low, i64::from(ox), i64::from(oy));
        frames.push(PropFrame {
            angle_deg: *angle,
            atlas_rect: [ox, oy, width, height],
            pivot_pixels: [width as f32 * o.anchor[0], height as f32 * o.anchor[1]],
            content_bounds: [b[0], b[1], b[2] - b[0] + 1, b[3] - b[1] + 1],
        });
    }
    Ok(PropAtlas {
        sprites,
        metadata: PropMetadata {
            format_version: 1,
            frame_width: width,
            frame_height: height,
            atlas_width: aw,
            atlas_height: ah,
            columns,
            pivot_world: pivot,
            pixels_per_unit: scale,
            world_scale: o.world_scale,
            elevation_deg: o.elevation_deg,
            frames,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cube() -> PropMesh {
        PropMesh {
            vertices: vec![
                [-1., 0., -1.],
                [1., 0., -1.],
                [1., 2., -1.],
                [-1., 2., -1.],
                [-1., 0., 1.],
                [1., 0., 1.],
                [1., 2., 1.],
                [-1., 2., 1.],
            ],
            faces: [
                [0, 1, 2],
                [0, 2, 3],
                [4, 6, 5],
                [4, 7, 6],
                [0, 4, 5],
                [0, 5, 1],
                [3, 2, 6],
                [3, 6, 7],
                [0, 3, 7],
                [0, 7, 4],
                [1, 5, 6],
                [1, 6, 2],
            ]
            .map(|indices| PropFace {
                indices,
                material: 0,
            })
            .to_vec(),
            materials: vec![PropMaterial {
                name: "stone".into(),
                color: [150, 110, 80, 255],
                surface: "STONE".into(),
            }],
        }
    }
    #[test]
    fn deterministic_transparent_rectangular_atlas() {
        let options = PropOptions {
            tile_width: 112,
            tile_height: 144,
            angles_deg: vec![0., 45., 90.],
            columns: 2,
            ..Default::default()
        };
        let a = bake(&cube(), &options).unwrap();
        let b = bake(&cube(), &options).unwrap();
        assert_eq!(a.sprites, b.sprites);
        assert_eq!(a.metadata.frames.len(), 3);
        assert_eq!(a.metadata.atlas_width, 224);
        assert_eq!(a.metadata.atlas_height, 288);
        assert_eq!(a.metadata.pivot_world, [0., 0., 0.]);
        assert_eq!(a.sprites.get_pixel(0, 0)[3], 0);
        assert!(a.sprites.pixels().any(|p| p[3] == 255));
        assert!(a.sprites.pixels().all(|p| p[3] == 0 || p[3] == 255));
        for f in a.metadata.frames {
            let [x, y, w, h] = f.content_bounds;
            assert!(x >= 2 && y >= 2 && x + w < 112 && y + h < 144);
        }
    }
    #[test]
    fn shared_scale_and_expansion_are_preserved() {
        let options = PropOptions {
            tile_width: 32,
            tile_height: 32,
            ..Default::default()
        };
        let a = bake(&cube(), &options).unwrap();
        assert!(a.metadata.frame_width > 32);
        assert_eq!(a.metadata.pixels_per_unit, 32.);
        let fixed = PropOptions {
            framing: Framing::Fixed,
            ..options.clone()
        };
        assert!(bake(&cube(), &fixed)
            .unwrap_err_text()
            .contains("does not fit"));
        let fit = PropOptions {
            framing: Framing::Fit,
            ..options
        };
        let b = bake(&cube(), &fit).unwrap();
        assert_eq!(b.metadata.frame_width, 32);
        assert!(b.metadata.pixels_per_unit < 32.);
    }
    #[test]
    fn source_translation_and_explicit_ground_pivot() {
        let a = bake(&cube(), &PropOptions::default()).unwrap();
        let mut mesh = cube();
        for p in &mut mesh.vertices {
            p[0] += 10.;
            p[1] += 5.;
            p[2] -= 7.;
        }
        let b = bake(&mesh, &PropOptions::default()).unwrap();
        assert_eq!(a.sprites, b.sprites);
        assert_eq!(b.metadata.pivot_world, [10., 5., -7.]);
        let c = bake(
            &mesh,
            &PropOptions {
                pivot: Some([10., 5., -7.]),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(b.sprites, c.sprites);
    }
    #[test]
    fn world_scale_changes_size_not_pivot() {
        let a = bake(&cube(), &PropOptions::default()).unwrap();
        let b = bake(
            &cube(),
            &PropOptions {
                world_scale: 0.5,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            a.metadata.frames[0].pivot_pixels,
            b.metadata.frames[0].pivot_pixels
        );
        assert!(b.metadata.frames[0].content_bounds[2] < a.metadata.frames[0].content_bounds[2]);
    }
    #[test]
    fn invalid_data_and_resource_limits_fail_before_rendering() {
        let mut mesh = cube();
        mesh.faces[0].indices[0] = 100;
        assert!(bake(&mesh, &PropOptions::default()).is_err());
        let mut mesh = cube();
        mesh.vertices[0][0] = f32::NAN;
        assert!(bake(&mesh, &PropOptions::default()).is_err());
        let mut mesh = cube();
        mesh.materials[0].color[3] = 100;
        assert!(bake(&mesh, &PropOptions::default()).is_err());
        for options in [
            PropOptions {
                pixels_per_unit: f32::NAN,
                ..Default::default()
            },
            PropOptions {
                columns: 0,
                ..Default::default()
            },
            PropOptions {
                anchor: [0., 1.],
                ..Default::default()
            },
            PropOptions {
                world_scale: 1e6,
                ..Default::default()
            },
            PropOptions {
                tile_width: 2048,
                tile_height: 2048,
                angles_deg: vec![0.; 64],
                ..Default::default()
            },
        ] {
            assert!(bake(&cube(), &options).is_err());
        }
        assert!(serde_json::from_str::<PropOptions>("{\"typo\":1}").is_err());
    }
    #[test]
    fn coplanar_triangles_have_no_depth_or_coverage_seams() {
        let mut mesh = cube();
        mesh.faces = vec![
            PropFace {
                indices: [0, 1, 2],
                material: 0,
            },
            PropFace {
                indices: [0, 2, 3],
                material: 0,
            },
        ];
        mesh.materials[0].surface = "NONE".into();
        let options = PropOptions {
            tile_width: 96,
            tile_height: 96,
            angles_deg: vec![0.0],
            elevation_deg: 0.0,
            pivot: Some([0.0, 1.0, -1.0]),
            anchor: [0.5, 0.5],
            ..Default::default()
        };
        let atlas = bake(&mesh, &options).unwrap();
        let color = *atlas.sprites.get_pixel(48, 48);
        assert_eq!(color[3], 255);
        for y in 18..78 {
            for x in 18..78 {
                assert_eq!(*atlas.sprites.get_pixel(x, y), color);
            }
        }
    }
    #[test]
    fn fitted_lighting_is_scale_safe() {
        let mut ordinary = cube();
        ordinary.materials[0].surface = "NONE".into();
        let mut huge = ordinary.clone();
        for p in &mut huge.vertices {
            for value in p {
                *value *= 5e5;
            }
        }
        let options = PropOptions {
            tile_width: 64,
            tile_height: 64,
            angles_deg: vec![0.0],
            elevation_deg: 0.0,
            framing: Framing::Fit,
            ..Default::default()
        };
        let a = bake(&ordinary, &options).unwrap();
        let b = bake(
            &huge,
            &PropOptions {
                world_scale: 1e6,
                ..options
            },
        )
        .unwrap();
        assert_eq!(*a.sprites.get_pixel(32, 32), *b.sprites.get_pixel(32, 32));
        assert!(a.sprites.get_pixel(32, 32)[0] > 0);
    }
    #[test]
    fn late_imported_materials_keep_their_colors() {
        let mut mesh = cube();
        mesh.materials = (0..10)
            .map(|i| PropMaterial {
                name: format!("warm{i}"),
                color: [150 + i * 10, 30 + i, 0, 255],
                surface: "NONE".into(),
            })
            .collect();
        mesh.materials.push(PropMaterial {
            name: "blue".into(),
            color: [0, 0, 255, 255],
            surface: "NONE".into(),
        });
        for face in &mut mesh.faces {
            face.material = 10;
        }
        let atlas = bake(&mesh, &PropOptions::default()).unwrap();
        assert!(atlas
            .sprites
            .pixels()
            .filter(|p| p[3] > 0)
            .all(|p| p[0] == 0 && p[2] > 100));
    }
    trait ErrorText {
        fn unwrap_err_text(self) -> String;
    }
    impl ErrorText for Result<PropAtlas, PropError> {
        fn unwrap_err_text(self) -> String {
            match self {
                Err(e) => e.to_string(),
                Ok(_) => panic!("expected error"),
            }
        }
    }
}
