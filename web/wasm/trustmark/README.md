# Local TrustMark detection

Both the WASM demo and native localhost viewer have a **Detect watermark** panel.
It accepts PNG spritesheets up to 16 MiB, 8192 pixels per dimension, and 16
megapixels. PNG dimensions are checked before image decoding. Files are never
uploaded: the selected pixels are processed locally in the browser. Detection
is explicitly user-triggered and independent of generation and gameplay.

The first check downloads Adobe's Q decoder (47,401,222 bytes, about 45.2 MiB),
a small resize graph, and Microsoft ONNX Runtime Web 1.20.1 (additional JS/WASM).
Subsequent checks reuse the sessions. An Internet connection and CDN/model CORS
availability are required on the first check. Network/inference errors are shown
as errors, never as a negative watermark result. The decoder prefers WebGPU
(HTTPS or localhost) and attempts a WASM fallback.
The float16 decoder may not be supported by every WASM backend; unsupported
browsers report a check error. Resize always uses single-threaded WASM.

Protocol: TrustMark **Q**, **BCH_5**, 61 data bits containing the seven-bit ASCII
characters `INFRNL1`, most significant bit first, followed by 12 zero bits.
Require successful ECC decoding AND an exact schema/payload match. Other valid
TrustMark payloads are not classified as Infernal Diffusion. This marker is public
and forgeable: it is not a signature, identity verification, or C2PA provenance.
Absence is inconclusive because normal generation is unmarked and transformations
can destroy an existing watermark.

Raw 8-bit RGB/RGBA/indexed non-interlaced PNG samples are decoded with bounded streaming
DEFLATE, CRC checks, and PNG row reconstruction. Indexed PNG palettes (PLTE) and transparency (tRNS) expand losslessly to RGBA.
Grayscale, interlaced, and animated PNGs are explicitly unsupported. RGB hidden by alpha is retained:
it is the main watermark carrier in sparse atlases. Alpha is ignored for decoding,
matching native Rust. Transparency processing, conversion, optimization, or sprite
repacking commonly removes this fragile marker. Canvas is deliberately not used.

Q center-crops only images beyond a 2:1 aspect ratio, then resizes to 256×256.
After the upstream resizer's [0,1] clipping, input is converted to [-1,1], matching
native Rust `ModelImage`. Browser cubic versus Rust bilinear resizing can introduce
differences. Native-to-browser round-trip validation passed for the generated V2 spider fixture described below.

## Third-party source

`vendor/bch-ecc.mjs` and `vendor/data-layer.mjs` come from Adobe TrustMark
commit `f0df43ed760bf3305f98f495f990b8c2315e7983`:
https://github.com/adobe/trustmark/tree/f0df43ed760bf3305f98f495f990b8c2315e7983/js
Only ES module imports/exports were added to those files. `vendor/resize.mjs`
extracts upstream Q preprocessing, adds configuration/exports, propagates errors,
and disposes temporary tensors. The accompanying Adobe MIT license is in
`vendor/LICENSE`. No model binaries or runtime binaries are vendored.

Model source: https://cai-watermark.adobe.net/watermarking/trustmark-models/
Runtime source: https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.1/dist/

Run protocol/validation tests: `node --test web/wasm/tests/trustmark.node.mjs`.

## Real-model integration test

Install the pinned official `onnxruntime-web@1.20.1` package into a temporary
location, download `decoder_Q.onnx` and `resizer.onnx` into a model directory,
then run:

```sh
ORT_WEB_MODULE=/tmp/deps/node_modules/onnxruntime-web/dist/ort.webgpu.min.mjs \
TRUSTMARK_MODEL_DIR=/tmp/models \
node web/wasm/tests/trustmark.actual-wasm.mjs image.png match
```

Additional image/expected-result pairs can follow. This invokes the actual browser
runtime bundle's WASM backend in Node with the same PNG decoder, preprocessing,
ECC, and classification functions as the page. It does not use native ONNX Runtime.
Real-model checks passed for Adobe `ghost_Q.png` and `ripley_Q.png` (`other`),
Adobe's unmarked `ghost.png` (`absent`), and the unmarked repository showcase
`motion_orc/sprites.png` (`absent`). The native-generated V2 spider fixture also
passed: marked PNG returned `match`, original PNG returned `absent`. Alpha was
byte-identical after watermarking. These tests do not replace browser DOM/CDN/
WebGPU QA, which remains untested.

Ordinary native exports use lossless indexed PNG when their palette fits 256 colors;
watermarked exports retain full RGB/RGBA samples. Both formats are accepted by this
detector. An independently encoded 50-color indexed version of the showcase orc
round-tripped byte-for-byte to its original RGBA and returned `absent` through the
actual browser runtime WASM backend. This is a basic public marker check, not a robust provenance system.

## Verified native interoperability

The Rust integration generated and marked a V2 giant spider (seed 42, 3328×3200),
then wrote a PNG. This exact browser runtime bundle's WASM provider, running in
Node with the same PNG reader/preprocessing/detector code, returned `match` for
the marked export and `absent` for its original. Alpha bytes were unchanged.
The image was 7.9% visible; RGB under alpha carries much of this fragile marker.
Visible RGB changed by a mean absolute 1.75/255 (maximum 16), and the PNG grew
from 2,615,849 to 4,840,815 bytes. These are measurements for this fixture, not a
quality or detection guarantee for arbitrary atlases.

Browser DOM interaction, live CDN loading, and WebGPU execution were not tested
in that environment because Chromium could not create sockets. Node tests cover
reset/replacement/repeated-click behavior, but do not replace full browser QA.
