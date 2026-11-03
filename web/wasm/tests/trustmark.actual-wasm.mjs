// Optional real-model integration check; requires onnxruntime-web 1.20.1 and model files.
// ORT_WEB_MODULE=/absolute/node_modules/onnxruntime-web/dist/ort.webgpu.min.mjs \
// TRUSTMARK_MODEL_DIR=/models node web/wasm/tests/trustmark.actual-wasm.mjs image.png expected-class
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { resolve, join } from 'node:path';
import { decodePNG } from '../trustmark/png.mjs';
import { detectWithSessions } from '../trustmark/detector.mjs';
const { ORT_WEB_MODULE, TRUSTMARK_MODEL_DIR } = process.env;
if (!ORT_WEB_MODULE || !TRUSTMARK_MODEL_DIR || process.argv.length < 4 || process.argv.length % 2 !== 0)
  throw new Error('Set ORT_WEB_MODULE and TRUSTMARK_MODEL_DIR; pass PNG path and expected class (match/other/absent).');
for (let i = 3; i < process.argv.length; i += 2)
  assert.ok(['match', 'other', 'absent'].includes(process.argv[i]), `Invalid expected class: ${process.argv[i]}`);
const ort = await import(pathToFileURL(resolve(ORT_WEB_MODULE)).href);
ort.env.wasm.numThreads = 1;
const options = { executionProviders: ['wasm'] };
const resize = await ort.InferenceSession.create(await readFile(join(TRUSTMARK_MODEL_DIR,'resizer.onnx')), options);
let decoder;
try {
  decoder = await ort.InferenceSession.create(await readFile(join(TRUSTMARK_MODEL_DIR,'decoder_Q.onnx')), options);
  for(let i=2; i<process.argv.length; i+=2) {
    const file=process.argv[i], expected=process.argv[i+1];
    const pixels = await decodePNG(await readFile(file));
    const result = await detectWithSessions(pixels, {ort,decoder,resize});
    console.log(JSON.stringify({file,result,expected,backend:'onnxruntime-web WASM'}));
    assert.equal(result, expected);
  }
} finally {
  try { await decoder?.release(); } finally { await resize.release(); }
}
