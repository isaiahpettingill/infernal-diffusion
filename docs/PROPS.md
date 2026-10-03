# Imported meshes → decorative tile sprites

Props use the same deterministic triangle rasterizer, two-times coverage
sampling, directional light and palette quantization as the procedural monsters.
They bypass monster anatomy, prompts and animation. Existing monster metadata,
locomotion and behavior contracts are unchanged.

## Quick start

```sh
cargo run --release --no-default-features -- bake-prop \
  examples/props/assets/nature-kit/rock_largeC.obj output/rock \
  examples/props/options.json
uv run tools/prop_evidence.py --output output/props
```

The first command writes transparent `sprites.png` and `prop.json`. The second
re-bakes five actual Kenney CC0 models, verifies all directional alpha bounds,
and makes a labeled checkerboard preview. It uses one shared 96 pixels/world
unit scale, not independent thumbnail fitting. The supplied OBJ/MTL files and
pack licenses live in `examples/props/assets`; `examples/props/PROVENANCE.json`
records official sources, download hashes and license details. There is no
network fetch needed to reproduce the bakes after checkout.

The CLI accepts OBJ plus adjacent MTL, or the public `PropMesh` JSON schema.
The optional fourth argument is a `PropOptions` JSON file; omitted fields retain
defaults. Unknown option/mesh fields are errors, so misspelled controls do not
silently disappear.

## Coordinates, tile placement and scale

Meshes are right-handed Y-up. Input positions are source units, with X horizontal
and Z depth. The default pivot is the source bounds' bottom-center; this makes
models exported off-center usable without editing. `pivot: [x,y,z]` overrides it
in **source space**, useful for doors, asymmetric scenery or a specific contact
point. The baker subtracts this pivot, then multiplies by `world_scale`.

`pixels_per_unit` converts the resulting world units to pixels. Keep it equal
across assets to preserve relative size. Different asset packs do not necessarily
agree on the meaning of one unit: calibrate `world_scale` deliberately rather
than fitting each asset independently. Monster coordinates use
`render3d::PIXELS_PER_UNIT` (96/62 pixels per anatomy unit); use that exact value
when props are authored in the same units. The prop-friendly default is 32.

`anchor` is the pivot position as a fraction of the output frame, measured from
its top-left (default `[0.5,0.75]`). `tile_width` and `tile_height` default to 128.
The metadata exports the actual `pivot_pixels` for every frame. Position sprites
using this pivot; do not assume the center or bottom-most visible pixel. Tall
scenery may occupy more than a single logical map tile.

Three explicit framing modes prevent accidental cropping:

- `expand` (default): grow frame dimensions in 8-pixel increments as needed for
  every requested view, preserving world scale
- `fixed`: require geometry and a 3-pixel safety margin to fit the given tile;
  return an error otherwise
- `fit`: reduce one shared pixel scale across all views to fit; never enlarge
  smaller meshes. The effective scale is returned as `pixels_per_unit`

`angles_deg` is an ordered list of yaw angles (default 0,90,180,270).
`elevation_deg` controls the camera elevation (default 16.04°, matching monsters).
The zero-degree camera looks along the Z axis; larger depth is nearer. At 90°,
X becomes depth. `columns` lays these views into an atlas. There is no animation
stride: frame order is the requested angle order. Empty final-row cells remain
transparent.

## Rust API and JSON meshes

