"""Actual baked dinosaur/count-modifier frames at 128px and 96px, plus motion.
Usage: python tools/dinosaur_evidence.py BEFORE AFTER OUTPUT
No synthesis, repainting, or silhouette substitution is used.
"""
import json
import sys
from pathlib import Path
from PIL import Image, ImageDraw
before, after, out = map(Path, sys.argv[1:4])
out.mkdir(parents=True, exist_ok=True)
groups = {
    'dinosaurs': ['dinosaur', 'trex', 'raptor', 'longneck', 'stego', 'triceratops', 'ankylosaur'],
    'explicit-modifiers': ['three_heads', 'eight_arms', 'combined', 'skeleton_mermaid', 'numeric'],
}
def load(root, name, seed=0):
    p = root / f'{name}_{seed}'
    return json.loads((p / 'metadata.json').read_text()), Image.open(p / 'sprites.png').convert('RGBA')
def frame(package, clip='idle', phase=0, direction=0, size=128):
    m, a = package
    c = next(c for c in m['animations'] if c['id'] == clip)
    i = c['frames'][phase % len(c['frames'])]['sprite_frame_id'] + direction * m['sprites']['direction_stride']
    s = m['sprites']; w, h = s['frame_width'], s['frame_height']
    im = a.crop((i % s['columns'] * w, i // s['columns'] * h, i % s['columns'] * w+w, i // s['columns'] * h+h))
    im.thumbnail((size, size), Image.Resampling.NEAREST)
    tile = Image.new('RGBA', (size, size)); tile.alpha_composite(im, ((size-im.width)//2, (size-im.height)//2))
    return tile
for group, names in groups.items():
    available = [n for n in names if (before / f'{n}_0/metadata.json').exists() and (after / f'{n}_0/metadata.json').exists()]
    sheet = Image.new('RGB', (1100, 164*len(available)), (58, 63, 70)); draw = ImageDraw.Draw(sheet)
    for row, name in enumerate(available):
        y = row*164
        for col, root in enumerate([before, after]):
            p = load(root, name); x = col*550
            draw.text((x+5, y+3), f'{name}: {"BEFORE" if col==0 else "AFTER"} / side, front, back (128px), side (96px)', fill='white')
            for d in range(3):
                im = frame(p, direction=d); sheet.paste(im, (x+d*130, y+27), im)
            im = frame(p, size=96); sheet.paste(im, (x+400, y+43), im)
    sheet.save(out / f'{group}-comparison.png')
    seeds = Image.new('RGB', (3*264, 164*len(available)), (58, 63, 70))
    seed_draw = ImageDraw.Draw(seeds)
    for row, name in enumerate(available):
        for col, seed in enumerate([0, 7, 42]):
            if not (after / f'{name}_{seed}/metadata.json').exists(): continue
            package = load(after, name, seed)
            seed_draw.text((col*264+4, row*164+3), f'{name} seed {seed} / 128px side + front', fill='white')
            for direction in [0, 1]:
                im = frame(package, direction=direction)
                seeds.paste(im, (col*264+direction*130, row*164+27), im)
    seeds.save(out / f'{group}-three-seeds.png')
    # Inspect animation phases at game size, rather than relying on a single idle silhouette.
    frames = []
    packages = [(name, load(after, name)) for name in available]
    for phase in range(24):
        canvas = Image.new('RGB', (len(packages)*144, 3*156), (58, 63, 70)); d = ImageDraw.Draw(canvas)
        for col, (name, p) in enumerate(packages):
            attack = p[0]['attacks'][0]['animation_id'] if p[0]['attacks'] else 'idle'
            for row, clip in enumerate(['idle', 'move', attack]):
                if not any(c['id'] == clip for c in p[0]['animations']): clip = 'idle'
                d.text((col*144+3, row*156+3), f'{name}\n{clip}', fill='white')
                im = frame(p, clip, phase); canvas.paste(im, (col*144, row*156+28), im)
        frames.append(canvas)
    if frames:
        for phase in [0, 5, 11, 17]: frames[phase].save(out / f'{group}-motion-{phase:02}.png')
        frames[0].save(out / f'{group}-motion.gif', save_all=True, append_images=frames[1:], duration=90, loop=0, disposal=2)
print(out)
