#!/usr/bin/env bash
# Full seeded V1/V2 pipeline with the bundled open-source Candle BERT encoder.
set -euo pipefail
cd "$(dirname "$0")/.."
THREADED="${WASM_THREADED:-0}"
FEATURES=simd128,bulk-memory
export RUSTUP_TOOLCHAIN="${WASM_TOOLCHAIN:-stable}"
CARGO_ARGS=()
CRATE_FEATURES=wasm,bert
if [[ "$THREADED" == 1 ]]; then
  export RUSTUP_TOOLCHAIN="${WASM_TOOLCHAIN:-nightly-2026-10-12}"
  FEATURES=atomics,bulk-memory,simd128,relaxed-simd,tail-call,multivalue,mutable-globals,nontrapping-fptoint,sign-ext,extended-const,reference-types
  CRATE_FEATURES=wasm-threads,bert
  CARGO_ARGS=(-Z build-std=std,panic_abort)
  export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\" -C target-feature=+${FEATURES//,/,+} -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base -C link-arg=--import-memory -C link-arg=--shared-memory -C link-arg=--max-memory=2147483648"
else
  export RUSTFLAGS="--cfg getrandom_backend=\"wasm_js\" -C target-feature=+${FEATURES//,/,+}"
fi
cargo build --locked --lib --target wasm32-unknown-unknown --no-default-features --features "$CRATE_FEATURES" --profile wasm-release "${CARGO_ARGS[@]}"
wasm-bindgen --version
mkdir -p web/wasm/pkg
wasm-bindgen "${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/wasm-release/infernal_diffusion.wasm" --target web --out-dir web/wasm/pkg
wasm-opt --all-features -O3 web/wasm/pkg/infernal_diffusion_bg.wasm -o web/wasm/pkg/infernal_diffusion_bg.opt.wasm
mv web/wasm/pkg/infernal_diffusion_bg.opt.wasm web/wasm/pkg/infernal_diffusion_bg.wasm
cp LICENSE web/wasm/pkg/infernal-LICENSE
cp models/bert-mini/LICENSE-APACHE-2.0.txt web/wasm/pkg/
cp models/bert-mini/NOTICE.md web/wasm/pkg/model-NOTICE.md
cp models/bert-mini/config.json web/wasm/pkg/model-config.json
python3 - "$THREADED" "$FEATURES" "$RUSTUP_TOOLCHAIN" <<'PY'
import hashlib, json, pathlib, subprocess, sys
root = pathlib.Path('web/wasm')
model = pathlib.Path('models/bert-mini/model.safetensors')
info = {'target':'wasm32-unknown-unknown', 'rustFeatures':sys.argv[2], 'optimization':'-O3 --all-features', 'toolchain':sys.argv[3], 'liveGenerationBudgetVersion':1, 'portable':sys.argv[1]!='1', 'threads':sys.argv[1]=='1', 'bert':True, 'backend':'candle', 'model':'google/bert_uncased_L-2_H-128_A-2', 'modelSha256':hashlib.sha256(model.read_bytes()).hexdigest()}
(root/'pkg/build-info.json').write_text(json.dumps(info, indent=2)+'\n')
files = [root/'client.js', root/'worker.js', *sorted((root/'pkg').rglob('*'))]
manifest = {'schema_version':1, 'revision':subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(), 'source_dirty':bool(subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip()), 'model':{'id':info['model'], 'embedded':True, 'sha256':info['modelSha256'], 'license':'Apache-2.0'}, 'artifacts':[{'path':str(p.relative_to(root)), 'sha256':hashlib.sha256(p.read_bytes()).hexdigest(), 'size_bytes':p.stat().st_size} for p in files if p.is_file()]}
(root/'bundle-manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')
PY
