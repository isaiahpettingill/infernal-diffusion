"""Assemble real baked sprites, never synthetic redraws, into game-sized QA sheets.
Usage: python tools/make_species_previews.py CATALOG OUTPUT
"""
import json
import sys
from pathlib import Path
from PIL import Image, ImageDraw
root, out = map(Path, sys.argv[1:3]); out.mkdir(parents=True, exist_ok=True)
groups = {
    'humanoids': ['orc','troll','cyclops','minotaur','anime','grendel','historical','reaper'],
    'animals': ['wolf','lion','tiger','bear','elephant','mammoth','rhino','horse','goat','boar','rabbit','eagle'],
    'fantasy': ['crab','lobster','hydra','cerberus','snake','gorgon','worm','sandworm','dragon','trex','stego','triceratops'],
    'modifiers': ['potato','round','cacodaemon','devil','alien','robot','cute_elephant','chibi_wolf','zombie_wolf','ninja','fire_orc','water_bear'],
}
def frame(path, key=0):
    meta=json.loads((path/'metadata.json').read_text());s=meta['sprites'];a=Image.open(path/'sprites.png').convert('RGBA');w,h=s['frame_width'],s['frame_height'];x=key%s['columns']*w;y=key//s['columns']*h
    im=a.crop((x,y,x+w,y+h));im.thumbnail((128,128),Image.Resampling.NEAREST)
    tile=Image.new('RGBA',(128,128));tile.alpha_composite(im,((128-im.width)//2,(128-im.height)//2))
    return tile,meta
for group,names in groups.items():
    sheet=Image.new('RGB',(1152,((len(names)+2)//3)*300),(77,84,92));d=ImageDraw.Draw(sheet)
    for i,name in enumerate(names):
        path=root/(name+'_0')
        if not path.exists():continue
        im,m=frame(path);front,_=frame(path,m['sprites']['direction_stride']);x=i%3*384;y=i//3*300
        d.text((x+6,y+6),f'{name} | seed0 | 128px game canvas +2x',fill='white')
        sheet.paste(im,(x,y+32),im);sheet.paste(front,(x,y+160),front);big=im.resize((256,256),Image.Resampling.NEAREST);sheet.paste(big,(x+128,y+25),big)
    sheet.save(out/(group+'.png'))
