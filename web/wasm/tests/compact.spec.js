import {test,expect} from '@playwright/test';

test('real browser worker packs losslessly without removing special attacks or changing anchors', async ({page})=>{
 await page.goto('/tests/browser-check.html');
 const results=await page.evaluate(async()=>{
  const {MonsterGenerator}=await import('../client.js');
  const generator=new MonsterGenerator();
  const digest=async b=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',b))).join(',');
  const normalize=m=>{const {sprites,format_version,...rest}=m;return JSON.stringify(rest,(_,v)=>typeof v==='bigint'?v.toString():v);};
  const results=[];
  try {
   for(const profile of ['v1','v2']) {
    const dense=(await generator.generate('small ice slime',42n,{profile})).packages[0];
    const packed=(await generator.generate('small ice slime',42n,{profile,compact:true})).packages[0];
    const s=packed.monster.sprites;
    const restored=new Uint8Array(dense.sprites.rgba.length);
    for(const [id,r] of s.frame_regions.entries()) for(let y=0;y<r.height;y++) {
     const src=((r.y+y)*packed.sprites.width+r.x)*4;
     const dst=((Math.floor(id/s.columns)*s.frame_height+r.offset_y+y)*dense.sprites.width+id%s.columns*s.frame_width+r.offset_x)*4;
     restored.set(packed.sprites.rgba.subarray(src,src+r.width*4),dst);
    }
    results.push({profile,format:packed.monster.format_version,pixelEqual:await digest(restored)===await digest(dense.sprites.rgba),
     metadataEqual:normalize(dense.monster)===normalize(packed.monster),regions:s.frame_regions.length,frames:s.frame_count,
     before:dense.sprites.rgba.length+dense.emission.rgba.length,after:packed.sprites.rgba.length+packed.emission.rgba.length,
     emissionFile:s.emission_file,emissionBytes:packed.emission.rgba.length,isolated:crossOriginIsolated});
   }
  } finally {generator.dispose();}
  return results;
 });
 for(const r of results) {
  expect(r.format).toBe(8);expect(r.pixelEqual).toBe(true);expect(r.metadataEqual).toBe(true);
  expect(r.regions).toBe(r.frames);expect(r.after).toBeLessThan(r.before/3);
  expect(r.emissionFile).toBe('');expect(r.emissionBytes).toBe(4);
 }
});
