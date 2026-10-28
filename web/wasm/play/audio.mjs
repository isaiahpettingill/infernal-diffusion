// Web Audio's native DSP nodes; no copied synthesizer or unlicensed recordings.
import {voicePolicy} from './engine.mjs';
export const FREE_LICENSES=new Set(['CC0-1.0','CC0','PD-self','PD-author','Public domain']);
export const COMMERCIAL_LICENSES=new Set([...FREE_LICENSES,'CC-BY-2.0-FR','CC-BY-2.5','CC-BY-3.0','CC-BY-4.0','CC-BY-SA-3.0','CC-BY-SA-4.0']);
export function eligible(asset,mode){return asset?.verification?.source_page_verified===true && /^[a-f0-9]{64}$/.test(asset.verification.sha256||'') && COMMERCIAL_LICENSES.has(asset.license_id) && !asset.verification.blocked && (mode!=='no-attribution'||(asset.attribution_required===false&&FREE_LICENSES.has(asset.license_id)));}
export function hash(text){let n=2166136261;for(const c of text)n=Math.imul(n^c.charCodeAt(0),16777619);return n>>>0;}
export function selectAsset(assets,{mode,role,tags=[],seed,profile='v2',sequence=0}){
  const pool=assets.filter(a=>eligible(a,mode)); if(!pool.length)return null;
  const wilhelm=role==='death'&&pool.find(a=>a.id==='wilhelm-scream');
  if(wilhelm&&hash(`${seed}:death:${sequence}`)%100<(tags.includes('HUMANOID')?65:profile==='v1'?40:15))return wilhelm;
  const normalized=tags.map(t=>String(t).toLowerCase()),has=t=>normalized.includes(t);
  const desired=new Set(normalized);
  for(const [tag,aliases] of Object.entries({anime_girl:['anime','speech'],chibi:['comedy'],mermaid:['mermaid'],merman:['mermaid'],orc:['orc','grunt'],troll:['troll','grunt'],canine:['dog','canine'],feline:['cat','big-cat'],avian:['bird'],arthropod:['insect'],amphibian:['frog']}))if(has(tag))aliases.forEach(x=>desired.add(x));
  const vocal=a=>['word','phoneme','animal'].includes(a.source_kind)||a.roles?.some(t=>['grunt','voice','roar','taunt','speech'].includes(t));
  let choices=['fire','splash','flop'].includes(role)?pool.filter(a=>a.roles?.includes(role)):role==='attack'?pool.filter(a=>a.tags?.includes('GRUNT')):pool.filter(vocal);
  if(role==='death'||role==='hurt'){const explicit=choices.filter(a=>a.roles?.includes(role)||a.roles?.includes('grunt'));const family=choices.filter(a=>a.tags?.some(t=>desired.has(t.toLowerCase())));choices=family.length?family:explicit.length?explicit:choices;}
  if(!choices.length)return null;
  const words=has('humanoid'),score=a=>(a.tags||[]).reduce((n,t)=>n+(desired.has(t.toLowerCase())?(t.toLowerCase()==='humanoid'?1:4):0),0),best=Math.max(0,...choices.map(score)),preferred=best?choices.filter(a=>score(a)===best):[];
  // V1 is artfully mismatched occasionally, never louder or more abrasive.
  if(preferred.length&&(profile==='v2'||hash(`${seed}:fit:${sequence}`)%5!==0))choices=preferred;
  else if(words&&hash(`${seed}:word:${sequence}`)%4<3){const w=choices.filter(a=>['word','phoneme'].includes(a.source_kind));if(w.length)choices=w;}


  return choices[hash(`${seed}:${role}:${profile}:${sequence}`)%choices.length];
}
export function eventRole(event){if(event.kind==='animation'){const s=`${event.semantic} ${event.id}`.toLowerCase();if(/death|die/.test(s))return 'death';if(/hurt|hit|pain|impact|knockback/.test(s))return 'hurt';if(/swim|splash/.test(s))return 'splash';if(/flop|stumble|land/.test(s))return 'flop';if(/roar|taunt/.test(s))return 'voice';return null;}if(event.kind==='attack')return event.element==='FIRE'?'fire':'attack';return null;}
export class MonsterAudio {
  constructor({onStatus=()=>{},contextFactory=()=>new AudioContext(),disabledSourceIds=[]}={}){this.disabledSources=new Set(disabledSourceIds);this.onStatus=onStatus;this.contextFactory=contextFactory;this.assets=[];this.mode='unrestricted';this.enabled=false;this.generation=0;this.sequence=0;this.cache=new Map();this.decoded=new Map();this.voices=new Set();this.cooldowns=new Map();this.pending=new Set();this.used=new Map();this.diagnostics={played:0,active:0,selectedSourceIds:[],mode:this.mode,peak:0,outputRmsMax:0,decodedCount:0};}
  async load(){const r=await fetch(new URL('./audio/catalog.json',import.meta.url));if(!r.ok)throw new Error('Sound bank unavailable');const c=await r.json();if(c.schema_version!==1||!Array.isArray(c.assets))throw new Error('Invalid sound bank');this.assets=c.assets.filter(a=>!this.disabledSources.has(a.id));return this.assets;}
  async enable(value){this.enabled=value;if(!value){this.stop();return;}this.context??=this.contextFactory();if(!this.master){this.master=this.context.createGain();this.master.gain.value=.65;this.analyser=this.context.createAnalyser();this.analyser.fftSize=512;this.master.connect(this.analyser).connect(this.context.destination);}await this.context.resume();this.onStatus('Sound on');}
  stop(){this.generation++;for(const v of this.voices){try{v.stop();}catch{}}this.voices.clear();this.pending.clear();clearInterval(this.probe);this.probe=null;this.diagnostics.active=0;}
  configure({mode=this.mode,monster,seed,profile}){this.stop();this.mode=mode;this.monster=monster;this.seed=`audio-v2:${seed}:${monster?.id||''}`;this.profile=profile;this.sequence=0;this.lastEvent=-1;this.cache.clear();this.decoded.clear();this.used.clear();this.cooldowns.clear();Object.assign(this.diagnostics,{mode,played:0,selectedSourceIds:[],peak:0,outputRmsMax:0,decodedCount:0});}
  async decode(asset){const generation=this.generation;if(this.disabledSources.has(asset.id)||!eligible(asset,this.mode))throw new Error('Sound excluded by license mode');if(this.decoded.has(asset.id))return this.decoded.get(asset.id);if(!/^[a-zA-Z0-9_.-]+\.(ogg|wav|mp3)$/.test(asset.file))throw new Error('Invalid local sound path');const r=await fetch(new URL(`./audio/${asset.file}`,import.meta.url));if(!r.ok)throw new Error('Sound file unavailable');const bytes=await r.arrayBuffer();if(bytes.byteLength>8*1024*1024)throw new Error('Sound file exceeds budget');const digest=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),b=>b.toString(16).padStart(2,'0')).join('');if(digest!==asset.verification.sha256)throw new Error('Sound checksum mismatch');const buffer=await this.context.decodeAudioData(bytes);if(buffer.duration>65)throw new Error('Sound duration exceeds budget');if(generation!==this.generation||!eligible(asset,this.mode))throw new Error('Audio configuration changed');this.decoded.set(asset.id,buffer);this.diagnostics.decodedCount++;return buffer;}
  async render(asset,role,variation){
    const input=await this.decode(asset),policy=voicePolicy(this.monster?.tags),r=hash(`${this.seed}:${asset.id}:${variation}`),v2=this.profile==='v2';
    // One pitch stage even when chibi/anime/mermaid traits overlap. Playback-rate
    // transposition intentionally changes duration; this is not voice cloning.
    const semitones=policy.pitchStages?3+(r%3):v2?(r%7)-3:(r%3)-1;
    const rate=2**(semitones/12),body=Math.min(input.duration,role==='death'?1.6:role==='fire'?1.3:.85),tail=policy.coloration==='airy-80s'?.45:.18;
    const sampleRate=24000,seconds=body/rate+tail,offline=new OfflineAudioContext(1,Math.ceil(seconds*sampleRate),sampleRate);
    const source=offline.createBufferSource();source.buffer=input;source.playbackRate.value=rate;
    const hp=offline.createBiquadFilter();hp.type='highpass';hp.frequency.value=policy.coloration==='radio'?400:90;
    const lp=offline.createBiquadFilter();lp.type='lowpass';lp.frequency.value=policy.coloration==='radio'?2800:policy.watery?2400:6500;lp.Q.value=.5;
    const envelope=offline.createGain();envelope.gain.setValueAtTime(0,0);envelope.gain.linearRampToValueAtTime(.65,.018);envelope.gain.setValueAtTime(.65,Math.max(.019,body/rate-.08));envelope.gain.linearRampToValueAtTime(0,body/rate);
    source.connect(hp).connect(lp).connect(envelope);
    const mix=offline.createGain();envelope.connect(mix);
    if(policy.coloration==='metallic-flanger'||policy.watery||policy.coloration==='airy-80s'){
      const delay=offline.createDelay(.5),wet=offline.createGain(),lfo=offline.createOscillator(),depth=offline.createGain();
      delay.delayTime.value=policy.watery?.03:.008;wet.gain.value=.18;depth.gain.value=policy.watery?.005:.003;lfo.frequency.value=policy.watery?3: .65;
      lfo.connect(depth).connect(delay.delayTime);envelope.connect(delay).connect(wet).connect(mix);lfo.start();lfo.stop(seconds);
    }
    if(policy.coloration!=='natural'||v2){for(const [time,gain] of [[.075,.12],[.16,.06],[.29,policy.coloration==='airy-80s'?.055:0]]){if(!gain)continue;const d=offline.createDelay(.5),g=offline.createGain();d.delayTime.value=time;g.gain.value=gain;envelope.connect(d).connect(g).connect(mix);}}
    const compressor=offline.createDynamicsCompressor();compressor.threshold.value=-18;compressor.ratio.value=8;compressor.attack.value=.003;compressor.release.value=.09;mix.connect(compressor).connect(offline.destination);
    source.start(0,0,body);const buffer=await offline.startRendering(),data=buffer.getChannelData(0);let peak=0;for(const x of data)peak=Math.max(peak,Math.abs(x));
    // Never amplify quiet/noisy bases. Bound source gain and fade final delay tail.
    const gain=peak>.32?.32/peak:1;for(let i=0;i<data.length;i++)data[i]*=gain*Math.min(1,(data.length-1-i)/(sampleRate*.025));
    return {buffer,peak:peak*gain,parameters:{semitones,rate,pitchStages:policy.pitchStages,coloration:policy.coloration,watery:policy.watery},changes:[`Cropped to ${body.toFixed(3)}s; ${semitones} semitone rate shift`,`${policy.coloration}; ${policy.watery?'watery modulation; ':''}high/low-pass, envelope, compressor; bounded peak`],asset};
  }
  async trigger(event){
    const role=eventRole(event);if(!role||!this.enabled||!this.context||this.context.state!=='running'||this.voices.size+this.pending.size>=3)return;
    const now=this.context.currentTime;if(now<(this.lastEvent||0)+.18)return;
    const asset=selectAsset(this.assets,{mode:this.mode,role,tags:this.monster?.tags,seed:this.seed,profile:this.profile,sequence:this.sequence++});if(!asset||now<(this.cooldowns.get(asset.id)||0))return;
    this.lastEvent=now;this.cooldowns.set(asset.id,now+(role==='death'?4:1.2));const generation=this.generation,key=`${this.seed}:${this.mode}:${asset.id}:${role}:${this.profile}:${this.sequence%(this.profile==='v1'?2:6)}`,pendingKey=`${generation}:${key}`;this.pending.add(pendingKey);
    try{let rendered=this.cache.get(key);if(!rendered){rendered=await this.render(asset,role,key);if(generation!==this.generation)return;this.cache.set(key,rendered);if(this.cache.size>32)this.cache.delete(this.cache.keys().next().value);}
      if(generation!==this.generation||!this.enabled||!eligible(asset,this.mode))return;
      const source=this.context.createBufferSource();source.buffer=rendered.buffer;source.connect(this.master);this.voices.add(source);source.onended=()=>{this.voices.delete(source);source.disconnect();this.diagnostics.active=this.voices.size;if(!this.voices.size){clearInterval(this.probe);this.probe=null;}};source.start();if(!this.probe){const wave=new Float32Array(this.analyser.fftSize);this.probe=setInterval(()=>{this.analyser.getFloatTimeDomainData(wave);let energy=0;for(const x of wave)energy+=x*x;this.diagnostics.outputRmsMax=Math.max(this.diagnostics.outputRmsMax,Math.sqrt(energy/wave.length));},20);}
      this.used.set(key,{asset:rendered.asset,changes:rendered.changes,id:`voice-${hash(key).toString(16)}`});this.diagnostics.played++;this.diagnostics.active=this.voices.size;this.diagnostics.peak=rendered.peak;this.diagnostics.selectedSourceIds=[...new Set([...this.diagnostics.selectedSourceIds,asset.id])];this.onStatus(`Sound · ${asset.title}`);
    }catch(error){if(generation===this.generation)this.onStatus(`Sound unavailable: ${error.message}`);}finally{this.pending.delete(pendingKey);}
  }
  provenance(){const rendered=[...this.used.values()],sources=[...new Map(rendered.map(r=>[r.asset.id,r.asset])).values()].map(a=>({id:a.id,title:a.title,artist:a.artist,source_url:a.source_url,license_id:a.license_id,license_url:a.license_url,source_kind:a.source_kind,changes:[...(Array.isArray(a.changes)?a.changes:a.changes?[a.changes]:[]),...new Set(rendered.filter(r=>r.asset.id===a.id).flatMap(r=>r.changes))],...(a.attribution?{attribution:a.attribution}:{})}));const outputs=rendered.map(r=>({id:r.id,title:`Monster voice · ${r.asset.title}`,source_ids:[r.asset.id],license_review:'approved',changes:r.changes,effective_license_id:r.asset.license_id,effective_license_url:r.asset.license_url}));return {schema_version:1,kind:'infernal.audio.provenance',sources,banks:[],outputs,used_bank_ids:[],used_output_ids:outputs.map(o=>o.id)};}
}
