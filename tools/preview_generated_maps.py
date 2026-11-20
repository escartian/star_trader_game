#!/usr/bin/env python3
"""Reproducible schematic previews from an authoritative generated save.

Requires Pillow. Run after building the client:
    python3 tools/preview_generated_maps.py
The images are data previews, not GPU captures of the game's UI.
"""

import json
import math
import subprocess
from collections import Counter
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont


ROOT = Path(__file__).resolve().parents[1]
PREVIEWS = ROOT / "docs" / "previews"
WIDTH, HEIGHT = 1600, 900
BG = (13, 22, 30)
CYAN = (73, 224, 232)
MUTED = (144, 173, 187)
STAR_COLORS = {
    "YellowDwarf": (255, 216, 124),
    "RedDwarf": (238, 113, 91),
    "BlueGiant": (118, 177, 255),
    "RedGiant": (255, 143, 98),
    "WhiteDwarf": (215, 233, 255),
    "OrangeDwarf": (255, 175, 77),
    "BlueWhiteStar": (176, 216, 255),
    "BrownDwarf": (154, 82, 66),
    "NeutronStar": (224, 250, 255),
}
BODY_COLORS = {
    "RockyPlanet": (118, 190, 148),
    "GasGiant": (149, 178, 237),
    "Moon": (190, 206, 219),
    "Asteroid": (189, 158, 126),
    "Station": CYAN,
}
BIOME_COLORS = {
    "temperate": (118, 190, 148), "arid": (211, 143, 83),
    "frozen": (179, 211, 232), "ocean": (38, 151, 198),
    "jungle": (108, 182, 67), "volcanic": (238, 90, 45),
    "barren": (167, 156, 150),
}
GAS_COLORS = {
    "moderate": (207, 169, 126), "insidious": (163, 122, 213),
    "dangerous": (89, 199, 204),
}


def body_color(body):
    profile = body.get("planet")
    if body["kind"] == "RockyPlanet" and profile:
        return BIOME_COLORS[profile["biome"]]
    if body["kind"] == "GasGiant" and profile:
        return GAS_COLORS.get(profile["danger"], BODY_COLORS["GasGiant"])
    return BODY_COLORS[body["kind"]]


def font(size):
    for name in ("DejaVuSans.ttf", "/System/Library/Fonts/Supplemental/Arial.ttf"):
        try:
            return ImageFont.truetype(name, size)
        except OSError:
            pass
    return ImageFont.load_default()


def world():
    command = [
        str(ROOT / "target" / "debug" / "game-client"),
        "--new", "--headless", "--seed", "42", "--systems", "150",
        "--radius", "95", "--objects", "15",
    ]
    return json.loads(subprocess.check_output(command, cwd=ROOT))


def canvas(title, subtitle):
    image = Image.new("RGB", (WIDTH, HEIGHT), BG)
    draw = ImageDraw.Draw(image)
    for y in range(HEIGHT):
        shade = int(10 * y / HEIGHT)
        draw.line((0, y, WIDTH, y), fill=(13 + shade, 22 + shade, 30 + shade))
    draw.text((55, 35), title, font=font(34), fill=(224, 245, 246))
    draw.text((57, 83), subtitle, font=font(18), fill=MUTED)
    draw.rounded_rectangle((1100, 125, 1548, 840), radius=18, fill=(22, 38, 48), outline=(63, 101, 111), width=2)
    draw.rounded_rectangle((1310, 43, 1548, 79), radius=7, fill=(37, 74, 83))
    draw.text((1326, 51), "DATA SCHEMATIC", font=font(16), fill=CYAN)
    return image, draw


def section(draw, y, title):
    draw.text((1130, y), title, font=font(20), fill=CYAN)
    draw.line((1130, y + 31, 1518, y + 31), fill=(62, 93, 105), width=1)


def value(draw, y, label, number):
    draw.text((1130, y), label, font=font(18), fill=MUTED)
    draw.text((1450, y), str(number), font=font(18), fill=(226, 240, 241), anchor="ra")


