"""Review actual baked encounter frames, with no synthesized/repainted artwork.
Usage: python tools/encounter_evidence.py CATALOG OUTPUT
Build the catalog with cargo run --release --no-default-features --example encounter_catalog.
"""
import json
import sys
from pathlib import Path
from PIL import Image, ImageDraw
root, out = map(Path, sys.argv[1:3]); out.mkdir(parents=True, exist_ok=True)
names = ['slime', 'bush', 'banana', 'vase', 'sentinel', 'pusher']
background = (52, 59, 67)
def load(name, seed):
    path = root / f'{name}_{seed}'
    return json.loads((path / 'metadata.json').read_text()), Image.open(path / 'sprites.png').convert('RGBA')
packages = {(name, seed): load(name, seed) for name in names for seed in [0, 7, 42]}
def extract(name, seed, clip='idle', phase=0, direction=0, size=128):
    meta, atlas = packages[name, seed]
    c = next(c for c in meta['animations'] if c['id'] == clip)
    idx = c['frames'][phase % len(c['frames'])]['sprite_frame_id'] + direction * meta['sprites']['direction_stride']
    s = meta['sprites']; w, h = s['frame_width'], s['frame_height']; x, y = idx % s['columns'] * w, idx // s['columns'] * h
    image = atlas.crop((x, y, x+w, y+h))
    image.thumbnail((size, size), Image.Resampling.NEAREST)
    tile = Image.new('RGBA', (size, size)); tile.alpha_composite(image, ((size-image.width)//2, (size-image.height)//2))
    return tile
# Environmental forms are the actual lure clip. Idle is permanently revealed.
sheet=Image.new('RGB',(4*140,6*165),background);d=ImageDraw.Draw(sheet)
for row,name in enumerate(names):
    clip='v2_lure' if name in ['bush','banana','vase'] else 'idle'
    d.text((5,row*165+4),f'{name} | seed 7 | {clip} | 0 / 90 / 180 / 270 degrees',fill='white')
    for direction in range(4):
        image=extract(name,7,clip,direction=direction);sheet.paste(image,(direction*140,row*165+28),image)
sheet.save(out/'encounters-four-directions.png')
seeds=Image.new('RGB',(3*280,6*165),background);d=ImageDraw.Draw(seeds)
for row,name in enumerate(names):
    clip='v2_lure' if name in ['bush','banana','vase'] else 'idle'
    for col,seed in enumerate([0,7,42]):
        d.text((col*280+4,row*165+4),f'{name} seed{seed} | side + front',fill='white')
        for direction in [0,1]:
            image=extract(name,seed,clip,direction=direction);seeds.paste(image,(col*280+direction*136,row*165+28),image)
seeds.save(out/'encounters-three-seeds.png')
clips=[('slime','move'),('slime','v2_emerge'),('slime','v2_split_burst'),('bush','v2_reveal'),('banana','v2_reveal'),('vase','v2_reveal'),('sentinel','v2_shield_bash'),('sentinel','v2_shield_recovery'),('pusher','v2_push'),('pusher','v2_push_recovery')]
motion=Image.new('RGB',(6*140,len(clips)*158),background);d=ImageDraw.Draw(motion)
for row,(name,clip) in enumerate(clips):
    meta,_=packages[name,7];count=len(next(c for c in meta['animations'] if c['id']==clip)['frames'])
    d.text((4,row*158+3),f'{name}: {clip} | start to end | actor-only sprites',fill='white')
    for col in range(6):
        phase=round(col/5*(count-1));image=extract(name,7,clip,phase)
        motion.paste(image,(col*140,row*158+24),image)
motion.save(out/'encounters-keyframes.png')
for name in names:
    chosen=[c for n,c in clips if n==name]
    if name in ['bush','banana','vase']: chosen=['v2_lure','v2_reveal','idle','attack_primary']
    if name=='sentinel':chosen=['v2_guard','v2_shield_bash','v2_shield_recovery']
    frames=[]; durations=[]
    for clip in chosen:
        meta,_=packages[name,7];c=next(c for c in meta['animations'] if c['id']==clip)
        for phase,frame in enumerate(c['frames']):
            canvas=Image.new('RGB',(4*136,160),background);draw=ImageDraw.Draw(canvas)
            draw.text((5,4),f'{name}: {clip} | seed7 | all four directions',fill='white')
            for direction in range(4):
                image=extract(name,7,clip,phase,direction);canvas.paste(image,(direction*136,26),image)
            frames.append(canvas);durations.append(frame['duration_ms'])
    frames[0].save(out/f'{name}-motion.gif',save_all=True,append_images=frames[1:],duration=durations,loop=0,disposal=2)
# Inspect every actual baked frame for alpha touching the canvas edge.
clipped=[];count=0
for (name,seed),(meta,atlas) in packages.items():
    s=meta['sprites'];w,h=s['frame_width'],s['frame_height']
    for idx in range(s['frame_count']):
        x,y=idx%s['columns']*w,idx//s['columns']*h
        alpha=atlas.crop((x,y,x+w,y+h)).getchannel('A');bbox=alpha.getbbox();count+=1
        if bbox and (bbox[0]==0 or bbox[1]==0 or bbox[2]==w or bbox[3]==h):clipped.append([name,seed,idx])
(out/'render-audit.json').write_text(json.dumps({'packages':len(packages),'directional_frames_checked':count,'alpha_edge_clipping':clipped,'source':'actual generated sprites.png; no repainted or synthesized images'},indent=2)+'\n')
print(f'{len(packages)} packages, {count} frames, {len(clipped)} edge touches; evidence: {out}')
