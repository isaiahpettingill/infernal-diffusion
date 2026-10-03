# /// script
# dependencies = ["pillow"]
# ///
"""Label deterministic Blender renders and record their anatomy for review.
Usage: uv run tools/geometry_evidence.py BASELINE AFTER OUTPUT
"""
import json, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
before, after, out = map(Path,sys.argv[1:4]); out.mkdir(parents=True,exist_ok=True)
font_path='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
font=ImageFont.truetype(font_path,20);small=ImageFont.truetype(font_path,15)
families=['spider','beetle','hornet','scorpion','centipede']
cell=420; header=110;row=450
sheet=Image.new('RGB',(cell*2,header+len(families)*row),(18,23,29));d=ImageDraw.Draw(sheet)
d.text((22,18),'Infernal Diffusion · intermediate anatomy',font=font,fill='white')
d.text((22,47),'Identical anatomy seed 42 · exact baker triangles · fixed Blender camera',font=small,fill=(175,188,200))
d.text((22,79),'BEFORE',font=font,fill=(255,184,138));d.text((cell+22,79),'AFTER',font=font,fill=(151,227,191))
records=[]
for i,name in enumerate(families):
    record={'family':name,'anatomy_seed':42}
    for side,root in enumerate((before,after)):
        image=Image.open(root/f'{name}.png').convert('RGB').resize((cell,cell),Image.Resampling.LANCZOS)
        y=header+i*row;sheet.paste(image,(side*cell,y))
        data=json.loads((root/name/'idle_00.json').read_text());nodes=data['nodes']
        legs=sum(n['kind']=='FOOT' for n in nodes)
        d.text((side*cell+16,y+cell+8),f'{name.title()} · {legs} walking legs',font=small,fill='white')
        record['before' if side==0 else 'after']={
            'body_plan':data['body_plan'],'walking_legs':legs,
            'wing_nodes':sum(n['kind']=='WING' for n in nodes),
            'antenna_nodes':sum(n['kind']=='ANTENNA' for n in nodes),
            'abdomen_nodes':sum(n['kind']=='ABDOMEN' for n in nodes),
            'body_segment_nodes':sum(n['kind']=='BODY_SEGMENT' for n in nodes),
            'triangle_count':len(data['triangles'])}
    records.append(record)
sheet.save(out/'Infernal_anatomy_before_after.png')
(out/'anatomy_audit.json').write_text(json.dumps({'comparison':'Direct anatomy seed; identical fixed camera and lighting; exact renderer triangles','specimens':records},indent=2)+'\n')
