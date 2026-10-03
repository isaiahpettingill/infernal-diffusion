# /// script
# dependencies = ["cbor2", "pillow"]
# ///
"""Compare every clip in same-seed CBOR bakes without hard-coded frame offsets.

UV_CACHE_DIR=/tmp/uv uv run tools/animation_evidence.py BEFORE AFTER OUTPUT
Writes per-family full-state contact sheets, per-state before/after strips, GIFs,
and a manifest retaining prompts, seeds, timing, direction and coverage counts.
"""
import argparse
import json
from pathlib import Path
import cbor2
from PIL import Image, ImageDraw


def package(path):
    with (path / 'monster.cbor').open('rb') as f:
        metadata = cbor2.load(f)
    return metadata, Image.open(path / metadata['sprites']['image_file']).convert('RGBA')


def cell(data, frame, size=224, direction=0):
    m, sheet = data
    s = m['sprites']
    ident = frame['sprite_frame_id'] + direction * s['direction_stride']
    w, h, cols = s['frame_width'], s['frame_height'], s['columns']
    image = sheet.crop(((ident % cols)*w, (ident // cols)*h, (ident % cols+1)*w, (ident // cols+1)*h))
    # Same pixel scale on both sides even when the output canvas grew.
    image = image.resize((w*2, h*2), Image.Resampling.NEAREST)
    canvas = Image.new('RGBA', (size, size), '#14171d')
    canvas.alpha_composite(image, ((size-image.width)//2, round((size-image.height)*0.57)))
    return canvas


def borders(data):
    m,sheet=data;s=m['sprites'];w,h=s['frame_width'],s['frame_height'];result=[]
    lookup={f['sprite_frame_id']:c['id'] for c in m['animations'] for f in c['frames']}
    for frame_id in range(s['frame_count']):
        x=(frame_id%s['columns'])*w;y=(frame_id//s['columns'])*h
        edges=[(x+i,y) for i in range(w)]+[(x+i,y+h-1) for i in range(w)]+[(x,y+i) for i in range(1,h-1)]+[(x+w-1,y+i) for i in range(1,h-1)]
        pixels=sum(sheet.getpixel(p)[3]>0 for p in edges)
        if pixels:result.append({'frame':frame_id,'clip':lookup.get(frame_id%s['direction_stride']),'direction':frame_id//s['direction_stride'],'pixels':pixels})
    return result


def compare(before, after, output, families=None):
    output.mkdir(parents=True, exist_ok=True)
    manifest = []
    for family in sorted(x.name for x in after.iterdir() if (x/'monster.cbor').exists() and (not families or x.name in families)):
        b, a = package(before/family), package(after/family)
        assert b[0]['generation']['seed'] == a[0]['generation']['seed']
        assert b[0]['generation']['prompt'] == a[0]['generation']['prompt']
        dest = output/family
        dest.mkdir(exist_ok=True)
        rows=[]
        old={c['id']:c for c in b[0]['animations']}
        for clip in a[0]['animations']:
            ident=clip['id']
            sources=[(b,old.get(ident)),(a,clip)]
            size=max(192, 2*max(b[0]['sprites']['frame_width'],a[0]['sprites']['frame_width'], b[0]['sprites']['frame_height'],a[0]['sprites']['frame_height']))
            strip=Image.new('RGBA',(size*8, 2*(size+24)), '#0b0e13')
            draw=ImageDraw.Draw(strip)
            for row,(data,c) in enumerate(sources):
                draw.text((8,row*(size+24)+5),f'{family} / {ident} / {"BEFORE" if row==0 else "AFTER"}', fill='white')
                if not c: continue
                for col in range(8):
                    idx=round(col*(len(c['frames'])-1)/7)
                    strip.alpha_composite(cell(data,c['frames'][idx],size), (col*size,row*(size+24)+24))
            strip.convert('RGB').save(dest/f'{ident}.png')
            rows.append(strip)
            # Respect each bake's real frame durations and hold a completed one-shot.
            length=max(sum(f['duration_ms'] for f in c['frames']) for _,c in sources if c)
            gif=[]
            for ms in range(0,length+500,50):
                canvas=Image.new('RGBA',(2*size,size+24),'#14171d');d=ImageDraw.Draw(canvas)
                for col,(data,c) in enumerate(sources):
                    d.text((col*size+6,5), 'BEFORE' if col==0 else 'AFTER',fill='white')
                    if not c:continue
                    total=sum(f['duration_ms'] for f in c['frames']);clock=ms%total if c['looped'] else min(ms,total-1)
                    chosen=c['frames'][-1]
                    for frame in c['frames']:
                        if clock<frame['duration_ms']:chosen=frame;break
                        clock-=frame['duration_ms']
                    canvas.alpha_composite(cell(data,chosen,size), (col*size,24))
                gif.append(canvas.convert('RGB'))
            gif[0].save(dest/f'{ident}.gif',save_all=True,append_images=gif[1:],duration=50,loop=0)
        sheet=Image.new('RGBA',(size*8,len(rows)*(2*(size+24))),'#0b0e13')
        for i,row in enumerate(rows):sheet.alpha_composite(row,(0,i*(2*(size+24))))
        sheet.convert('RGB').save(output/f'{family}_all_states.png')
        manifest.append({'family':family,'prompt':a[0]['generation']['prompt'],'seed':a[0]['generation']['seed'],
            'clips':[{'id':c['id'],'state':c['semantic_state'],'frames':len(c['frames']),'duration_ms':sum(f['duration_ms'] for f in c['frames'])} for c in a[0]['animations']],
            'before_frames':b[0]['sprites']['frame_count'],'after_frames':a[0]['sprites']['frame_count'], 'border_pixels': {'before': borders(b), 'after': borders(a)}})
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    print(f'{len(manifest)} same-seed families, {sum(len(m["clips"]) for m in manifest)} clips compared in {output}')

if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('before',type=Path);p.add_argument('after',type=Path);p.add_argument('output',type=Path);p.add_argument('--families',nargs='*')
    args=p.parse_args();compare(args.before,args.after,args.output,args.families)
