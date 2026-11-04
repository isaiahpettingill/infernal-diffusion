# Experimental TrustMark atlas label

This is an opt-in toy provenance label, **not a signature or cryptographic proof**.
Anyone can embed the public identifier. A positive means the fixed Infernal Diffusion
marker decoded; a negative does not establish that an image came from elsewhere.
False positives and false negatives are possible. Resizing, cropping, recompression,
palette reduction, transparency, and sparse pixel art can destroy the marker.

## Native export

Default Rust, C, Godot, WASM, and in-game generation do **not** run watermark inference,
load models, or link TrustMark/ONNX. The native-only `trustmark` Cargo feature adds
[Adobe's Rust crate 0.4.0](https://github.com/adobe/trustmark/tree/main/rust), which needs
Rust 1.88+, its supported native ONNX runtime platform, and separately downloaded
`encoder_Q.onnx` and `decoder_Q.onnx`. Adobe distributes the code and model weights
under MIT; retain their license when redistributing weights. No weights are bundled
or downloaded by Infernal Diffusion at build or runtime. Building the optional crate
can download its native ONNX runtime dependency.

Fetch just Q's models with the explicit checksum-verified helper (Python 3.11+):

```sh
python3 tools/fetch_trustmark_models.py /tmp/trustmark-models
```

Alternatively use Adobe's documented `cargo xtask fetch-models` in the upstream
repository's `rust/` directory (it downloads every variant). Then point to that folder:

```sh
cargo run --release --no-default-features --features trustmark -- \
  generate-marked "giant spider" 42 output/marked path/to/trustmark/rust/models --profile v2
```

Use `watermark::AtlasWatermarker::new(model_dir)` and `mark_generated(&mut generated)`
for Rust in-memory export. Save with `save_generated_with_format_and_encoding` and
`atlas_png::Encoding::Rgba`, as `generate-marked` does, to bypass ordinary palette PNG storage. Load the model once and reuse it. This explicit path marks
root and companion `sprites.png` color atlases; emission/projectile textures and
metadata are untouched. Existing commands keep producing unmarked images.

Watermarking necessarily changes RGB, including the pixel-art palette. Keep unmarked
originals for pixel-perfect games. Dimensions and every alpha byte are preserved.
Raw RGB under transparent pixels carries part of the label. The web detector reads
PNG samples directly instead of using Canvas, which would erase those hidden bytes.
After restoring alpha and quantizing to PNG bytes, each candidate is decoded again;
if it fails, the entire in-memory operation fails without changing any package.
Images smaller than 256 pixels on either side are rejected rather than enlarged.
No claim is made that all procedural atlases work. Optimizers, palette conversion,
canvas roundtrips, alpha premultiplication, sprite repacking, and image editors may
erase hidden RGB and break detection even if the visible sprite looks identical.
Keep the original exported PNG when trying this experiment.

The on-image format is TrustMark variant Q, BCH_5 (schema 1), the ASCII7 string
`INFRNL1` followed by twelve zero bits: 61 data bits total. This short fixed namespace
fits the error-corrected payload; the full product name does not fit as text.

## Verification

```sh
cargo test --no-default-features --features trustmark watermark::tests
TRUSTMARK_MODELS=path/to/models cargo test --no-default-features --features trustmark \
  watermark::tests::generated_atlas_roundtrip -- --ignored
```

The model-backed test generates a real V2 spider atlas, verifies detection after a PNG
roundtrip, and checks exact size and alpha preservation. It is ignored by ordinary `cargo test` runs. The dedicated optional-feature CI job
explicitly fetches checksum-pinned weights, runs the model test, and checks the
export through the real browser WASM decoder. Weights are never repository assets.
A skipped local test is not evidence of successful real-model encoding.

## Dependency size

At implementation time Adobe's model server reports 17,312,208 bytes for the native
Q encoder, 47,401,222 bytes for the Q decoder, and 454 bytes for the browser resize
model. These are separate from the ONNX runtime. Model files stay external to normal
builds, and browser inference is loaded only on an explicit detection request.

## Native runtime fallback

If ORT's dependency CDN is unavailable, use Microsoft's official matching
[ONNX Runtime 1.22.0 release](https://github.com/microsoft/onnxruntime/releases/tag/v1.22.0)
for your platform. For the Linux x64 archive, set `ORT_LIB_LOCATION` to the extracted
folder, `ORT_PREFER_DYNAMIC_LINK=1`, and add its `lib` directory to `LD_LIBRARY_PATH`
when building/running. The optional CI job demonstrates this with a verified archive.
Do not change ONNX versions independently of TrustMark's pinned `ort` dependency.
