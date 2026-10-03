Hades bought all the GPUs in the world and built a monster generator. Enter Infernal Diffusion

Infernal Diffusion generates deterministic, animated directional monster sprites,
metadata and collision information. An optional embedded BERT parser interprets
prompts; anatomy, intermediate 3D meshes, animation and sprite baking are procedural
and run on the CPU. No external image-generation service or model training is needed.

## Generate a monster

```sh
cargo run --release -- "giant spider that shoots fire" 42 output/spider
cargo run --release -- inspect output/spider
cargo run --release -- validate output/spider
```

Add `--no-default-features` to the Cargo command to use the vocabulary-only parser.
Use the `bake3d-cbor` subcommand for CBOR metadata instead of protobuf. Each package
contains its sprite/emission atlases, animation timing, anatomy-linked attacks,
and locomotion categories/modifiers. Use returned atlas dimensions and column
counts; animated framing is fitted dynamically at a shared pixel scale.

## Bake imported props

Use the same CPU sprite style for static OBJ/MTL or JSON meshes:

```sh
cargo run --release --no-default-features -- bake-prop \
  examples/props/assets/nature-kit/rock_largeC.obj output/rock \
  examples/props/options.json
uv run tools/prop_evidence.py
```

See [prop controls, import limits and CC0 examples](docs/PROPS.md) for world
scale, tile dimensions, ground pivot, camera angles and transparent atlases.

## Integrate with Godot

Start with the [GDExtension setup](godot/README.md) and
[engine-owned locomotion contract](godot/LOCOMOTION.md). The asynchronous wrapper
returns in-memory RGBA atlases and a versioned `package.locomotion` profile by
default. Your engine decides navigation, targets and movement patterns.

Build on the matching host operating system. For Windows x64 with the Rust MSVC
toolchain and Windows build tools installed:

```sh
uv run tools/build_gdextension.py --target x86_64-pc-windows-msvc
uv run tools/smoke_gdextension.py --godot C:/path/to/Godot.exe
```

Linux x64 uses `x86_64-unknown-linux-gnu`; macOS targets are listed in the build
script. A Linux `.so` cannot be used in a Windows project. The desktop-binaries
GitHub Actions workflow can build platform-specific addons when run in your repo.

## Inspect and test changes

- [Anatomical constraints, diversity and exact Blender inspection](docs/anatomy-and-inspection.md)
- [Animation solver and same-seed visual evidence](docs/animation-motion.md)
- [CBOR bindings and compatibility](bindings/cbor/README.md)

```sh
cargo test --release
cargo test --release --no-default-features
cargo clippy --release --no-default-features --all-targets -- -D warnings
```

The `export_intermediate` example exports the exact triangles consumed by the
sprite baker for inspection in Blender. The evidence tools compare identical
prompts/seeds and check every directional atlas frame for clipping.
