/** Real bundled BERT/WASM cost and pixel-fidelity benchmark. No network needed.
 * node --expose-gc web/wasm/tests/atlas-cost.node.mjs [package-directory] [compact]
 * JSON stdout separates decoded/transfer/GPU candidate bytes from compute time.
 */
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { createHash, webcrypto } from 'node:crypto';
if (!globalThis.crypto) globalThis.crypto = webcrypto;
const root=pathToFileURL(resolve(process.argv[2] || new URL('../pkg/',import.meta.url).pathname)+'/');
const api=await import(new URL('infernal_diffusion.js',root));
await api.default({module_or_path:await readFile(new URL('infernal_diffusion_bg.wasm',root))});
api.initialize_model();
const compact=process.argv[3]==='compact';
const digest=b=>createHash('sha256').update(b).digest('hex');
const samples=[];
for(const [name,prompt] of [['slime','small ice slime'],['orc','orc with a sword'],['complex','giant orc riding a giant spider that shoots fire'],['shade','shade humanoid'],['hybrid',JSON.stringify({schema_version:1,upper_body:'STORMTROOPER',lower_body:'SPIDER',head:'FISH',lower_limb_count:8,upper_arm_count:2})]]) {
 for(const profile of name==='hybrid'?['v2']:['v1','v2']) {
  globalThis.gc?.();
  const start=performance.now();
  let result=name==='hybrid' ? (compact?api.generate_hybrid_compact_progress(prompt,42n,()=>{}):api.generate_hybrid_json(prompt,42n)) : (compact?api.generate_monster_compact_progress(prompt,42n,profile,()=>{}):api.generate_monster_with_profile(prompt,42n,profile));
  const elapsed=performance.now()-start;
  const packages=result.packages.map(p=>{
   const s=p.monster.sprites; const hash=createHash('sha256'); const regions=s.frame_regions||[];
   const frame=new Uint8Array(s.frame_width*s.frame_height*4);
   let visible=0, boundsArea=0;
   const unique=new Set();
   for(let id=0;id<s.frame_count;id++) {
    frame.fill(0);
    const r=regions[id]||{x:id%s.columns*s.frame_width,y:Math.floor(id/s.columns)*s.frame_height,width:s.frame_width,height:s.frame_height,offset_x:0,offset_y:0};
    for(let y=0;y<r.height;y++) {
     const src=((r.y+y)*p.sprites.width+r.x)*4;
     frame.set(p.sprites.rgba.subarray(src,src+r.width*4),((y+r.offset_y)*s.frame_width+r.offset_x)*4);
    }
    hash.update(frame); unique.add(digest(frame));
    let x0=s.frame_width,y0=s.frame_height,x1=0,y1=0;
    for(let y=0;y<s.frame_height;y++) for(let x=0;x<s.frame_width;x++) if(frame[(y*s.frame_width+x)*4+3]) {visible++;x0=Math.min(x0,x);y0=Math.min(y0,y);x1=Math.max(x1,x+1);y1=Math.max(y1,y+1);}
    if(x1) boundsArea+=(x1-x0)*(y1-y0);
   }
   const {sprites,...gameplay}=p.monster;
   delete gameplay.format_version;
   const stringify=o=>JSON.stringify(o,(_,v)=>typeof v==='bigint'?v.toString():v);
   return {path:p.path,frames:s.frame_count,uniqueFrames:unique.size,virtualFrame:[s.frame_width,s.frame_height],visiblePixels:visible,visibleBoundsPixels:boundsArea,
    spriteBytes:p.sprites.rgba.byteLength,emissionBytes:p.emission.rgba.byteLength,projectileBytes:p.projectiles?.rgba.byteLength||0,
    packedDimensions:[p.sprites.width,p.sprites.height],logicalPixelsSha256:hash.digest('hex'),gameplaySha256:digest(stringify(gameplay)),
    emissionEmpty:!p.emission.rgba.some(v=>v!==0)};
  });
  samples.push({name,prompt,profile,seed:42,compact,computeMs:Math.round(elapsed),rssBytes:process.memoryUsage().rss,packages});
  console.error(name,profile,Math.round(elapsed)+'ms',packages.reduce((a,p)=>a+p.spriteBytes+p.emissionBytes+p.projectileBytes,0)+' bytes');
  result=null;
 }
}
console.log(JSON.stringify({host:'Node real WASM; no browser/GPU timing implied',compact,peakRssBytes:process.resourceUsage().maxRSS*1024,samples},null,2));