def galaxy_preview(data):
    systems = data["systems"]
    image, draw = canvas("THE FRONTIER / 150 SYSTEMS", "Seed 42 · 95 ly radius · up to 15 objects per system")
    cx, cy, scale = 565, 492, 3.7
    radius = 95 * scale
    draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), outline=(53, 80, 91), width=2)
    for r in (20, 50, 80):
        rr = r * scale
        draw.ellipse((cx - rr, cy - rr, cx + rr, cy + rr), outline=(38, 61, 70), width=1)
    points = [(cx + s["position"]["x"] * scale, cy - s["position"]["z"] * scale) for s in systems]
    for i in range(1, len(systems)):
        near = min(range(i), key=lambda j: (points[i][0] - points[j][0]) ** 2 + (points[i][1] - points[j][1]) ** 2)
        draw.line((*points[i], *points[near]), fill=(35, 69, 75), width=1)
    glow = Image.new("RGBA", image.size)
    gd = ImageDraw.Draw(glow)
    for system, (x, y) in zip(systems, points):
        color = STAR_COLORS[system["star_kind"]]
        gd.ellipse((x - 10, y - 10, x + 10, y + 10), fill=(*color, 95))
    image = Image.alpha_composite(image.convert("RGBA"), glow.filter(ImageFilter.GaussianBlur(9)))
    draw = ImageDraw.Draw(image)
    for system, (x, y) in zip(systems, points):
        color = STAR_COLORS[system["star_kind"]]
        r = 7 if system["id"] == 1 else 3
        draw.ellipse((x - r, y - r, x + r, y + r), fill=color)
    for index in (0, 1, 2):
        x, y = points[index]
        draw.text((x + 13, y - 17), systems[index]["name"], font=font(17), fill=(225, 240, 241))
    section(draw, 155, "GALAXY")
    value(draw, 205, "Star systems", len(systems))
    value(draw, 240, "Orbiting objects", sum(len(s["bodies"]) for s in systems))
    value(draw, 275, "Market ports", len(systems))
    section(draw, 330, "STAR CLASSES")
    for index, (kind, count) in enumerate(Counter(s["star_kind"] for s in systems).most_common()):
        y = 380 + index * 29
        draw.ellipse((1130, y + 5, 1143, y + 18), fill=STAR_COLORS[kind])
        label = kind.replace("BlueWhite", "Blue-white ").replace("Neutron", "Neutron ")
        label = label.replace("Dwarf", " dwarf").replace("Giant", " giant")
        draw.text((1158, y), label, font=font(18), fill=(218, 232, 236))
        draw.text((1490, y), str(count), font=font(18), fill=MUTED, anchor="ra")
    section(draw, 665, "TRAVEL")
    draw.multiline_text((1130, 715), "Every star has a route back to Sol\nthrough hops of 20 ly or less.", font=font(18), fill=MUTED, spacing=9)
    PREVIEWS.mkdir(parents=True, exist_ok=True)
    image.convert("RGB").save(PREVIEWS / "generated-galaxy-150.png")


def system_preview(data):
    system = next(s for s in data["systems"][1:] if len(s["bodies"]) == 14)
    image, draw = canvas(f"{system['name'].upper()} / 15 OBJECTS", f"Central star + 14 orbiting objects · seed 42")
    cx, cy, scale = 565, 490, 1.78 / 1_000_000
    for body in system["bodies"]:
        orbit = body.get("orbit")
        if orbit and not orbit.get("parent"):
            epoch = orbit["epoch"]
            r = math.hypot(epoch["x_km"], epoch["z_km"]) * scale
            draw.ellipse((cx - r, cy - r, cx + r, cy + r), outline=(48, 73, 81), width=1)
    draw.ellipse((cx - 13, cy - 13, cx + 13, cy + 13), fill=STAR_COLORS[system["star_kind"]])
    draw.text((cx + 21, cy - 14), system["name"], font=font(18), fill=(228, 240, 241))
    for body in system["bodies"]:
        pos = body["position"]
        x, y = cx + pos["x_km"] * scale, cy - pos["z_km"] * scale
        kind = body["kind"]
        r = {"RockyPlanet": 9, "GasGiant": 14, "Moon": 5, "Asteroid": 5, "Station": 7}[kind]
        color = body_color(body)
        draw.ellipse((x - r, y - r, x + r, y + r), fill=color, outline=(218, 238, 240), width=1)
        draw.text((x + r + 5, y - 11), str(body["id"]), font=font(15), fill=(226, 240, 241))
    section(draw, 153, "ORBITING OBJECTS")
    for i, body in enumerate(system["bodies"]):
        col = i // 8
        row = i % 8
        x = 1130 + col * 202
        y = 205 + row * 54
        draw.ellipse((x, y + 5, x + 13, y + 18), fill=body_color(body))
        label = body["planet"]["biome"].title() if body.get("planet") and body["kind"] == "RockyPlanet" else body["kind"].replace("Planet", "")
        draw.text((x + 20, y), f"{body['id']:02}  {label[:13]}", font=font(16), fill=(220, 235, 238))
    section(draw, 680, "SYSTEM RULES")
    draw.multiline_text((1130, 729), "Orbits remain clear at every tick.\nPlanet markets and the port use\nnormal trade rules.", font=font(17), fill=MUTED, spacing=6)
    image.save(PREVIEWS / "generated-system-15.png")


if __name__ == "__main__":
    data = world()
    galaxy_preview(data)
    system_preview(data)
    print("Wrote docs/previews/generated-galaxy-150.png and generated-system-15.png")
