from pathlib import Path
from PIL import Image,ImageDraw
import json
import sys
p=Path(sys.argv[1]);m=json.load(open(p/'metadata.json'));s=m['sprites'];atlas=Image.open(p/s['image_file']).convert('RGBA');a=next(a for a in m['attacks'] if a['id']=='v2_spear_throw');clip=next(c for c in m['animations'] if c['id']==a['animation_id']);step=next(x for x in a['steps'] if x['primitive']=='SPAWN_PROJECTILE');track=next(a for a in m['v2_profile']['attack_patterns'] if a['attack_id']=='v2_spear_throw');origin=next(x for x in track['motion_samples'] if x['active'])['views'][0];projectile=next(x for x in m['projectiles'] if x['id']==a['projectile_id']);pa=Image.open(p/projectile['image_file']).convert('RGBA');frames=[];time=0
for f in clip['frames']:
 canvas=Image.new('RGBA',(256,128),(30,34,41,255));fid=f['sprite_frame_id'];x=fid%s['columns']*s['frame_width'];y=fid//s['columns']*s['frame_height'];actor=atlas.crop((x,y,x+s['frame_width'],y+s['frame_height']));canvas.alpha_composite(actor,(24,20));elapsed=time-step['at_ms']
 if 0<=elapsed<step['duration_ms']:
  pf=projectile['first_frame']+(elapsed//projectile['frame_duration_ms'])%projectile['frame_count'];px=pf%projectile['columns']*projectile['frame_width'];py=pf//projectile['columns']*projectile['frame_height'];bullet=pa.crop((px,py,px+projectile['frame_width'],py+projectile['frame_height']));cx=24+origin['x']+elapsed/1000*step['speed']*96/62;cy=20+origin['y'];canvas.alpha_composite(bullet,(round(cx-bullet.width/2),round(cy-bullet.height/2)))
 canvas=canvas.resize((768,384),Image.Resampling.NEAREST);draw=ImageDraw.Draw(canvas);draw.text((12,8),'EXPORTED SPEAR PLAYBACK | seed 43',fill='white');draw.text((12,24),f'{time} ms | '+('wind-up' if time<450 else 'release + follow-through' if time<800 else 'recover + re-equip'),fill=(180,193,206));frames.append(canvas.convert('RGB'));time+=f['duration_ms']
frames[0].save(p/'projectile-playback.gif',save_all=True,append_images=frames[1:],duration=[f['duration_ms'] for f in clip['frames']],loop=0,optimize=False)
frames[10].save(p/'projectile-playback.png')
