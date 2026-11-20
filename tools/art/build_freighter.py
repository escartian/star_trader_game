"""Build the freighter, an original bulk container hauler, and export its assets.

Writes assets/source/freighter.blend, assets/models/freighter.glb, the 256px
map sprite assets/sprites/freighter.png and a review render
(assets/source/freighter-preview.png). Shared conventions and the triangle
budget live in hull_kit.py. The freighter keeps its identity from the first
prototype: blue-grey titanium, orange cargo, a teal canopy, blue engines.

Scale is sold the prospector's way: a long yard of small containers stacked
three high, gantries between the rows, and a deliberately tiny bridge with lit
windows, so the hull reads as huge.

Run from the repository root:

    blender --background --factory-startup --python tools/art/build_freighter.py

or through the Blender MCP by exec-ing this file with ``__file__`` set and
calling ``build()``.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hull_kit import (  # noqa: E402
    QUARTER, box, clear_scene, cone, cylinder, loft, material, publish, stage,
)

ROW_COUNT = 8
ROW_PITCH = 0.64  # Container length plus the gantry gap (y).
YARD_BOW = 2.3  # y of the first row's centre; rows run sternward from here.
COLUMNS = (0.44, 0.88)  # Container column centres either side of the spine (|x|).
TIERS = (-0.18, 0.18, 0.54)  # Container layer centres (z).
CONTAINER = (0.42, 0.58, 0.34)


def palette():
    """Linear colours."""
    return {
        "hull": material("Titanium hull", (0.32, 0.43, 0.53), 0.6, 0.4),
        "frame": material("Gunmetal frame", (0.11, 0.12, 0.14), 0.6, 0.45),
        "steel": material("Bare steel", (0.48, 0.50, 0.52), 0.85, 0.35),
        "stripe": material("Hazard black", (0.03, 0.03, 0.035), 0.2, 0.6),
        "hazard": material("Hazard yellow", (0.85, 0.52, 0.05), 0.3, 0.5),
        "glass": material("Canopy", (0.025, 0.16, 0.21), 0.8, 0.15),
        "engine": material("Engine blue", (0.08, 0.6, 1.0), emission=3.0),
        "lamp": material("Window light", (1.0, 0.94, 0.78), emission=2.0),
        "port": material("Port light", (1.0, 0.08, 0.05), emission=3.0),
        "starboard": material("Starboard light", (0.1, 1.0, 0.25), emission=3.0),
        "cargo": [
            material("Cargo orange", (0.88, 0.32, 0.06), 0.35, 0.5),
            material("Cargo ivory", (0.70, 0.68, 0.60), 0.2, 0.55),
            material("Cargo teal", (0.05, 0.32, 0.36), 0.3, 0.5),
            material("Cargo rust", (0.45, 0.09, 0.04), 0.3, 0.6),
            material("Cargo slate", (0.20, 0.24, 0.30), 0.4, 0.5),
        ],
        "rib": material("Container rib", (0.05, 0.05, 0.06), 0.4, 0.5),
        "seal": material("Cargo latches", (0.18, 0.21, 0.23), 0.5, 0.48),
    }


def cargo_colour(row, column, tier):
    """Deterministic mostly-orange livery; neighbouring boxes rarely match."""
    pick = (row * 7 + column * 3 + tier * 5) % 11
    return (0, 0, 0, 0, 1, 1, 2, 3, 0, 4, 1)[pick]


def stacked(row, column, tier):
    """Whether a top-tier slot is filled: a ragged skyline, full lower tiers."""
    if tier < 2:
        return True
    return (row * 5 + column * 3) % 7 not in (0, 4)


def build_spine(m):
    """Keel spine under the yard, a ventral keel and the catwalk on top."""
    length = ROW_COUNT * ROW_PITCH + 1.0
    centre = YARD_BOW - (ROW_COUNT - 1) * ROW_PITCH / 2
    box("Keel spine", (0, centre, -0.05), (0.44, length, 0.62), m["frame"], 0.04)
    box("Ventral keel", (0, centre, -0.48), (1.0, length - 0.6, 0.22), m["hull"], 0.04)
    box("Spine catwalk", (0, centre, 0.27), (0.16, length - 0.4, 0.04), m["steel"])
    for side in (-1, 1):
        box("Yard rail", (side * 1.12, centre, -0.36), (0.06, length - 0.6, 0.06), m["frame"])
        box("Yard rail", (side * 1.12, centre, 0.36), (0.06, length - 0.6, 0.05), m["frame"])


def build_yard(m):
    """The container yard: eight rows, two columns a side, up to three high."""
    for row in range(ROW_COUNT):
        y = YARD_BOW - row * ROW_PITCH
        for side in (-1, 1):
            for column, x in enumerate(COLUMNS):
                for tier, z in enumerate(TIERS):
                    if not stacked(row, column + (side > 0) * 2, tier):
                        continue
                    colour = cargo_colour(row, column + (side > 0) * 2, tier)
                    box("Container", (side * x, y, z), CONTAINER, m["cargo"][colour])
                    box("Container rib", (side * x, y, z),
                        (CONTAINER[0] + 0.02, 0.05, CONTAINER[2] + 0.02), m["rib"])
                    box("Container roof latch", (side * x, y + 0.15, z + 0.18),
                        (0.18, 0.09, 0.025), m["seal"])
        gap = y - ROW_PITCH / 2
        if row < ROW_COUNT - 1 and row % 2 == 0:
            build_gantry(m, gap)
    build_gantry(m, YARD_BOW + ROW_PITCH / 2)
    build_gantry(m, YARD_BOW - (ROW_COUNT - 0.5) * ROW_PITCH)


def build_gantry(m, y):
    """A loading gantry arching over the yard at `y`, with a lamp on each post."""
    for side in (-1, 1):
        box("Gantry post", (side * 1.16, y, 0.15), (0.08, 0.06, 1.1), m["hazard"])
        box("Gantry lamp", (side * 1.16, y + 0.04, 0.62), (0.05, 0.03, 0.05), m["lamp"])
    box("Gantry beam", (0, y, 0.76), (2.4, 0.07, 0.07), m["hazard"])
    box("Gantry trolley", (0.3 * ((int(y * 10) % 3) - 1), y, 0.71), (0.16, 0.12, 0.06),
        m["stripe"])


def build_bow(m):
    """The forward hull: lofted prow, hazard collar, tiny bridge and windows."""
    loft("Prow", [
        (2.62, 1.15, -0.6, 0.38, 0.1),
        (3.3, 1.0, -0.55, 0.34, 0.1),
        (4.0, 0.62, -0.4, 0.24, 0.07),
        (4.5, 0.22, -0.18, 0.08, 0.03),
    ], m["hull"], 0.03)
    box("Bow collar", (0, 2.68, -0.1), (2.44, 0.1, 1.0), m["frame"])
    for index in range(6):
        box("Collar hazard", (-1.0 + 0.4 * index, 2.74, 0.37), (0.2, 0.03, 0.06),
            m["hazard"] if index % 2 == 0 else m["stripe"])
    box("Bridge pylon", (0, 3.05, 0.48), (0.26, 0.3, 0.26), m["frame"], 0.02)
    box("Bridge", (0, 3.12, 0.66), (0.5, 0.3, 0.14), m["hull"], 0.02)
    box("Bridge windows", (0, 3.28, 0.67), (0.44, 0.03, 0.05), m["glass"])
    cylinder("Bridge mast", (0.16, 3.04, 0.86), 0.015, 0.3, m["steel"], 4)
    for side in (-1, 1):
        for index in range(5):
            box("Prow window", (side * (1.08 - 0.1 * index), 2.85 + 0.2 * index, 0.05),
                (0.03, 0.08, 0.04), m["lamp"])
        box("Nav light", (side * 1.17, 2.7, 0.4), (0.06, 0.06, 0.06),
            m["starboard"] if side > 0 else m["port"])
        box("Docking lamp", (side * 0.3, 4.2, 0.12), (0.08, 0.04, 0.04), m["lamp"])
    loft("Prow ridge", [
        (3.3, 0.3, 0.2, 0.46, 0.05),
        (4.0, 0.22, 0.14, 0.34, 0.04),
        (4.45, 0.06, 0.04, 0.12, 0.01),
    ], m["hull"], 0.02)
    for side in (-1, 1):
        cylinder("Sensor dome", (side * 0.55, 3.5, 0.33), 0.12, 0.1, m["steel"], 10)
        box("Prow hazard", (side * 0.42, 3.75, 0.27), (0.24, 0.06, 0.04), m["hazard"], 0.0,
            side * 0.5)
        for step in range(4):
            box("Prow service hatch", (side * 0.38, 3.4 + step * 0.19, 0.34 - step * 0.04),
                (0.16, 0.1, 0.025), m["seal"])


def build_engines(m):
    """Engine section: hull block, four nacelles, radiator wings and a mast."""
    stern = YARD_BOW - (ROW_COUNT - 0.5) * ROW_PITCH  # Aft edge of the yard.
    y = stern - 0.55
    box("Engine section", (0, y, -0.1), (2.2, 1.0, 1.0), m["hull"], 0.05)
    box("Engine hazard band", (0, y + 0.45, -0.1), (2.24, 0.08, 1.04), m["stripe"])
    for x in (-0.55, 0.55):
        for z in (-0.32, 0.12):
            at_y = y - 0.75
            cylinder("Nacelle", (x, at_y, z), 0.24, 0.6, m["steel"], 12, (QUARTER, 0, 0))
            cylinder("Nacelle ring", (x, at_y + 0.12, z), 0.26, 0.06, m["frame"], 12,
                     (QUARTER, 0, 0))
            cone("Engine bell", (x, at_y - 0.4, z), 0.25, 0.2, m["steel"], 12, tip=0.19)
            cylinder("Engine glow", (x, at_y - 0.51, z), 0.21, 0.02, m["engine"], 12,
                     (QUARTER, 0, 0))
    for side in (-1, 1):
        box("Radiator wing", (side * 1.75, y, 0.25), (1.3, 0.8, 0.05), m["steel"], 0.02)
        for index in range(4):
            box("Radiator fin", (side * 1.75, y - 0.3 + 0.2 * index, 0.29),
                (1.2, 0.04, 0.03), m["stripe"])
        box("Radiator strut", (side * 1.15, y, 0.25), (0.1, 0.5, 0.1), m["frame"])
        for index in range(4):
            box("Engine window", (side * 1.11, y - 0.3 + 0.15 * index, 0.15),
                (0.03, 0.07, 0.04), m["lamp"])
    cylinder("Comms mast", (0.6, y + 0.1, 0.65), 0.03, 0.5, m["steel"], 6)
    cone("Comms dish", (0.6, y + 0.1, 0.92), 0.2, 0.08, m["steel"], 10, (0.6, 0, 0), tip=0.04)
    box("Beacon", (-0.6, y, 0.44), (0.06, 0.06, 0.06), m["port"])


def build():
    clear_scene()
    parts = palette()
    build_spine(parts)
    build_yard(parts)
    build_bow(parts)
    build_engines(parts)
    stage((0, 0.2, 0.1), 9.4, "")
    return publish("freighter", preview_scale=9.4, sprite_scale=8.8, sprite_pixels=512)


if __name__ == "__main__":
    print(build())
