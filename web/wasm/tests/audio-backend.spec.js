import {test,expect} from '@playwright/test';

async function audioHarness(page){
  // Isolate audio without starting the BERT UI; audio modules and recordings
  // are still fetched from the real local server unchanged.
  await page.route('**/audio-test-harness',route=>route.fulfill({contentType:'text/html',body:'<button id="enable-test-audio">Enable sound</button>'}));
  await page.goto('/audio-test-harness');
  await page.evaluate(async()=>{
    const {MonsterAudio}=await import('/play/audio.mjs');
    window.audioTest=new MonsterAudio();
    await window.audioTest.load();
    document.querySelector('button').onclick=()=>{window.audioEnabled=window.audioTest.enable(true);};
  });
  await page.locator('#enable-test-audio').click();
  await page.evaluate(()=>window.audioEnabled);
  expect(await page.evaluate(()=>window.audioTest.context.state)).toBe('running');
}

test('real catalog PCM renders bounded distinct model and character effects',async({page})=>{
  await audioHarness(page);
  const rendered=await page.evaluate(async()=>{
    const audio=window.audioTest;
    const voice=audio.assets.find(a=>a.id==='human-grunts');
    const fire=audio.assets.find(a=>a.id==='fire-barbecue');
    if(!voice||!fire)throw new Error('Expected curated voice and fire sources are missing');
    const specs=[
      ['natural-v1',[],'v1',voice,'hurt'],['natural-v2',[],'v2',voice,'hurt'],
      ['robot',['ROBOT'],'v2',voice,'hurt'],['ghost',['GHOST'],'v2',voice,'hurt'],
      ['mermaid',['MERMAID'],'v2',voice,'hurt'],
      ['overlap',['MERMAID','CHIBI','ANIME_GIRL'],'v2',voice,'hurt'],
      ['fire',[],'v2',fire,'fire'],
    ];
    const results=[];
    for(const [name,tags,profile,asset,role] of specs){
      audio.configure({monster:{id:'audio-test',tags},seed:'42',profile});
      const result=await audio.render(asset,role,'same-variation');
      const pcm=result.buffer.getChannelData(0);
      let peak=0,energy=0,finite=true;
      for(const sample of pcm){finite&&=Number.isFinite(sample);peak=Math.max(peak,Math.abs(sample));energy+=sample*sample;}
      const fingerprint=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',pcm.buffer)),b=>b.toString(16).padStart(2,'0')).join('');
      results.push({name,peak,energy,finite,fingerprint,parameters:result.parameters,duration:result.buffer.duration,channels:result.buffer.numberOfChannels});
    }
    await audio.enable(false);
    await audio.context.close();
    return results;
  });
  await test.info().attach('audio-render-measurements',{body:JSON.stringify(rendered,null,2),contentType:'application/json'});
  for(const item of rendered){
    expect(item.finite,`${item.name} finite PCM`).toBe(true);
    expect(item.energy,`${item.name} audible signal energy`).toBeGreaterThan(0);
    expect(item.peak).toBeGreaterThan(0);
    expect(item.peak).toBeLessThanOrEqual(.320001);
    expect(item.duration).toBeGreaterThan(.02);
    expect(item.duration).toBeLessThan(3);
    expect(item.channels).toBe(1);
  }
  const byName=Object.fromEntries(rendered.map(item=>[item.name,item]));
  expect(byName['natural-v1'].fingerprint).not.toBe(byName['natural-v2'].fingerprint);
  const distinct=['natural-v2','robot','ghost','mermaid','fire'].map(name=>byName[name].fingerprint);
  expect(new Set(distinct).size).toBe(distinct.length);
  // Multiple chipmunk-like traits use one pitch stage, not stacked transposition.
  expect(byName.overlap.parameters).toEqual(byName.mermaid.parameters);
  expect(byName.overlap.parameters.pitchStages).toBe(1);
  expect(byName.overlap.parameters.watery).toBe(true);
  expect(byName.overlap.duration).toBeCloseTo(byName.mermaid.duration,5);
  // Browser decoding/resampling and native DSP need not be bit-identical.
  expect(Math.abs(byName.overlap.energy-byName.mermaid.energy)/byName.mermaid.energy).toBeLessThan(.02);
});

test('mode switch discards a real restricted decode already in flight',async({page})=>{
  await audioHarness(page);
  await page.evaluate(async()=>{
    const audio=window.audioTest;
    const restricted=audio.assets.find(a=>a.id==='schwa');
    if(!restricted||restricted.attribution_required!==true)throw new Error('Missing attribution-required test source');
    audio.assets=[restricted];
    audio.configure({monster:{id:'race-test',tags:['HUMANOID']},seed:'42',profile:'v2'});
    const decode=audio.context.decodeAudioData.bind(audio.context);
    audio.context.decodeAudioData=async bytes=>{
      const actual=await decode(bytes); // Actual native decoder, not fabricated PCM.
      window.realDecodeFinished=true;
      await new Promise(resolve=>{window.releaseRealDecode=resolve;});
      return actual;
    };
    window.pendingSound=audio.trigger({kind:'animation',semantic:'DEATH',id:'death'});
  });
  await page.waitForFunction(()=>window.realDecodeFinished===true);
  await page.evaluate(()=>{
    const audio=window.audioTest;
    audio.configure({mode:'no-attribution',monster:{id:'race-test',tags:['HUMANOID']},seed:'42',profile:'v2'});
    window.releaseRealDecode();
  });
  await page.evaluate(()=>window.pendingSound);
  const result=await page.evaluate(async()=>{
    const audio=window.audioTest;
    const result={diagnostics:{...audio.diagnostics},used:audio.used.size,pending:audio.pending.size,restrictedCached:audio.decoded.has('schwa')};
    await audio.enable(false);await audio.context.close();return result;
  });
  expect(result.diagnostics.mode).toBe('no-attribution');
  expect(result.diagnostics.played).toBe(0);
  expect(result.diagnostics.active).toBe(0);
  expect(result.diagnostics.selectedSourceIds).toEqual([]);
  expect(result.used).toBe(0);
  expect(result.pending).toBe(0);
  expect(result.restrictedCached).toBe(false);
});
