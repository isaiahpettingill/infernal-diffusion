# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow>=11,<13"]
# ///
"""Re-bake vendored CC0 props and produce labeled nearest-neighbor pixel evidence.
Run: uv run tools/prop_evidence.py [--output output/props]
All geometry is imported from Kenney; no generated substitute images.
"""
import argparse
import json
import pathlib
import subprocess
from PIL import Image, ImageDraw, ImageFont

ROOT = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--output", type=pathlib.Path, default=ROOT / "output" / "props")
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
source = ROOT / "examples" / "props"
manifest = json.loads((source / "PROVENANCE.json").read_text())
subprocess.run(["cargo", "build", "--release", "--no-default-features"], cwd=ROOT, check=True)
rows = []
checks = []
for item in manifest:
    destination = args.output / item["model"]
    subprocess.run([str(ROOT / "target" / "release" / "infernal"), "bake-prop", str(source / item["file"]), str(destination), str(source / "options.json")], check=True)
    metadata = json.loads((destination / "prop.json").read_text())
    atlas = Image.open(destination / "sprites.png").convert("RGBA")
    assert atlas.size == (metadata["atlas_width"], metadata["atlas_height"])
    frame_width, frame_height = metadata["frame_width"], metadata["frame_height"]
    for frame in metadata["frames"]:
        x,y,w,h = frame["atlas_rect"]
        crop = atlas.crop((x,y,x+w,y+h))
        bounds = crop.getchannel("A").getbbox()
        assert bounds and bounds[0] > 0 and bounds[1] > 0 and bounds[2] < w and bounds[3] < h, (item["model"], frame["angle_deg"], bounds)
        assert set(crop.getchannel("A").tobytes()) == {0,255}
        expected = frame["content_bounds"]
        assert bounds == (expected[0], expected[1], expected[0]+expected[2], expected[1]+expected[3])
    checks.append({"model": item["model"], "frame_size": [frame_width, frame_height], "views": len(metadata["frames"]), "pixels_per_unit": metadata["pixels_per_unit"], "pivot_world": metadata["pivot_world"], "transparent_background": True, "all_views_unclipped": True})
    # Each row preserves the exact same pixels/world-unit. Display magnification
    # is nearest-neighbor; checkerboard and crosshair belong to the preview only.
    panel = Image.new("RGB", (atlas.width, atlas.height + 40), "#20242b")
    draw = ImageDraw.Draw(panel)
    draw.text((8,5), item["model"] + " | Kenney / CC0 | 96 px per world unit", fill="#eeeeee")
    for y in range(40, panel.height, 8):
        for x in range(0, panel.width, 8):
            draw.rectangle((x,y,x+7,y+7),fill="#30353d" if ((x//8+y//8)%2) else "#363c46")
    panel.paste(atlas,(0,40),atlas)
    for frame in metadata["frames"]:
        x,y,w,h=frame["atlas_rect"]
        px,py=frame["pivot_pixels"]
        draw.text((x+7,y+23),str(frame["angle_deg"])+" deg",fill="#e9e9e9")
        cx,cy=x+px,y+40+py
        draw.line((cx-3,cy,cx+3,cy),fill="#ef8f65")
        draw.line((cx,cy-3,cx,cy+3),fill="#ef8f65")
    rows.append(panel)
width=max(r.width for r in rows);height=sum(r.height for r in rows)+54
contact=Image.new("RGB",(width,height),"#20242b")
draw=ImageDraw.Draw(contact)
draw.text((8,8),"Imported 3D props | actual CPU rasterizer output",fill="white")
draw.text((8,26),"Same world scale in every row. Orange + marks ground pivot. 2x nearest-neighbor preview.",fill="#b4becb")
y=54
for row in rows:contact.paste(row,(0,y));y+=row.height
contact.resize((width*2,height*2),Image.Resampling.NEAREST).save(args.output/"prop-preview.png")
(args.output/"checks.json").write_text(json.dumps(checks,indent=2)+"\n")
(args.output/"sources-and-licenses.json").write_text(json.dumps(manifest,indent=2)+"\n")
print("Verified five imported models; evidence:",args.output/"prop-preview.png")
