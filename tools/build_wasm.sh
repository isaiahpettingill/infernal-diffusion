#!/usr/bin/env bash
# Full pipeline, deterministic vocabulary parser, shared-memory Rayon workers.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN="${WASM_TOOLCHAIN:-nightly-2026-10-12}"
FEATURES=atomics,bulk-memory,simd128,relaxed-simd,tail-call,multivalue,mutable-globals,nontrapping-fptoint,sign-ext,extended-const,reference-types
export RUSTFLAGS="-C target-feature=+${FEATURES//,/,+} -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base -C link-arg=--import-memory -C link-arg=--shared-memory -C link-arg=--max-memory=2147483648"
# Rebuild std with atomics; using the precompiled std silently loses thread safety.
cargo build --locked --lib --target wasm32-unknown-unknown --no-default-features --features wasm --profile wasm-release -Z build-std=std,panic_abort
wasm-bindgen --version
wasm-bindgen target/wasm32-unknown-unknown/wasm-release/infernal_diffusion.wasm --target web --out-dir web/wasm/pkg
# Accept every supported WASM proposal during optimization, with the requested O3.
# No fast-math: it would weaken deterministic rendering.
wasm-opt --all-features -O3 web/wasm/pkg/infernal_diffusion_bg.wasm -o web/wasm/pkg/infernal_diffusion_bg.opt.wasm
mv web/wasm/pkg/infernal_diffusion_bg.opt.wasm web/wasm/pkg/infernal_diffusion_bg.wasm
printf '{"target":"wasm32-unknown-unknown","rustFeatures":"%s","optimization":"-O3 --all-features","toolchain":"%s"}\n' "$FEATURES" "$RUSTUP_TOOLCHAIN" > web/wasm/pkg/build-info.json
