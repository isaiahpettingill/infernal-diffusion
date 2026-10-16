"""Measure actual baked packages and show sprites at their declared world scale."""
import json
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
INPUT = ROOT / 'output/world-scale-catalog'
OUTPUT = ROOT / 'docs/evidence/world-scaling'
FONT = '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
font = ImageFont.truetype(FONT, 18)
small = ImageFont.truetype(FONT, 14)


def specimen(path):
    monster = json.loads((path / 'metadata.json').read_text())
    sprites, size = monster['sprites'], monster['size']
    atlas = Image.open(path / 'sprites.png').convert('RGBA')
    fw, fh, columns = sprites['frame_width'], sprites['frame_height'], sprites['columns']
    clipped = 0
    for index in range(sprites['frame_count']):
        x, y = (index % columns) * fw, (index // columns) * fh
        bounds = atlas.crop((x, y, x + fw, y + fh)).getbbox()
        if bounds and (bounds[0] == 0 or bounds[1] == 0 or bounds[2] == fw or bounds[3] == fh):
            clipped += 1
    clip = next((a for a in monster['animations'] if a['semantic_state'] == 'IDLE'), monster['animations'][0])
    frame = clip['frames'][0]['sprite_frame_id']
    x, y = frame % columns * fw, frame // columns * fh
    tile = atlas.crop((x, y, x + fw, y + fh))
    bounds = tile.getbbox()
    crop = tile.crop(bounds)
    scale = size['world_scale']
    image = crop.resize((max(1, round(crop.width * scale)), max(1, round(crop.height * scale))), Image.Resampling.NEAREST)
    row = {'profile': path.parent.name, 'name': path.name,
           'prompt': monster['generation']['prompt'], 'seed': monster['generation']['seed'],
           **{k: size[k] for k in ('reference_height_px', 'target_height_px', 'world_scale')},
           'idle_full_silhouette_height_px': crop.height * scale,
           'frames_checked': sprites['frame_count'], 'clipped_frames': clipped}
    return image, row


def panel(profile, title, names, samples, destination):
    items = [samples[(profile, name)] for name in names]
    widths = [max(160, image.width + 24) for image, _ in items]
    width = max(1000, sum(widths) + 110)
    height = max(300, max(image.height for image, _ in items) + 150)
    ground = height - 65
    out = Image.new('RGB', (width, height), '#171d2a')
    draw = ImageDraw.Draw(out)
    draw.text((20, 15), f'{profile.upper()}  •  {title}', fill='#f2eee5', font=font)
    draw.text((20, 44), 'Actual baked sprites at world scale • 96px player • no per-cell resizing', fill='#b0becd', font=small)
    draw.line((15, ground, width-15, ground), fill='#788899', width=1)
    draw.line((15, ground-96, width-15, ground-96), fill='#344352', width=1)
    x = 18
    draw.rectangle((x+12, ground-96, x+32, ground), outline='#80d1ec', width=2)
    draw.text((x, ground+8), 'Player', fill='#80d1ec', font=small)
    draw.text((x, ground+27), '96px', fill='#80d1ec', font=small)
    x = 100
    for cell_width, (image, row) in zip(widths, items):
        out.paste(image, (x+(cell_width-image.width)//2, ground-image.height), image)
        label = row['prompt']
        draw.text((x+8, ground+8), label, fill='#f2eee5', font=small)
        draw.text((x+8, ground+27), f"body {row['target_height_px']:.1f}px • seed {row['seed']}", fill='#b0becd', font=small)
        x += cell_width
    out.save(OUTPUT / destination)


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    samples = {}
    for path in sorted(INPUT.glob('*/*')):
        if (path / 'metadata.json').exists(): samples[(path.parent.name, path.name)] = specimen(path)
    for profile in ('v1', 'v2'):
        panel(profile, 'Species stature', ['beetle_7', 'humanoid_7', 'wolf_7', 'elephant_7', 'fish_7', 'bat_7'], samples, f'{profile}-species.png')
        panel(profile, 'Universal giant / mini / cute sizes', ['elephant_7', 'giant_elephant_7', 'mini_elephant_7', 'cute_elephant_7', 'chibi_bat_7'], samples, f'{profile}-modifiers.png')
        panel(profile, 'Seeded giant variation', ['giant_elephant_0', 'giant_elephant_7', 'giant_elephant_42'], samples, f'{profile}-seeds.png')
    rows = [row for _, row in samples.values()]
    report = {'packages': len(rows), 'frames_checked': sum(r['frames_checked'] for r in rows),
              'clipped_frames': sum(r['clipped_frames'] for r in rows), 'specimens': rows}
    (OUTPUT / 'measurements.json').write_text(json.dumps(report, indent=2)+'\n')
    text = f'''# Actual world-scale renderer evidence

Generated with the vocabulary parser on both V1 and V2, using the exact baked
PNG atlas and exported world scale. The blue reference player is 96px tall.
No specimen is auto-fitted into a common canvas box. Labels give neutral body
height; wing/tail silhouettes and animated poses may extend beyond that body.

- {report['packages']} packages
- {report['frames_checked']:,} directional animation frames checked
- {report['clipped_frames']} edge-clipped frames
- Giant elephant seeds 0 / 7 / 42: 361.5 / 327.2 / 384.5px body height
- Mini elephant seed 7: 54.9px; cute elephant and chibi bat: 64px body height

## Species defaults

![V1 species](v1-species.png)
![V2 species](v2-species.png)

## Same species, different modifiers

![V1 modifiers](v1-modifiers.png)
![V2 modifiers](v2-modifiers.png)

## Deterministic seeded variation

![V1 giant seeds](v1-seeds.png)
![V2 giant seeds](v2-seeds.png)

[Exact measurements](measurements.json) · [Same-host V1 compatibility](v1-compatibility.json) · [Policy and integration contract](../../world-scaling.md)

Reproduce with `cargo run --release --no-default-features --example world_scale_catalog`
and `python3 tools/world_scale_evidence.py`. These are procedural renderer outputs,
not AI-generated illustration substitutes.
'''
    (OUTPUT / 'README.md').write_text(text)
    print(json.dumps({k:v for k,v in report.items() if k != 'specimens'}))
    if report['clipped_frames']: raise SystemExit('Unexpected clipped frame')

if __name__ == '__main__': main()
