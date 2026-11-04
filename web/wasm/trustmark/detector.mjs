import { rgbTensor, classify } from './core.mjs';
import { DataLayer_Decode, eccengine } from './vendor/data-layer.mjs';
import { configureResize, runResizeModelSquare } from './vendor/resize.mjs';
// Pinned official Microsoft runtime. No dependency/model fetch until Detect is clicked.
const RUNTIME = 'https://cdn.jsdelivr.net/npm/onnxruntime-web@1.20.1/dist/';
const MODELS = 'https://cai-watermark.adobe.net/watermarking/trustmark-models/';
let loading;
function load() {
  if (!loading) loading = (async () => {
    const ort = await import(`${RUNTIME}ort.webgpu.min.mjs`);
    ort.env.wasm.wasmPaths = RUNTIME;
    ort.env.wasm.numThreads = 1;
    // Resize requires WASM antialias support. The decoder prefers WebGPU for its float16 weights.
    const resize = await ort.InferenceSession.create(`${MODELS}resizer.onnx`, { executionProviders: ['wasm'] });
    let decoder;
    try {
      try {
        if (!globalThis.navigator?.gpu) throw new Error('WebGPU unavailable');
        decoder = await ort.InferenceSession.create(`${MODELS}decoder_Q.onnx`, { executionProviders: ['webgpu'] });
      } catch {
        decoder = await ort.InferenceSession.create(`${MODELS}decoder_Q.onnx`, { executionProviders: ['wasm'] });
      }
    } catch (error) { await resize.release(); throw error; }
    return { ort, decoder, resize };
  })().catch(error => { loading = null; throw error; });
  return loading;
}
export async function detect(imageData) {
  return detectWithSessions(imageData, await load());
}

// Also used by the actual ONNX WASM integration test (no mock inference).
export async function detectWithSessions(imageData, { ort, decoder, resize }) {
  configureResize(ort, resize);
  const input = new ort.Tensor('float32', rgbTensor(imageData.data, imageData.channels), [1, 3, imageData.height, imageData.width]);
  let resized, results;
  try {
    resized = await runResizeModelSquare(input, 256, false);
    // Native TrustMark ModelImage uses [-1, 1]; the Adobe resizer only clips to [0, 1].
    for (let i = 0; i < resized.data.length; i++) resized.data[i] = resized.data[i] * 2 - 1;
    results = await decoder.run({ image: resized });
    const output = results.output;
    const values = output.data;
    if (values.length !== 100 || !Array.from(values).every(Number.isFinite)) throw new Error('Unexpected TrustMark model output.');
    return classify(DataLayer_Decode(Array.from(values, v => Number(v >= 0)), eccengine, 'Q'));
  } finally {
    input.dispose();
    resized?.dispose();
    if (results) for (const tensor of Object.values(results)) tensor.dispose();
  }
}
