Hades bought all the GPUs in the world and built a monster generator. Enter Infernal Diffusion

Infernal Diffusion generates deterministic, animated directional monster sprites,
metadata and collision information. An optional embedded BERT parser interprets
prompts; anatomy, intermediate 3D meshes, animation and sprite baking are procedural
and run on the CPU. No external image-generation service or model training is needed.

## Opt-in V2

V1 remains the default. Select the refined procedural profile explicitly:

```sh
cargo run --release --no-default-features -- "friendly frog that sprays acid" 42 output/frog --profile v2
```

[V2 recipes and capabilities](docs/V2.md), [CLI/Rust/C/Godot/WASM profile APIs](docs/generation-profiles.md),
[explicit hybrid recipes](docs/hybrid-recipes.md), [spectral damage/phasing rules](docs/spectral-contract.md),
[posed attack-origin tracks](docs/attack-origin-tracks.md),
[typed encounter contracts](docs/encounter-contract.md),
[player-relative V1/V2 world sizes](docs/world-scaling.md), and the
[shared V1/V2 water navigation correction](docs/aquatic-animation.md).

V2 includes refined humanoid faces and merfolk hair, seeded extra limbs/tails,
full-body mermaid attacks, growing-arm tree smashes, recognizable jumbled skeletons,
and inky shades that attack through coherent deforming tendrils. See the
[actual rendered review](docs/evidence/README.md).

## Local playground

From the repository root, with Rust stable and your platform's native build tools installed:

```sh
cargo run --locked --release -- serve
```

Open <http://localhost:8799/>. Enter a prompt and seed, select **V1** (the default)
or **V2**, then generate. The Arena includes example prompts, a directional animation
viewer, a fight sandbox, and protobuf/CBOR packages saved in `web/generated/`.
The default build uses the included **embedded BERT model** on your CPU, alongside
explicit vocabulary rules; model errors are reported instead of silently falling back.
The UI shows the server parser and each generated package's actual parser provenance.
No external model download, Node, Python, or generation service is required.
For an explicitly vocabulary-only build, add `--no-default-features` before `-- serve`.
The two parser builds use separate output caches.

Choose an animation and direction to play it immediately. Pause/resume, restart, and
0.25×–2× speed controls are available. Loops repeat; one-shot attacks (including shade
recovery) and death clips play their baked frames once and hold the last frame.
The development server listens on all interfaces; use it only on a trusted network.
There is no hosted playground URL configured in this repository.
After building, `python3 tools/smoke_playground.py --expect-parser embedded-bert` checks both profiles over HTTP
using an isolated temporary output directory.

## Browser / Web Worker build

The full procedural pipeline can run in a modern browser with a shared-memory
WASM thread pool and deterministic vocabulary parsing instead of BERT. See
[the runnable browser demo and build instructions](web/wasm/README.md).
The WASM CI builds an optimized wasm32 artifact and tests actual worker generation.

## Random monster prompts

The playgrounds offer a separate summoner-profile selector and seeded **Random prompt**
control: **Default**, **Necromancer**, **Shaman**, **Lunatic**, **Boss** and **Ocean**.
Default now covers the broad bestiary at every difficulty. Suggestions remain editable,
and V1/V2 is still an independent generation choice. See [profile behavior and APIs](docs/random-prompts.md).

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
