"""Compose actual profile_catalog sprite pixels; no generated substitute artwork.

Usage: python tools/profile_evidence.py CATALOG OUTPUT.png
Native frames retain scale; enlarged details use nearest-neighbor pixels.
"""
import json
import sys
from pathlib import Path
from PIL import Image, ImageDraw

root, output = Path(sys.argv[1]), Path(sys.argv[2])
names = ['wolf', 'beetle', 'dragon', 'frog', 'mermaid', 'tree']
canvas = Image.new('RGB', (1000, 360 * len(names)), (38, 40, 45))
draw = ImageDraw.Draw(canvas)
for row, name in enumerate(names):
    for col, version in enumerate(['v1', 'v2']):
        directory = root / name / version
        monster = json.loads((directory / 'metadata.json').read_text())
        sprite = monster['sprites']
        clip_id = 'v2_lure' if version == 'v2' and name in ['frog', 'mermaid', 'tree'] else 'idle'
        clip = next(clip for clip in monster['animations'] if clip['id'] == clip_id)
        frame = clip['frames'][min(3, len(clip['frames']) - 1)]['sprite_frame_id']
        atlas = Image.open(directory / 'sprites.png').convert('RGBA')
        width, height = sprite['frame_width'], sprite['frame_height']
        x, y = frame % sprite['columns'] * width, frame // sprite['columns'] * height
        image = atlas.crop((x, y, x + width, y + height))
        bounds = image.getbbox()
        if bounds is None:
            raise ValueError(f'empty sprite: {name} {version}')
        detail = image.crop(bounds)
        zoom = detail.resize((detail.width * 3, detail.height * 3), Image.Resampling.NEAREST)
        if zoom.width > 300 or zoom.height > 325:
            raise ValueError(f'increase evidence cell, never silently crop: {name} {version}')
        canvas.paste(image, (col * 500 + 3, row * 360 + 28), image)
        canvas.paste(zoom, (col * 500 + 180, row * 360 + 28), zoom)
        draw.text((col * 500 + 5, row * 360 + 4), f'{name} {version}: native / 3x nearest ({clip_id})', fill='white')
output.parent.mkdir(parents=True, exist_ok=True)
canvas.save(output)
print(output)
