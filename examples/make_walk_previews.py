"""Extract the default 3D MOVE clip into simple, enlarged inspection strips.

Run from the repository root: python examples/make_walk_previews.py
"""

from pathlib import Path

from PIL import Image


SHOWCASE = Path("examples/showcase")
NAMES = ("motion_wolf", "motion_orc", "motion_snake", "motion_slug", "motion_spider")
COLUMNS = 8
MOVE_FIRST_FRAME = 4  # The four IDLE poses precede MOVE in the baked atlas.
MOVE_FRAME_COUNT = 8
CELL = 256


for name in NAMES:
    directory = SHOWCASE / name
    sheet = Image.open(directory / "sprites.png").convert("RGBA")
    size = sheet.width // COLUMNS
    if sheet.width != size * COLUMNS or sheet.height < size * 2:
        raise ValueError(f"Unexpected sprite atlas layout: {directory}")
    strip = Image.new("RGBA", (MOVE_FRAME_COUNT * CELL, CELL), "#08090c")
    for index in range(MOVE_FRAME_COUNT):
        frame_id = MOVE_FIRST_FRAME + index
        source = sheet.crop(
            (
                (frame_id % COLUMNS) * size,
                (frame_id // COLUMNS) * size,
                (frame_id % COLUMNS + 1) * size,
                (frame_id // COLUMNS + 1) * size,
            )
        )
        enlarged = source.resize((size * 2, size * 2), Image.Resampling.NEAREST)
        strip.alpha_composite(
            enlarged,
            (index * CELL + (CELL - enlarged.width) // 2, (CELL - enlarged.height) // 2),
        )
    strip.convert("RGB").save(directory / "walk_preview.png")
    print(directory / "walk_preview.png")
