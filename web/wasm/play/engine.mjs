// Small input-driven adapter for the existing generator's declarative contracts.
// No AI controller, parser, synthesizer or graphics engine lives here.
export const FLOOR = 430;
export const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
export function animationBindings(animations) {
  const keys = ['Digit1','Digit2','Digit3','Digit4','Digit5','Digit6','Digit7','Digit8','Digit9','Digit0', ...'QERTYUIOPFGHJKLZXCVBNM'.split('').map(c => `Key${c}`)];
  return [...animations].sort((a,b)=>a.id.localeCompare(b.id)).map((clip,index)=> {
    const page = Math.floor(index / keys.length);
    return {id:clip.id, code:keys[index%keys.length], shift:!!(page&1), alt:!!(page&2), page:Math.floor(page/4), label:`${page&2?'Alt+':''}${page&1?'Shift+':''}${keys[index%keys.length].replace(/Key|Digit/g,'')}`};
  });
}
export const duration = clip => clip?.frames.reduce((n,f)=>n+Math.max(1,f.duration_ms),0)||1;
export function frameAt(clip, clock) {
  if (!clip?.frames.length) return 0;
  let time = clip.looped ? clock%duration(clip) : Math.min(clock,duration(clip)-1);
  for(const frame of clip.frames) {time-=Math.max(1,frame.duration_ms);if(time<0)return frame.sprite_frame_id;}
  return clip.frames.at(-1).sprite_frame_id;
}
export function createSession(pkg) {
  const monster=pkg.monster;
  if(!monster?.animations?.length || !monster.sprites)throw new Error('Package has no playable animation data');
  return {pkg,monster,x:260,y:FLOOR,vy:0,facing:1,clock:0,clip:monster.animations.find(c=>c.semantic_state==='IDLE')||monster.animations[0],action:null,shots:[],cooldowns:{},dummy:{x:720,y:FLOOR,hp:1000,total:0,last:0},events:[],feedback:'',manual:false};
}
export const scale = m => m.size?.world_scale>0?m.size.world_scale:2;
export const length = (m,v) => v*(m.size?.pixels_per_unit||1)*scale(m);
export function anchor(m) {
  const dir=direction(m), support=(m.colliders||[]).flatMap(c=>{const v=c.views?.find(v=>v.direction_index===dir);return v?[{id:c.node_id,bottom:v.y+v.radius}]:[];});
  const feet=support.filter(v=>/^(?:mount_)*foot(?:_|$)/.test(v.id));
  return {x:m.size?.anchor_x_px??m.sprites.frame_width/2,y:(feet.length?feet:support).length?Math.max(...(feet.length?feet:support).map(v=>v.bottom)):(m.size?.anchor_y_px??m.sprites.frame_height*.57)};
}
export function direction(m) {return (m.sprites.direction_angles_deg||[0]).reduce((best,a,i,all)=>Math.abs(((a+180)%360+360)%360-180)<Math.abs(((all[best]+180)%360+360)%360-180)?i:best,0);}
export function origin(s,attack,time) {
  const track=s.monster.v2_profile?.attack_patterns?.find(p=>p.attack_id===attack.id)?.motion_samples||[];
  const sample=track.filter(p=>p.active).reduce((best,p)=>!best||Math.abs(p.time_ms-time)<Math.abs(best.time_ms-time)?p:best,null);
  const view=sample?.views?.find(v=>v.direction_index===direction(s.monster)) || s.monster.colliders?.find(c=>c.node_id===attack.origin_node)?.views?.find(v=>v.direction_index===direction(s.monster));
  const a=anchor(s.monster), z=scale(s.monster);
  return view?{x:s.x+(view.x-a.x)*z*s.facing,y:s.y+(view.y-a.y)*z,radius:view.radius*z}:{x:s.x,y:s.y-(s.monster.size?.target_height_px||90)*.65,radius:0};
}
export function exercise(s,id) {
  const clip=s.monster.animations.find(c=>c.id===id);if(!clip)return false;
  const attacks=s.monster.attacks?.filter(a=>a.animation_id===id)||[];
  if(attacks.some(a=>(s.cooldowns[a.id]||0)>0)){s.feedback='Attack cooling down';return false;}
  s.clip=clip;s.clock=0;s.manual=true;s.action={attacks,done:new Set(),hit:new Set()};
  for(const a of attacks)s.cooldowns[a.id]=Math.max(0,a.cooldown_ms||0);
  s.events.push({kind:'animation',id,semantic:clip.semantic_state});return true;
}
function hit(s,amount,id){if(s.dummy.hp<=0)return;s.dummy.last=Math.max(0,amount);s.dummy.total+=s.dummy.last;s.dummy.hp=Math.max(0,s.dummy.hp-s.dummy.last);s.feedback=`${id}: ${s.dummy.last} damage`;s.events.push({kind:'hit',id,amount:s.dummy.last});}
function circleDummy(s,x,y,r){const d=s.dummy;return Math.hypot(x-clamp(x,d.x-22,d.x+22),y-clamp(y,d.y-100,d.y))<=r;}
function step(s,a,p,key){
  const o=origin(s,a,p.at_ms||0), kind=p.primitive;
  if(['TELEGRAPH','COOLDOWN','RECOVER','WAIT'].includes(kind))return;
  if(kind==='SPAWN_PROJECTILE'){
    const speed=length(s.monster,p.speed||180), count=clamp(p.count||1,1,16), spread=(p.spread_deg||0)*Math.PI/180;
    for(let i=0;i<count;i++){const angle=(s.facing<0?Math.PI:0)+(count>1?(i/(count-1)-.5)*spread:0);s.shots.push({x:o.x,y:o.y,vx:Math.cos(angle)*speed,vy:Math.sin(angle)*speed,radius:Math.max(2,length(s.monster,p.radius||3)),life:3,damage:a.damage,id:a.id});}
  } else if(['SWIPE','BITE','THRUST','SLAM','HIT','STRIKE','CONTACT'].includes(kind)){
    const reach=Math.max(o.radius,length(s.monster,p.radius||a.range||20));
    if(!s.action.hit.has(a.id)&&circleDummy(s,o.x,o.y,reach)&& (s.dummy.x-s.x)*s.facing>=-22){hit(s,a.damage,a.id);s.action.hit.add(a.id);}else if(!s.action.hit.has(a.id)) s.feedback=`${a.id}: miss (${Math.round(o.x)}, ${Math.round(o.y)}; reach ${Math.round(reach)})`;
  } else if(['DASH','LEAP','SWOOP','BURROW','DODGE'].includes(kind)){
    s.action.motion={start:s.x,at:p.at_ms||0,duration:Math.max(1,p.duration_ms||300),distance:Math.min(300,length(s.monster,(p.speed||100))*(p.duration_ms||300)/1000),kind};
  } else if(kind==='TELEPORT'){s.x=clamp(s.x+s.facing*160,40,960);}
  else s.feedback=`${kind}: animation only in this harness`;
  s.events.push({kind:'attack',id:a.id,primitive:kind,element:a.element});
}
export function tick(s,dt,input={}) {
  dt=clamp(dt,0,.05);const ms=dt*1000;
  for(const id in s.cooldowns)s.cooldowns[id]=Math.max(0,s.cooldowns[id]-ms);
  const before=s.clock;s.clock+=ms;
  if(s.action){
    for(const a of s.action.attacks){
      const steps=a.steps?.length?a.steps:[{primitive:a.delivery==='PROJECTILE'?'SPAWN_PROJECTILE':'HIT',at_ms:a.telegraph_ms||0}];
      steps.forEach((p,i)=>{const key=`${a.id}:${i}`;if(!s.action.done.has(key)&&(p.at_ms||0)<=s.clock){s.action.done.add(key);step(s,a,p,key);}});
    }
    const motion=s.action.motion;if(motion){const p=clamp((s.clock-motion.at)/motion.duration,0,1);const travel=motion.kind==='DODGE'?(p<.45?-motion.distance*p/.45:-motion.distance+motion.distance*1.4*(p-.45)/.55):motion.distance*p;s.x=clamp(motion.start+s.facing*travel,40,960);if(['LEAP','SWOOP'].includes(motion.kind))s.y=FLOOR-Math.sin(p*Math.PI)*100;}
    if(s.clock>=duration(s.clip))s.action=null;
  }
  if(input.move && !s.action && !s.monster.tags?.includes('ANCHORED')) {s.facing=Math.sign(input.move);s.x=clamp(s.x+input.move*180*dt,40,960);s.manual=false;}
  if(input.jump && s.y>=FLOOR){s.vy=-400;s.manual=false;}
  if(!s.action?.motion){s.vy+=1000*dt;s.y=Math.min(FLOOR,s.y+s.vy*dt);if(s.y===FLOOR)s.vy=0;}
  if(!s.manual&&!s.action){const desired=s.y<FLOOR?'JUMP':input.move?'MOVE':'IDLE';const clip=s.monster.animations.find(c=>c.semantic_state===desired);if(clip&&s.clip!==clip){s.clip=clip;s.clock=0;}}
  for(const shot of s.shots){const segments=Math.max(1,Math.ceil(Math.hypot(shot.vx,shot.vy)*dt/8));for(let i=0;i<segments;i++){shot.x+=shot.vx*dt/segments;shot.y+=shot.vy*dt/segments;if(circleDummy(s,shot.x,shot.y,shot.radius)){hit(s,shot.damage,shot.id);shot.life=0;break;}}shot.life-=dt;}
  s.shots=s.shots.filter(p=>p.life>0&&p.x>0&&p.x<1000);
  if(s.events.length>64)s.events.splice(0,s.events.length-64);
}
// Engine-independent policy; the browser backend resolves sources lazily.
export function voicePolicy(tags=[]) {
  const has=t=>tags.includes(t);
  const chipmunk=['CHIBI','ANIME_GIRL','MERMAID','MERMAN'].some(has);
  const color=has('STORMTROOPER')?'radio':has('ROBOT')?'metallic-flanger': ['GHOST','SPECTRAL','SHADE'].some(has)?'airy-80s':'natural';
  return {register:chipmunk?'restrained-chipmunk':'natural',pitchStages:chipmunk?1:0,
    coloration:color,watery:['MERMAID','MERMAN'].some(has),wordBias:has('HUMANOID')?'increased':'normal',
    wordBank:has('STORMTROOPER')?'licensed-star-wars-name-pronunciations':has('ANIME_GIRL')?'ordinary-japanese':has('HITLER')?'harmless-unrelated-words':['MERMAID','MERMAN'].some(has)?'french-esperanto':null,
    limits:'shared-bounded-gain-harshness-duration-repetition-voices',reverbTailBudget:'included',status:'runtime-policy'};
}
export function audioRecipe(monster,seed,profile){return {schema:1,profile,seed:`audio-v2:${seed}:${monster.id}`,voice:null,policy:voicePolicy(monster.tags),death:{asset:'wilhelm-scream',status:'optional-source-policy',weight:monster.tags?.includes('HUMANOID')?'increased-normalized':'normal',variation:profile==='v2'||monster.tags?.includes('HUMANOID')?'transform-usually':'fallback-common',cooldownGroup:'source:wilhelm-scream'}};}
