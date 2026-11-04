import test from 'node:test';
import assert from 'node:assert/strict';
import { MARKER, pngDimensions, classify, rgbTensor } from '../trustmark/core.mjs';
import { BCH, BCH_Encode } from '../trustmark/vendor/bch-ecc.mjs';
import { DataLayer_Decode, eccengine } from '../trustmark/vendor/data-layer.mjs';
function encoded(payload = MARKER) {
  const bits = [...payload].map(Number);
  const ecc = BCH_Encode(BCH(5, 137), bits).flatMap(byte => [...byte.toString(2).padStart(8, '0')].map(Number)).slice(0, 35);
  return [...bits, ...ecc, 0, 0, 0, 1];
}
test('exact versioned 61-bit public marker with strict schema', () => {
  assert.equal(MARKER.length, 61);
  assert.equal(MARKER.slice(49), '000000000000');
  assert.equal(classify({valid:true, schema:'BCH_5', data_binary:MARKER}), 'match');
  assert.equal(classify({valid:true, schema:'BCH_4', data_binary:MARKER}), 'other');
  assert.equal(classify({valid:false, schema:'BCH_5', data_binary:MARKER}), 'absent');
  assert.equal(classify({valid:true, schema:'BCH_5', data_binary:MARKER.slice(0,-1)+'1'}), 'other');
});
test('Adobe BCH decoder reads marker and corrects bit damage', () => {
  const bits = encoded();
  assert.equal(bits.length, 100);
  assert.equal(classify(DataLayer_Decode(bits, eccengine, 'Q')), 'match');
  for (const i of [0, 13, 40]) bits[i] ^= 1;
  assert.equal(classify(DataLayer_Decode(bits, eccengine, 'Q')), 'match');
  assert.equal(classify(DataLayer_Decode(encoded('0'.repeat(61)), eccengine, 'Q')), 'other');
});
test('raw RGB tensor retains invisible colors and ignores alpha', () => {
  const value = rgbTensor(new Uint8Array([255,0,40,0, 255,128,0,128]));
  assert.deepEqual(Array.from(value), [1, 1, 0, Math.fround(128/255), Math.fround(40/255), 0]);
});
test('PNG header dimension bounds before image decode', () => {
  const data = new Uint8Array(24);
  data.set([137,80,78,71,13,10,26,10]); data.set([73,72,68,82],12);
  const v = new DataView(data.buffer); v.setUint32(16,256); v.setUint32(20,512);
  assert.deepEqual(pngDimensions(data),{width:256,height:512});
  v.setUint32(16,8193); assert.throws(() => pngDimensions(data), /8192/);
  v.setUint32(16,8192); v.setUint32(20,8192); assert.throws(() => pngDimensions(data), /megapixels/);
  assert.throws(() => pngDimensions(new Uint8Array(24)), /PNG/);
});

import { readFile } from 'node:fs/promises';
const uiSource = (await readFile(new URL('../trustmark/ui.mjs', import.meta.url), 'utf8'))
  .replace("'./core.mjs'", JSON.stringify(new URL('../trustmark/core.mjs', import.meta.url).href))
  .replace("'./png.mjs'", JSON.stringify(new URL('../trustmark/png.mjs', import.meta.url).href))
  .replace("await import('./detector.mjs')", '({ detect: globalThis.mockDetect })');
function element() {
  return { listeners: {}, files: [], value: '', disabled: false, textContent: '',
    addEventListener(name, fn) { this.listeners[name] = fn; },
    fire(name) { return this.listeners[name](); } };
}
async function setupUI() {
  const file = element(), button = element(), reset = element(), status = element();
  const root = {querySelector: selector => ({input:file,'[data-detect]':button,'[data-reset]':reset,'[role=status]':status})[selector]};
  globalThis.document = { querySelector: () => root, createElement: () => ({
    getContext: () => ({drawImage(){},getImageData:()=>({data:new Uint8Array(4),width:1,height:1})})
  })};
  globalThis.createImageBitmap = async () => ({width:1,height:1,close(){}});
  await import('data:text/javascript;base64,' + Buffer.from(uiSource + `\n// ${Math.random()}`).toString('base64'));
  const data = new Uint8Array(24); data.set([137,80,78,71,13,10,26,10]); data.set([73,72,68,82],12);
  const view = new DataView(data.buffer); view.setUint32(16,1);view.setUint32(20,1);
  file.files = [new Blob([makePNG()])]; file.fire('change');
  return {file,button,reset,status};
}
test('UI rejects invalid input and reports errors without claiming absence', async () => {
  const {file,button,status} = await setupUI();
  file.files = [new Blob(['not PNG'])];
  await button.fire('click');
  assert.match(status.textContent, /Could not check/);
  assert.match(status.textContent, /No detection result/);
});
test('UI suppresses stale result after Clear and prevents repeated inference', async () => {
  let release, started;
  const entered = new Promise(resolve => {started = resolve;});
  globalThis.mockDetect = () => {started();return new Promise(resolve => {release = resolve;});};
  const {file,button,reset,status} = await setupUI();
  const running = button.fire('click'); await entered;
  assert.equal(button.disabled,true);
  await button.fire('click'); // busy: no second inference
  file.files=[]; reset.fire('click');
  release('match'); await running;
  assert.equal(status.textContent,'Choose a PNG spritesheet to check.');
  assert.equal(button.disabled,true);
});
test('UI replacement suppresses old result and allows new check', async () => {
  let release, started;
  const entered = new Promise(resolve => {started = resolve;});
  globalThis.mockDetect = () => {started();return new Promise(resolve => {release = resolve;});};
  const {file,button,status} = await setupUI();
  const running = button.fire('click');await entered;
  file.fire('change'); release('match');await running;
  assert.match(status.textContent,/Ready/); assert.equal(button.disabled,false);
  globalThis.mockDetect=async()=> 'other'; await button.fire('click');
  assert.match(status.textContent,/does not match/);
});

