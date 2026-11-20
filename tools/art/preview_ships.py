#!/usr/bin/env python3
"""Make a contact sheet of the exported ship review renders (requires Pillow)."""

import json
import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[2]
SHIPS = ("scout", "patrol", "freighter", "prospector")


def triangle_count(name):
    data = (ROOT / "assets/models" / f"{name}.glb").read_bytes()
    json_size = struct.unpack_from("<I", data, 12)[0]
    gltf = json.loads(data[20:20 + json_size])
    return sum(
        gltf["accessors"][part["indices"]]["count"] // 3
        for mesh in gltf["meshes"] for part in mesh["primitives"]
    )


def main():
    sheet = Image.new("RGB", (1400, 1000), (13, 22, 30))
    draw = ImageDraw.Draw(sheet)
    font_path = "/System/Library/Fonts/Supplemental/Arial.ttf"
    heading = ImageFont.truetype(font_path, 28)
    small = ImageFont.truetype(font_path, 19)
    draw.text((44, 27), "SHIP HULLS / LOW POLY ART", font=heading,
              fill=(219, 241, 241))
    draw.text((1050, 34), "Individual framing; not battle scale", font=small,
              fill=(141, 173, 183))
    for index, name in enumerate(SHIPS):
        x = 44 + index % 2 * 680
        y = 83 + index // 2 * 455
        draw.rounded_rectangle((x, y, x + 650, y + 430), radius=14,
                               fill=(24, 38, 48), outline=(60, 101, 112), width=2)
        preview = Image.open(ROOT / "assets/source" / f"{name}-preview.png").convert("RGBA")
        preview.thumbnail((600, 355))
        sheet.paste(preview, (x + (650 - preview.width) // 2, y + 8), preview)
        draw.text((x + 24, y + 374), name.title(), font=heading,
                  fill=(231, 244, 244))
        draw.text((x + 626, y + 380), f"{triangle_count(name):,} triangles", font=small,
                  fill=(122, 214, 220), anchor="ra")
    output = ROOT / "docs/previews/ship-hulls.png"
    output.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(output)
    print(output)


if __name__ == "__main__":
    main()
