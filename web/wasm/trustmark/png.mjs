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
  const chunks=[]; let channels, colorType, palette, transparency, seenIDAT=false, ended=false, offset=8, seenHeader=false, chunkCount=0;
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
      if (data[8]!==8 || ![2,3,6].includes(data[9]) || data[10]!==0 || data[11]!==0 || data[12]!==0)
        throw new Error('Use an 8-bit RGB, RGBA, or indexed non-interlaced PNG. Grayscale and interlaced PNGs are unsupported.');
      colorType=data[9];
      channels=colorType===6 ? 4 : colorType===3 ? 1 : 3;
    } else if (type==='PLTE') {
      if (palette || seenIDAT || !length || length>768 || length%3) throw new Error('Invalid PNG palette.');
      palette=data;
    } else if (type==='tRNS') {
      if (transparency || seenIDAT || colorType===6) throw new Error('Invalid PNG transparency.');
      if (colorType===3 && (!palette || !length || length>palette.length/3)) throw new Error('Invalid PNG palette transparency.');
      if (colorType===2 && length!==6) throw new Error('Invalid PNG RGB transparency.');
      transparency=data;
    } else if (type==='IDAT') {
      if (colorType===3 && !palette) throw new Error('Indexed PNG is missing its palette.');
      seenIDAT=true; chunks.push(data);
    }
    else if (type==='IEND') {ended=true;break;}
    else if (type==='acTL') throw new Error('Animated PNGs are unsupported.');
    else if ((bytes[offset+4]&32)===0) throw new Error('Unsupported critical PNG chunk.');
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
  if (colorType===3) {
    const rgba=new Uint8Array(width*height*4), entries=palette.length/3;
    for (let i=0;i<samples.length;i++) {
      const index=samples[i];
      if (index>=entries) throw new Error('PNG palette index is out of range.');
      rgba[i*4]=palette[index*3]; rgba[i*4+1]=palette[index*3+1]; rgba[i*4+2]=palette[index*3+2];
      rgba[i*4+3]=transparency && index<transparency.length ? transparency[index] : 255;
    }
    return {data:rgba,width,height,channels:4};
  }
  if (colorType===2 && transparency) {
    const rgba=new Uint8Array(width*height*4);
    const transparent=[0,2,4].map(i=>(transparency[i]<<8)|transparency[i+1]);
    for (let i=0;i<width*height;i++) {
      rgba.set(samples.subarray(i*3,i*3+3),i*4);
      rgba[i*4+3]=transparent.every((v,c)=>v===samples[i*3+c]) ? 0 : 255;
    }
    return {data:rgba,width,height,channels:4};
  }
  return {data:samples,width,height,channels};
}