```rust
use infernal_diffusion::props::{self, PropOptions};
let mesh = props::load_mesh(std::path::Path::new("scenery.obj"))?;
let atlas = props::bake(&mesh, &PropOptions::default())?;
props::save(&atlas, std::path::Path::new("output/scenery"))?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

For procedural geometry or an engine-owned converter, construct `PropMesh`
directly. `vertices` contains `[x,y,z]`, `faces` contains zero-based
`{"indices":[0,1,2],"material":0}`, and `materials` contains
`{"name":"stone","color":[140,130,110,255],"surface":"STONE"}`.
A material's base color is multiplied by the same face lighting as monsters,
then reduced to five shade bands per base color. Supported procedural surface
styles are `NONE`, `STONE`, `WOOD`, `METAL`, `BONE`, `LEAF`; imported OBJ colors
use `NONE` to preserve the authored surfaces. Set `seed` for deterministic
procedural texture variation. Input material alpha must be 255; untouched output
pixels have zero alpha. This is opaque geometry over a transparent background,
not alpha-blended glass.

`PropMetadata` version 1 records atlas/frame dimensions, the source pivot,
world scale, effective pixel scale, elevation, and each frame's angle, atlas
rectangle, pivot and nontransparent bounds. No inferred hitboxes, navigation or
terrain connectivity are included.

## Import scope and Blender/glTF conversion

The native importer supports OBJ position vertices, faces with positive or
negative indices (including `v/vt/vn`), named opaque `Kd` MTL materials, and simple
planar polygons triangulated by ear clipping. Zero-area OBJ triangles are
discarded as exporter debris; an entirely degenerate mesh is rejected. OBJ without MTL uses a neutral
opaque default. Material files must remain inside the OBJ's directory; absolute
paths, parent traversal and symlink escapes are rejected.

For a Blender scene or glTF model:

1. Import it into Blender, select the desired static objects, apply transforms,
   and apply geometry modifiers or choose evaluated-mesh export
2. Ensure materials are opaque flat diffuse colors. If visual identity comes
   from UV textures/PBR maps, manually convert/bake those to flat material regions
   or choose a low-poly flat-colored source instead
3. Export Wavefront OBJ with materials, triangulation, Y-up, and an intentional
   forward axis. Keep OBJ and MTL together
4. Bake with a known `world_scale`, inspect the directional preview, and choose
   the ground pivot/elevation your game uses

There is no direct `.blend`, glTF/GLB, UV texture sampling, PBR, skeletal animation,
normal-map, metallic/roughness-map or alpha-transparency importer. The importer
rejects texture maps, transparent materials, vertex colors and unsupported
geometry instead of silently losing their appearance. OBJ normals/UV indices
are validated but face lighting comes from geometric normals; UVs are not used.
Seamless terrain and tile edge stitching are not provided. These sprites are
appropriate for decorative rocks, furniture, pickups and standalone scenery.

Limits are explicit: at most 1M vertices/faces, 256 materials, 64 views, 2048px
per frame axis, and 16M atlas pixels. Nonfinite positions/options, bad indices,
degenerate JSON triangles, unrenderably thin views and unsupported formats produce
errors. For huge assets, decimate and simplify before importing.

## Godot and C

See [Godot prop integration](../godot/README.md#imported-static-props) for the async
`bake_prop_async` path and Sprite2D pivot setup. The additive C ABI is declared
in `include/infernal_diffusion.h`: load/bake a prop, copy metadata and RGBA bytes,
then free its independent handle. Prop results never masquerade as monsters.

## Verification

```sh
cargo test --release --no-default-features
cargo test --release
cargo clippy --release --no-default-features --all-targets -- -D warnings
uv run tools/prop_evidence.py
# After building/copying matching native libraries:
INFERNAL_PROP_SMOKE_ASSET="$PWD/examples/props/assets/nature-kit/rock_largeC.obj" \
  uv run tools/smoke_gdextension.py --godot /path/to/godot
```

Core tests cover malformed imports, concave polygons, material and path errors,
rectangular bounds, transparent pixels, world scale, automatic and explicit
pivots, deterministic output, all framing modes and allocation limits. Godot
smoke tests cover repeated prop requests, errors/recovery and preservation of a
retained monster while prop requests run.

Input allocation limits: OBJ/JSON files are capped at 64 MiB, all referenced
MTL text combined at 4 MiB, and individual logical OBJ/MTL lines at 1 MiB.
At most 256 MTL files are read. Count limits are checked during OBJ parsing.
The prop palette preserves all imported material colors (up to five shades per
material), rather than silently dropping later materials at the monster-only
48-color limit.

The shared rasterizer keeps the historical monster sampling unchanged. The prop
path uses consistent pixel-center barycentric interpolation and scale-safe
normal calculations; these avoid seams on coplanar imported furniture and retain
lighting when fitted assets use very large source units.
