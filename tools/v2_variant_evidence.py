"""Render real baked V2 attacks and seed variants; never synthesize creature art.
Usage: python tools/v2_variant_evidence.py CATALOG OUTPUT_DIR
"""
from pathlib import Path
from functools import lru_cache
import json
import sys
from PIL import Image, ImageDraw

root, output = Path(sys.argv[1]), Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)

@lru_cache(maxsize=None)
def package(name):
    folder = root / name
    return json.loads((folder / "metadata.json").read_text()), Image.open(folder / "sprites.png").convert("RGBA")

def frame_at(package, clip_id, time_ms):
    monster, atlas = package
    clip = next(c for c in monster["animations"] if c["id"] == clip_id)
    total = sum(f["duration_ms"] for f in clip["frames"])
    time_ms = time_ms % total if clip["looped"] else min(time_ms, total - 1)
    frame = clip["frames"][-1]
    for frame in clip["frames"]:
        if time_ms < frame["duration_ms"]:
            break
        time_ms -= frame["duration_ms"]
    sprites = monster["sprites"]
    width, height, index = sprites["frame_width"], sprites["frame_height"], frame["sprite_frame_id"]
    x, y = index % sprites["columns"] * width, index // sprites["columns"] * height
    return atlas.crop((x, y, x + width, y + height))

def draw_frame(canvas, image, x, y, size=256):
    # Full frame, including transparent padding, scaled to an explicit gameplay
    # preview size; no cropping that could hide an out-of-bounds limb.
    image = image.resize((size, size), Image.Resampling.NEAREST)
    canvas.paste(image, (x, y), image)

variants = [
    ("mermaid_ordinary", "ordinary / seed 0"),
    ("mermaid_extra_tail", "extra tail / seed 10"),
    ("mermaid_four_arms", "four arms / seed 36"),
    ("mermaid_both", "both / seed 42"),
]
contact = Image.new("RGB", (1024, 500), (38, 40, 45))
draw = ImageDraw.Draw(contact)
for column, (name, label) in enumerate(variants):
    data = package(name)
    draw.text((column * 256 + 8, 8), label, fill="white")
    draw.text((column * 256 + 8, 26), "Full frame fitted to 128px", fill="white")
    draw_frame(contact, frame_at(data, "v2_lure", 240), column * 256 + 64, 48, 128)
    draw.text((column * 256 + 8, 190), "Attack / 2x gameplay preview", fill="white")
    draw_frame(contact, frame_at(data, "attack_v2_mermaid_lunge", 720), column * 256, 218)
contact.save(output / "mermaid-seeded-variants.png")

subjects = [
    ("mermaid_ordinary", "attack_v2_mermaid_lunge", "Mermaid full-body lunge"),
    ("mermaid_both", "attack_v2_mermaid_tail_whip", "Mutant mermaid tail whip"),
    ("tree_ordinary", "attack_v2_tree_bilateral_smash", "Tree bilateral growing smash"),
    ("tree_mutant", "attack_v2_tree_bilateral_smash", "Mutant tree / folded extra arms"),
]
frames = []
for time_ms in range(0, 1800, 60):
    image = Image.new("RGB", (1024, 310), (38, 40, 45))
    draw = ImageDraw.Draw(image)
    for column, (name, clip, label) in enumerate(subjects):
        draw.text((column * 256 + 6, 8), label, fill="white")
        draw.text((column * 256 + 6, 25), "Real baked frame / fitted to 256px", fill="white")
        draw_frame(image, frame_at(package(name), clip, time_ms), column * 256, 48)
    frames.append(image)
frames[0].save(output / "v2-specialty-attacks.gif", save_all=True, append_images=frames[1:], duration=60, loop=0, disposal=2)
# Contact sheet samples make impact and windup inspectable without GIF playback.
contact = Image.new("RGB", (1024, 310 * 3), (38, 40, 45))
for row, index in enumerate([2, 11, 19]):
    contact.paste(frames[index], (0, row * 310))
contact.save(output / "v2-specialty-attacks-contact.png")
print(output)
