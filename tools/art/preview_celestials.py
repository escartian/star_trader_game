#!/usr/bin/env python3
"""Build a contact sheet from the runtime celestial sprites (requires Pillow)."""

from pathlib import Path
from PIL import Image, ImageDraw, ImageFont


ROOT = Path(__file__).resolve().parents[2]
GROUPS = [
    ("STARS", ["yellow-dwarf", "orange-dwarf", "red-dwarf", "brown-dwarf",
               "blue-white-star", "blue-giant", "red-giant", "white-dwarf", "neutron-star"]),
    ("ROCKY PLANETS", ["rocky-planet", "arid-planet", "frozen-planet", "ocean-planet",
                        "jungle-planet", "volcanic-planet", "barren-planet"]),
    ("GAS GIANTS AND SMALL BODIES", ["gas-giant", "storm-giant", "methane-giant",
                                    "moon", "ice-moon", "asteroid", "metallic-asteroid"]),
]


def main():
    image = Image.new("RGB", (1200, 1280), (13, 22, 30))
    draw = ImageDraw.Draw(image)
    font_path = "/System/Library/Fonts/Supplemental/Arial.ttf"
    font = ImageFont.truetype(font_path, 17)
    heading = ImageFont.truetype(font_path, 26)
    y = 28
    for title, names in GROUPS:
        draw.text((34, y), title, font=heading, fill=(119, 220, 226))
        y += 48
        for index, name in enumerate(names):
            column, row = index % 5, index // 5
            x, tile_y = 34 + column * 230, y + row * 186
            draw.rounded_rectangle((x, tile_y, x + 214, tile_y + 172), radius=12,
                                   fill=(25, 38, 49), outline=(55, 86, 97), width=2)
            sprite = Image.open(ROOT / "assets" / "sprites" / f"{name}.png").convert("RGBA")
            sprite.thumbnail((134, 134))
            image.paste(sprite, (x + (214 - sprite.width) // 2, tile_y + 5), sprite)
            draw.text((x + 107, tile_y + 145), name.replace("-", " "), font=font,
                      fill=(223, 237, 238), anchor="mt")
        y += ((len(names) + 4) // 5) * 186 + 18
    output = ROOT / "docs" / "previews" / "celestial-variants.png"
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output)
    print(output)


if __name__ == "__main__":
    main()
