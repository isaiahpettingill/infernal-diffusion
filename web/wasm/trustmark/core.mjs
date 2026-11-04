// Versioned public marker, not a signature or proof of authorship.
export const MARKER = [...'INFRNL1'].map(c => c.charCodeAt(0).toString(2).padStart(7, '0')).join('').padEnd(61, '0');
export const MAX_BYTES = 16 * 1024 * 1024;
export const MAX_PIXELS = 16 * 1024 * 1024;
export function pngDimensions(bytes) {
  const signature = [137, 80, 78, 71, 13, 10, 26, 10];
  if (bytes.length < 24 || !signature.every((b, i) => bytes[i] === b) ||
      String.fromCharCode(...bytes.slice(12, 16)) !== 'IHDR') throw new Error('Choose a PNG spritesheet.');
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width = view.getUint32(16), height = view.getUint32(20);
  if (!width || !height || width > 8192 || height > 8192 || width * height > MAX_PIXELS)
    throw new Error('PNG must be at most 8192 pixels per side and 16 megapixels.');
  return { width, height };
}
export function classify(result) {
  if (result.valid === true && result.schema === 'BCH_5' && result.data_binary === MARKER) return 'match';
  return result.valid === true ? 'other' : 'absent';
}
export function rgbTensor(samples, channels = 4) {
  const pixels = samples.length / channels, out = new Float32Array(pixels * 3);
  for (let i = 0; i < pixels; i++) {
    for (let c = 0; c < 3; c++) out[c * pixels + i] = samples[i * channels + c] / 255;
  }
  return out;
}
