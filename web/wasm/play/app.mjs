import {MonsterAudio} from './audio.mjs';
import {MonsterGenerator} from '../client.js';
import {createSession,exercise,tick,animationBindings,frameAt,scale,anchor,direction,audioRecipe,FLOOR} from './engine.mjs';
const $=id=>document.getElementById(id), canvas=$('arena'),ctx=canvas.getContext('2d');
const sound=new MonsterAudio({onStatus:text=>{$('audio-status').textContent=text;showCredits();}});window.__audioDiagnostics=sound.diagnostics;
const generator=new MonsterGenerator();let session=null,sheet=null,bindings=[],epoch=0,busy=false,last=0;const keys=new Set();
const typing=()=>{const e=document.activeElement;return e?.matches('input,textarea,select,[contenteditable="true"]');};
function setBusy(value){busy=value;$('generate').disabled=value;$('cancel').disabled=!value;}
function clearInput(){keys.clear();}
function showCredits(){const p=sound.provenance();$('source-credits').replaceChildren(...p.sources.map(a=>{const item=document.createElement('p');item.textContent=`${a.title} — ${Array.isArray(a.artist)?a.artist.join(', '):a.artist} · ${a.license_id}. ${a.changes.join('; ')}`;const link=document.createElement('a');link.href=a.source_url;link.textContent=' Source';link.target='_blank';link.rel='noopener';item.append(link);const license=document.createElement('a');license.href=a.license_url;license.textContent=' License';license.target='_blank';license.rel='noopener';item.append(license);return item;}));}
function configureSound(){sound.configure({mode:$('license-mode').value,monster:session?.monster,seed:session?.audioInputs.seed??$('seed').value,profile:session?.audioInputs.profile??$('profile').value});showCredits();}
$('audio-enable').addEventListener('change',async()=>{try{await sound.enable($('audio-enable').checked);if(!sound.assets.length)await sound.load();$('audio-status').textContent=$('audio-enable').checked?'Sound on':'Sound off';}catch(error){$('audio-status').textContent=error.message;$('audio-enable').checked=false;await sound.enable(false);}});
$('license-mode').addEventListener('change',()=>{configureSound();$('audio-status').textContent='License mode changed; old voices stopped';});
$('credits').addEventListener('click',()=>{const url=URL.createObjectURL(new Blob([JSON.stringify(sound.provenance(),null,2)],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='monster.audio-provenance.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);});
function status(text){$('status').textContent=text;}
function drawAtlas(atlas){if(atlas.width*atlas.height>16*1024*1024)throw new Error('Sprite atlas exceeds playground memory budget');const c=document.createElement('canvas');c.width=atlas.width;c.height=atlas.height;c.getContext('2d').putImageData(new ImageData(new Uint8ClampedArray(atlas.rgba),atlas.width,atlas.height),0,0);return c;}
$('form').addEventListener('submit',async e=>{e.preventDefault();if(busy)return;const own=++epoch,request={prompt:$('prompt').value,seed:$('seed').value,profile:$('profile').value};clearInput();sound.stop();if(session)session.events=[];setBusy(true);status('Loading generator…');try{
  await generator.ready({onProgress:p=>{if(own===epoch)status(p.phase+'…');}});
  const result=await generator.generate(request.prompt,request.seed,{profile:request.profile,compact:true,onProgress:p=>{if(own===epoch)status(p.phase+'…');}});
  if(own!==epoch)return;const pkg=result.packages.find(p=>p.path==='.'||p.path==='')||result.packages[0];
  const replacement=drawAtlas(pkg.sprites);session=createSession(pkg);session.audioInputs=request;configureSound();sheet=replacement;bindings=animationBindings(session.monster.animations);$('name').textContent=session.monster.display_name;
  $('bank').replaceChildren(...Array.from({length:Math.max(...bindings.map(b=>b.page))+1},(_,i)=>{const o=document.createElement('option');o.value=i;o.textContent=i+1;return o;}));
  $('animations').replaceChildren(...bindings.map(b=>{const button=document.createElement('button');button.textContent=`${b.page?`Page ${b.page+1} · `:''}${b.label} · ${b.id}`;button.dataset.animationId=b.id;button.dataset.shortcut=b.label;button.title=session.monster.animations.find(a=>a.id===b.id).semantic_state;button.addEventListener('click',()=>{exercise(session,b.id);canvas.focus();});return button;}));
  $('audio').textContent=JSON.stringify(audioRecipe(session.monster,request.seed,request.profile),null,2);status('Ready — click the room to play');canvas.focus();
}catch(error){if(own===epoch)status(error.name==='AbortError'?'Cancelled':error.message);}finally{if(own===epoch)setBusy(false);}});
$('cancel').addEventListener('click',()=>{epoch++;generator.cancel();setBusy(false);status('Cancelled');});
$('reset').addEventListener('click',()=>{if(session){session.dummy={x:720,y:FLOOR,hp:1000,total:0,last:0};session.shots=[];session.feedback='Dummy reset';}});
$('stop').addEventListener('click',()=>{sound.stop();if(session){session.events=[];session.manual=false;session.action=null;session.clock=0;session.y=FLOOR;session.vy=0;session.shots=[];session.cooldowns={};session.feedback='Idle';}canvas.focus();});
window.addEventListener('keydown',e=>{if(typing()||busy||!session||e.ctrlKey||e.metaKey||e.repeat)return;if(['ArrowLeft','ArrowRight','KeyA','KeyD','Space'].includes(e.code)){e.preventDefault();keys.add(e.code);return;}const b=bindings.find(b=>b.page===Number($('bank').value)&&b.code===e.code&&b.shift===e.shiftKey&&b.alt===e.altKey);if(b){e.preventDefault();exercise(session,b.id);}});
window.addEventListener('keyup',e=>keys.delete(e.code));window.addEventListener('blur',()=>{clearInput();sound.stop();});document.addEventListener('visibilitychange',()=>{clearInput();if(document.hidden)sound.stop();});document.addEventListener('focusin',()=>{if(typing())clearInput();});
window.addEventListener('pagehide',()=>{epoch++;generator.cancel();clearInput();sound.stop();});
function paint(){ctx.fillStyle='#191f2c';ctx.fillRect(0,0,1000,500);ctx.fillStyle='#384357';ctx.fillRect(0,FLOOR,1000,70);ctx.strokeStyle='#263246';for(let x=0;x<1000;x+=50){ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,FLOOR);ctx.stroke();}if(!session)return;
  const d=session.dummy;ctx.fillStyle=d.hp?'#bd905b':'#584a41';ctx.fillRect(d.x-20,d.y-100,40,78);ctx.fillRect(d.x-5,d.y-22,10,22);ctx.fillStyle='#242b3b';ctx.fillRect(d.x-40,d.y-120,80,7);ctx.fillStyle='#91ca9a';ctx.fillRect(d.x-40,d.y-120,80*d.hp/1000,7);
  const m=session.monster,s=m.sprites,id=frameAt(session.clip,session.clock)+direction(m)*(s.direction_stride||0),region=s.frame_regions?.[id],z=scale(m),a=anchor(m);ctx.save();ctx.imageSmoothingEnabled=false;ctx.translate(Math.round(session.x),Math.round(session.y));ctx.scale(session.facing,1);
  const r=region||{x:(id%s.columns)*s.frame_width,y:Math.floor(id/s.columns)*s.frame_height,width:s.frame_width,height:s.frame_height,offset_x:0,offset_y:0};ctx.drawImage(sheet,r.x,r.y,r.width,r.height,(r.offset_x-a.x)*z,(r.offset_y-a.y)*z,r.width*z,r.height*z);ctx.restore();
  ctx.fillStyle='#ffba75';for(const p of session.shots){ctx.beginPath();ctx.arc(p.x,p.y,Math.min(p.radius,25),0,Math.PI*2);ctx.fill();}
  canvas.dataset.x=String(session.x);canvas.dataset.animation=session.clip.id;
  $('damage').textContent=`Dummy ${d.hp} / 1000 · Total damage ${d.total} · ${session.feedback}`;
}
function frame(now){const dt=last?Math.min((now-last)/1000,.05):0;last=now;if(session&&!busy&&!document.hidden){const move=Number(keys.has('KeyD')||keys.has('ArrowRight'))-Number(keys.has('KeyA')||keys.has('ArrowLeft'));tick(session,dt,{move,jump:keys.has('Space')});keys.delete('Space');for(const event of session.events.splice(0))void sound.trigger(event);}paint();requestAnimationFrame(frame);}requestAnimationFrame(frame);
