"""Timing-aware GIFs of actual baked clips. Usage: SCRIPT CATALOG OUTPUT_DIR."""
from pathlib import Path
import json
import sys
from PIL import Image, ImageDraw
root, out = Path(sys.argv[1]), Path(sys.argv[2]); out.mkdir(parents=True, exist_ok=True)
def load(name, version):
 p=root/name/version
 return json.loads((p/'metadata.json').read_text()), Image.open(p/'sprites.png').convert('RGBA')
def clip_frame(package, clip_id, milliseconds):
 m, atlas=package; clip=next(c for c in m['animations'] if c['id']==clip_id)
 total=sum(f['duration_ms'] for f in clip['frames'])
 t=milliseconds%total if clip['looped'] else min(milliseconds,total-1)
 frame=clip['frames'][-1]
 for candidate in clip['frames']:
  frame=candidate
  if t<candidate['duration_ms']: break
  t-=candidate['duration_ms']
 s=m['sprites'];w=s['frame_width'];h=s['frame_height'];i=frame['sprite_frame_id'];x=i%s['columns']*w;y=i//s['columns']*h
 return atlas.crop((x,y,x+w,y+h))
def paste(canvas, image, x,y):
 # Preserve the original complete frame, and use nearest-neighbor enlargement.
 image=image.resize((image.width*2,image.height*2),Image.Resampling.NEAREST)
 canvas.paste(image,(x+(384-image.width)//2,y),image)
water=[load('mermaid','v1'),load('mermaid','v2')];frames=[]
for t in range(0,1440,60):
 im=Image.new('RGB',(768,5*320),(38,40,45));d=ImageDraw.Draw(im)
 for row,clip in enumerate(['move','water_hover','water_rise','water_dive','water_turn']):
  for col,p in enumerate(water):
   d.text((col*384+6,row*320+5),f'V{col+1} mermaid / {clip} / 2x nearest',fill='white')
   paste(im,clip_frame(p,clip,t),col*384,row*320+25)
 frames.append(im)
frames[0].save(out/'water-navigation.gif',save_all=True,append_images=frames[1:],duration=60,loop=0,disposal=2)
creatures=[('frog',load('frog','v2')),('mermaid',load('mermaid','v2')),('tree',load('tree','v2'))];frames=[];durations=[]
for stage,clip in [('LURE','v2_lure'),('REVEAL','v2_reveal'),('ATTACK','attack_primary')]:
 duration=max(sum(f['duration_ms'] for f in next(c for c in p[0]['animations'] if c['id']==clip)['frames']) for _,p in creatures)
 for t in range(0,duration,60):
  im=Image.new('RGB',(384*3,340),(38,40,45));d=ImageDraw.Draw(im)
  for col,(name,p) in enumerate(creatures):
   d.text((col*384+6,5),f'{name} / {stage} / 2x nearest',fill='white');paste(im,clip_frame(p,clip,t),col*384,25)
  frames.append(im);durations.append(min(60,duration-t))
frames[0].save(out/'friendly-ambush.gif',save_all=True,append_images=frames[1:],duration=durations,loop=0,disposal=2)
print(out/'water-navigation.gif');print(out/'friendly-ambush.gif')
