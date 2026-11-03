import { MAX_BYTES, pngDimensions } from './core.mjs';
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function paeth(a,b,c) {
  const p=a+b-c, pa=Math.abs(p-a), pb=Math.abs(p-b), pc=Math.abs(p-c);
  return pa<=pb && pa<=pc ? a : pb<=pc ? b : c;
}
// Decode raw PNG samples rather than canvas: canvas discards RGB hidden by alpha.
export async function decodePNG(buffer) {
  const bytes = new Uint8Array(buffer);
  if (bytes.length > MAX_BYTES) throw new Error('PNG must be no larger than 16 MiB.');
  const {width,height} = pngDimensions(bytes);
  const view = new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  const chunks=[]; let channels, ended=false, offset=8, seenHeader=false, chunkCount=0;
  while (offset + 12 <= bytes.length) {
    if (++chunkCount > 4096) throw new Error('PNG contains too many chunks.');
    const length=view.getUint32(offset), end=offset+12+length;
    if (end>bytes.length) throw new Error('Truncated PNG chunk.');
    const type=String.fromCharCode(...bytes.slice(offset+4,offset+8));
    if (crc32(bytes.subarray(offset+4,end-4))!==view.getUint32(end-4)) throw new Error('PNG checksum failed.');
    const data=bytes.subarray(offset+8,end-4);
    if (type==='IHDR') {
      if (seenHeader || offset!==8 || length!==13) throw new Error('Invalid PNG header.');
      seenHeader=true;
      if (data[8]!==8 || ![2,6].includes(data[9]) || data[10]!==0 || data[11]!==0 || data[12]!==0)
        throw new Error('Use an 8-bit RGB or RGBA non-interlaced PNG. Palette, grayscale, and interlaced PNGs are unsupported.');
      channels=data[9]===6 ? 4 : 3;
    } else if (type==='IDAT') chunks.push(data);
    else if (type==='IEND') {ended=true;break;}
    else if (type==='acTL') throw new Error('Animated PNGs are unsupported.');
    else if (type!=='PLTE' && (bytes[offset+4]&32)===0) throw new Error('Unsupported critical PNG chunk.');
    offset=end;
  }
  if (!ended || !channels || !chunks.length) throw new Error('Incomplete PNG.');
  const stride=width*channels, expected=(stride+1)*height;
  const raw=new Uint8Array(expected);
  const reader=new Blob(chunks).stream().pipeThrough(new DecompressionStream('deflate')).getReader();
  let received=0;
  try {
    while (true) {
      const {value,done}=await reader.read(); if (done) break;
      if (received+value.length>expected) throw new Error('PNG expands beyond its declared dimensions.');
      raw.set(value,received); received+=value.length;
    }
  } finally { await reader.cancel().catch(()=>{}); reader.releaseLock(); }
  if (received!==expected) throw new Error('Incomplete PNG pixel data.');
  const samples=new Uint8Array(height*stride);
  for(let y=0;y<height;y++) {
    const filter=raw[y*(stride+1)];
    if(filter>4) throw new Error('Invalid PNG row filter.');
    for(let x=0;x<stride;x++) {
      const i=y*stride+x, a=x>=channels?samples[i-channels]:0, b=y?samples[i-stride]:0, c=y&&x>=channels?samples[i-stride-channels]:0;
      const predictor=filter===0?0:filter===1?a:filter===2?b:filter===3?Math.floor((a+b)/2):paeth(a,b,c);
      samples[i]=(raw[y*(stride+1)+1+x]+predictor)&255;
    }
  }
  return {data:samples,width,height,channels};
}
