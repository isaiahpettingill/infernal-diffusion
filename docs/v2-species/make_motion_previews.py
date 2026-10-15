from PIL import Image,ImageDraw
from pathlib import Path
import json
import sys
root=Path(sys.argv[1])
labels={'dancer_orc':'Orc rapper / passive body contact','dancer_wolf':'Wolf dancer / rigid joints','dancer_jelly':'Jellyfish dancer / bell groove','dancer_shade':'Shade dancer / contact only','spear_throw':'Spear seed 43 / throw + recovery','spear_thrust':'Spear seed 42 / thrust','laser':'Orc / laser cannon two-hand grip','sword':'Orc / huge sword','axe':'Troll / huge axe','chainsaw':'Robot / chainsaw','scythe':'Reaper / scythe'}
contact=Image.new('RGB',(1152,len(labels)*154),(27,31,38)); d=ImageDraw.Draw(contact)
for row,(name,label) in enumerate(labels.items()):
 p=root/name;m=json.load(open(p/'metadata.json'));s=m['sprites'];atlas=Image.open(p/s['image_file']).convert('RGBA')
 clipid='idle' if name.startswith('dancer') else ('attack_v2_spear_throw' if name=='spear_throw' else 'attack_primary')
 clip=next((a for a in m['animations'] if a['id']==clipid),next(a for a in m['animations'] if a['id']=='idle'))
 frames=[]
 for i,frame in enumerate(clip['frames']):
  canvas=Image.new('RGBA',(1152,324),(27,31,38,255));draw=ImageDraw.Draw(canvas);draw.text((12,8),label,fill='white')
  total=sum(f['duration_ms'] for f in clip['frames'][:i]);draw.text((12,26),f'{clip["id"]}   {total}ms',fill=(174,186,201))
  for v in range(4):
   fid=frame['sprite_frame_id']+v*s['direction_stride'];x=fid%s['columns']*s['frame_width'];y=fid//s['columns']*s['frame_height']
   sprite=atlas.crop((x,y,x+s['frame_width'],y+s['frame_height']))
   sprite=sprite.resize((288,288),Image.Resampling.NEAREST);canvas.alpha_composite(sprite,(v*288,36))
  frames.append(canvas.convert('RGB'))
 frames[0].save(p/'motion.gif',save_all=True,append_images=frames[1:],duration=[f['duration_ms'] for f in clip['frames']],loop=0,optimize=False)
 # Keyframes are actual baked atlas frames; no synthetic animation.
 for col,frac in enumerate([0,.22,.38,.52,.72,1.]):
  idx=min(len(frames)-1,round(frac*(len(frames)-1))); frame=clip['frames'][idx];view=0 if name in ['spear_throw','spear_thrust','sword','axe','scythe','chainsaw'] else 1
  fid=frame['sprite_frame_id']+view*s['direction_stride'];x=fid%s['columns']*s['frame_width'];y=fid//s['columns']*s['frame_height'];sprite=atlas.crop((x,y,x+s['frame_width'],y+s['frame_height'])).resize((144,144),Image.Resampling.NEAREST)
  contact.paste(sprite,(col*192+24,row*154+10),sprite)
 d.text((5,row*154+2),name,fill='white')
 print(p/'motion.gif')
contact.save(root/'motion-keyframes.png')