import { deflateSync } from 'node:zlib';
import { decodePNG } from '../trustmark/png.mjs';
function crc(bytes) {
  let c=0xffffffff;
  for(const b of bytes){c^=b;for(let i=0;i<8;i++)c=(c>>>1)^((c&1)?0xedb88320:0);}
  return (c^0xffffffff)>>>0;
}
function chunk(type,data) {
  const out=Buffer.alloc(data.length+12);out.writeUInt32BE(data.length);out.write(type,4);out.set(data,8);
  out.writeUInt32BE(crc(out.subarray(4,-4)),out.length-4);return out;
}
function makePNG({filter=0, color=6, rows=Buffer.from([0,255,2,33,0]), height=1, width=1, extraChunks=[]}={}) {
 const header=Buffer.alloc(13);header.writeUInt32BE(width);header.writeUInt32BE(height,4);header[8]=8;header[9]=color;
 return Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]),chunk('IHDR',header),...extraChunks,chunk('IDAT',deflateSync(rows)),chunk('IEND',Buffer.alloc(0))]);
}
test('raw PNG decoder preserves transparent RGB and reconstructs all row filters', async()=>{
 for(let filter=0;filter<=4;filter++) {
  // A one-pixel first row has zero left/up predictors for every filter.
  const image=await decodePNG(makePNG({rows:Buffer.from([filter,255,2,33,0])}));
  assert.deepEqual(Array.from(image.data),[255,2,33,0]);
 }
 const up=await decodePNG(makePNG({height:2,rows:Buffer.from([0,255,2,33,0, 2,1,3,4,128])}));
 assert.deepEqual(Array.from(up.data),[255,2,33,0,0,5,37,128]);
 const rgb=await decodePNG(makePNG({color:2,rows:Buffer.from([0,1,2,3])}));
 assert.equal(rgb.channels,3);assert.deepEqual(Array.from(rgb.data),[1,2,3]);
});
test('raw PNG decoder rejects unsupported, corrupt and oversized decoded data',async()=>{
 await assert.rejects(decodePNG(makePNG({color:0})),/unsupported/);
 await assert.rejects(decodePNG(makePNG({rows:Buffer.alloc(100)})),/expands/);
 await assert.rejects(decodePNG(makePNG({rows:Buffer.from([5,1,2,3,4])})),/filter/);
 await assert.rejects(decodePNG(makePNG().subarray(0,40)),/Truncated|Incomplete/);
 const damaged=makePNG();damaged[30]^=1;await assert.rejects(decodePNG(damaged),/checksum/);
});

test('indexed PNG expands exact RGB including hidden color and partial/default alpha',async()=>{
 const palette=chunk('PLTE',Buffer.from([10,20,30, 40,50,60, 70,80,90]));
 const alpha=chunk('tRNS',Buffer.from([0,128]));
 for (const [filter,indices] of [[0,[0,1,2]],[1,[0,1,1]],[2,[0,1,2]],[3,[0,1,2]],[4,[0,1,1]]]) {
  const x=await decodePNG(makePNG({color:3,width:3,rows:Buffer.from([filter,...indices]),extraChunks:[palette,alpha]}));
  assert.equal(x.channels,4);
  assert.deepEqual(Array.from(x.data),[10,20,30,0,40,50,60,128,70,80,90,255]);
 }
 const opaque=await decodePNG(makePNG({color:3,rows:Buffer.from([0,1]),extraChunks:[palette]}));
 assert.deepEqual(Array.from(opaque.data),[40,50,60,255]);
});
test('indexed PNG rejects missing/malformed palettes, invalid alpha, and bad indices',async()=>{
 const palette=chunk('PLTE',Buffer.from([1,2,3]));
 const image=extraChunks=>makePNG({color:3,rows:Buffer.from([0,0]),extraChunks});
 await assert.rejects(decodePNG(image([])),/missing.*palette/);
 await assert.rejects(decodePNG(image([chunk('PLTE',Buffer.from([1,2]))])),/palette/);
 await assert.rejects(decodePNG(image([chunk('PLTE',Buffer.alloc(771))])),/palette/);
 await assert.rejects(decodePNG(image([palette,palette])),/palette/);
 await assert.rejects(decodePNG(image([chunk('tRNS',Buffer.from([0])),palette])),/transparency/);
 await assert.rejects(decodePNG(image([palette,chunk('tRNS',Buffer.from([0,1]))])),/transparency/);
 await assert.rejects(decodePNG(makePNG({color:3,rows:Buffer.from([0,1]),extraChunks:[palette]})),/index/);
});
