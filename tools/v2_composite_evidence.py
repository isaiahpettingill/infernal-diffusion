"""Inspect actual hybrid/spectral/skeleton sprites, including alpha over a checker.
Usage: python tools/v2_composite_evidence.py CATALOG OUTPUT_DIR
"""
from functools import lru_cache
from pathlib import Path
import json
import sys
from PIL import Image, ImageDraw

root, output = Path(sys.argv[1]), Path(sys.argv[2])
output.mkdir(parents=True, exist_ok=True)

@lru_cache(maxsize=None)
def package(name):
    folder = root / name
    return json.loads((folder / "metadata.json").read_text()), Image.open(folder / "sprites.png").convert("RGBA")

def frame_at(name, clip_id, time_ms):
    monster, atlas = package(name)
    clip = next(c for c in monster["animations"] if c["id"] == clip_id)
    total = sum(f["duration_ms"] for f in clip["frames"])
    time_ms = time_ms % total if clip["looped"] else min(time_ms, total - 1)
    for frame in clip["frames"]:
        if time_ms < frame["duration_ms"]:
            break
        time_ms -= frame["duration_ms"]
    sprites = monster["sprites"]
    w, h, index = sprites["frame_width"], sprites["frame_height"], frame["sprite_frame_id"]
    x, y = index % sprites["columns"] * w, index // sprites["columns"] * h
    return atlas.crop((x, y, x + w, y + h))

def checker(width, height):
    image = Image.new("RGB", (width, height), (150, 156, 166))
    draw = ImageDraw.Draw(image)
    for y in range(0, height, 16):
        for x in range(0, width, 16):
            if (x // 16 + y // 16) % 2:
                draw.rectangle((x, y, x + 15, y + 15), fill=(192, 199, 209))
    return image

def paste(image, sprite, x, y, size):
    sprite = sprite.resize((size, size), Image.Resampling.NEAREST)
    image.paste(sprite, (x, y), sprite)

hybrids = [
    ("six_legged_centaur", "Six lower legs + two arms"),
    ("spider_centaur", "Humanoid upper / spider lower"),
    ("fish_head_humanoid", "Fish head / humanoid legs"),
    ("trooper_mermaid", "Trooper upper / mermaid lower"),
    ("walking_tree", "Tree upper / walking root legs"),
]
contact = Image.new("RGB", (5 * 300, 470), (38, 40, 45))
draw = ImageDraw.Draw(contact)
for column, (name, label) in enumerate(hybrids):
    draw.text((column * 300 + 8, 8), label, fill="white")
    draw.text((column * 300 + 8, 26), "Full frame fitted to128px +2x", fill="white")
    sprite = frame_at(name, "idle", 0)
    paste(contact, sprite, column * 300 + 86, 50, 128)
    paste(contact, sprite, column * 300 + 22, 200, 256)
contact.save(output / "v2-hybrid-compositions.png")

names = ["shade_humanoid", "shade_wolf", "shade_tree", "shade_mermaid"]
frames = []
for time_ms in range(0, 1450, 50):
    image = checker(4 * 300, 310)
    draw = ImageDraw.Draw(image)
    for column, name in enumerate(names):
        draw.text((column * 300 + 8, 8), f"{name} / tendril lash / {time_ms}ms", fill="black")
        paste(image, frame_at(name, "attack_v2_shade_tendril_lash", time_ms), column * 300 + 22, 45, 256)
    frames.append(image)
frames[0].save(output / "v2-shade-tendrils.gif", save_all=True, append_images=frames[1:], duration=50, loop=0, disposal=2)
contact = Image.new("RGB", (1200, 930))
for row, index in enumerate([0, 8, 13]):
    contact.paste(frames[index], (0, row * 310))
contact.save(output / "v2-shade-tendrils-contact.png")

frames = []
for time_ms in range(0, 1400, 50):
    image = checker(600, 310)
    draw = ImageDraw.Draw(image)
    for column, name in enumerate(["ghost", "ghoul"]):
        draw.text((column * 300 + 8, 8), f"{name} / phase-reappear / {time_ms}ms", fill="black")
        paste(image, frame_at(name, "attack_v2_spectral_phase_lunge", time_ms), column * 300 + 22, 45, 256)
    frames.append(image)
frames[0].save(output / "v2-spectral-phase.gif", save_all=True, append_images=frames[1:], duration=50, loop=0, disposal=2)
contact = Image.new("RGB", (900, 470), (38, 40, 45))
draw = ImageDraw.Draw(contact)
for column, seed in enumerate([0, 7, 42]):
    draw.text((column * 300 + 8, 8), f"Skeleton / seed{seed} /128px+2x", fill="white")
    sprite = frame_at(f"skeleton_{seed}", "idle", 0)
    paste(contact, sprite, column * 300 + 86, 50, 128)
    paste(contact, sprite, column * 300 + 22, 200, 256)
contact.save(output / "v2-skeleton-variants.png")
print(output)
