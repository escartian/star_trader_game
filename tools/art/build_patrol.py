"""Build the patrol ship, an original armoured gunship, and export its assets.

Writes assets/source/patrol.blend, assets/models/patrol.glb, the 256px map
sprite assets/sprites/patrol.png and a review render
(assets/source/patrol-preview.png). Shared conventions and the triangle budget
live in hull_kit.py. The patrol keeps its identity from the first prototype:
blue-grey titanium, swept armoured wings, orange forward hardpoints, a teal
canopy and blue engines. Detail follows the prospector: layered armour plates,
missile racks, sensors, lit windows and banded engines.

Run from the repository root:

    blender --background --factory-startup --python tools/art/build_patrol.py

or through the Blender MCP by exec-ing this file with ``__file__`` set and
calling ``build()``.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hull_kit import (  # noqa: E402
    QUARTER, box, clear_scene, cone, cylinder, loft, material, publish, shell, stage,
)

# Cuboid faces for an 8-vertex slab (4 bottom corners, then the same 4 on top).
SLAB_FACES = [(3, 2, 1, 0), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5),
              (2, 3, 7, 6), (3, 0, 4, 7)]


def palette():
    """Linear colours."""
    return {
        "hull": material("Titanium hull", (0.32, 0.43, 0.53), 0.6, 0.4),
        "armor": material("Armour plate", (0.20, 0.27, 0.34), 0.7, 0.45),
        "frame": material("Gunmetal frame", (0.11, 0.12, 0.14), 0.6, 0.45),
        "gun": material("Hardpoint orange", (0.88, 0.32, 0.06), 0.35, 0.5),
        "steel": material("Bare steel", (0.48, 0.50, 0.52), 0.85, 0.35),
        "stripe": material("Hazard black", (0.03, 0.03, 0.035), 0.2, 0.6),
        "ivory": material("Missile ivory", (0.72, 0.70, 0.64), 0.2, 0.5),
        "glass": material("Canopy", (0.025, 0.16, 0.21), 0.8, 0.15),
        "engine": material("Engine blue", (0.08, 0.6, 1.0), emission=3.0),
        "lamp": material("Window light", (1.0, 0.94, 0.78), emission=2.0),
        "port": material("Port light", (1.0, 0.08, 0.05), emission=3.0),
        "starboard": material("Starboard light", (0.1, 1.0, 0.25), emission=3.0),
    }


def slab(name, side, outline, low, high, surface, bevel=0.0):
    """A flat plate over an (x, y) outline, mirrored to `side` with outward normals."""
    vertices = [(side * x, y, low) for x, y in outline]
    vertices += [(side * x, y, high) for x, y in outline]
    faces = SLAB_FACES if side > 0 else [tuple(reversed(face)) for face in SLAB_FACES]
    return shell(name, vertices, faces, surface, bevel)


def build_fuselage(m):
    """Lofted fuselage to a sharp nose, canopy with frame, spine and chin guns."""
    loft("Fuselage", [
        (-2.3, 0.48, -0.3, 0.3, 0.12),
        (-1.9, 0.56, -0.34, 0.4, 0.15),
        (-0.9, 0.62, -0.38, 0.46, 0.18),
        (0.3, 0.56, -0.34, 0.42, 0.15),
        (1.3, 0.38, -0.24, 0.3, 0.1),
        (2.2, 0.16, -0.12, 0.12, 0.04),
        (2.8, 0.03, -0.03, 0.03, 0.01),
    ], m["hull"], 0.02)
    loft("Canopy", [
        (0.05, 0.27, 0.38, 0.6, 0.04),
        (0.55, 0.24, 0.34, 0.55, 0.03),
        (1.05, 0.14, 0.28, 0.4, 0.02),
        (1.35, 0.04, 0.24, 0.28, 0.01),
    ], m["glass"])
    box("Canopy frame", (0, 0.55, 0.56), (0.5, 0.05, 0.06), m["frame"])
    box("Canopy spine", (0, 0.6, 0.57), (0.05, 0.9, 0.04), m["frame"])
    loft("Dorsal spine", [
        (-2.1, 0.14, 0.24, 0.5, 0.03),
        (-0.9, 0.17, 0.36, 0.62, 0.03),
        (-0.05, 0.1, 0.4, 0.52, 0.02),
    ], m["armor"], 0.01)
    for index in range(4):
        box("Spine vent", (0, -1.75 + 0.22 * index, 0.55 - 0.02 * (3 - index)),
            (0.22, 0.08, 0.04), m["stripe"])
    box("Nose stripe", (0, 1.75, 0.2), (0.3, 0.1, 0.02), m["gun"])
    cylinder("Sensor dome", (0, 1.3, -0.26), 0.12, 0.08, m["steel"], 10)
    for side in (-1, 1):
        slab("Cheek armour", side, [(0.6, -1.6), (0.6, 0.4), (0.7, 0.2), (0.7, -1.5)],
             -0.18, 0.24, m["armor"], 0.01)
        for index in range(5):
            box("Hull window", (side * 0.71, -1.25 + 0.25 * index, 0.05), (0.03, 0.09, 0.04),
                m["lamp"])
        box("Chin gun mount", (side * 0.24, 0.9, -0.24), (0.16, 0.24, 0.14), m["frame"], 0.02)
        cylinder("Chin gun", (side * 0.24, 1.3, -0.25), 0.04, 0.6, m["gun"], 8, (QUARTER, 0, 0))
        cylinder("Chin muzzle", (side * 0.24, 1.62, -0.25), 0.055, 0.08, m["frame"], 8,
                 (QUARTER, 0, 0))
        box("Intake", (side * 0.5, -0.5, -0.3), (0.26, 0.6, 0.16), m["frame"], 0.02)
        box("Intake lip", (side * 0.5, -0.18, -0.3), (0.28, 0.05, 0.18), m["gun"])
        box("Landing lamp", (side * 0.3, 1.6, -0.12), (0.06, 0.03, 0.03), m["lamp"])


def wing(m, side):
    """A swept wing with layered armour, missile rack and a wingtip cannon pod."""
    slab("Wing", side, [(0.6, -1.8), (0.6, 0.45), (2.05, -0.55), (2.05, -1.4)],
         -0.1, 0.02, m["hull"], 0.015)
    slab("Wing armour", side, [(0.72, -1.55), (0.72, 0.15), (1.75, -0.6), (1.75, -1.3)],
         0.02, 0.07, m["armor"], 0.01)
    slab("Wing armour cap", side, [(0.8, -1.4), (0.8, -0.3), (1.35, -0.75), (1.35, -1.2)],
         0.07, 0.1, m["hull"], 0.01)
    slab("Leading edge", side, [(0.62, 0.38), (0.62, 0.48), (2.0, -0.47), (2.0, -0.57)],
         -0.08, 0.04, m["frame"])
    slab("Flap", side, [(0.9, -1.85), (0.9, -1.7), (1.9, -1.45), (1.9, -1.55)],
         -0.08, 0.0, m["frame"])
    for offset in (0.0, 0.14):  # Twin squadron stripes, parallel to the fuselage.
        box("Wing stripe", (side * (1.5 + offset), -1.0, 0.075), (0.06, 0.5, 0.01), m["gun"])
    for index in range(3):  # Under-wing missile rack.
        x = side * (0.95 + 0.22 * index)
        box("Missile pylon", (x, -0.7, -0.15), (0.05, 0.5, 0.08), m["frame"])
        cylinder("Missile", (x, -0.65, -0.24), 0.05, 0.7, m["ivory"], 8, (QUARTER, 0, 0))
        cone("Missile tip", (x, -0.2, -0.24), 0.05, 0.16, m["gun"], 8)
    x = side * 2.1
    cylinder("Cannon pod", (x, -0.95, -0.04), 0.14, 1.0, m["armor"], 12, (QUARTER, 0, 0))
    cylinder("Pod band", (x, -0.7, -0.04), 0.155, 0.08, m["stripe"], 12, (QUARTER, 0, 0))
    cone("Pod nose", (x, -0.3, -0.04), 0.14, 0.3, m["armor"], 12, tip=0.07)
    cone("Pod tail", (x, -1.55, -0.04), 0.14, 0.2, m["steel"], 12, (QUARTER, 0, 0), tip=0.08)
    cylinder("Cannon barrel", (x, 0.3, -0.04), 0.045, 0.9, m["gun"], 8, (QUARTER, 0, 0))
    cylinder("Barrel shroud", (x, 0.05, -0.04), 0.065, 0.25, m["frame"], 8, (QUARTER, 0, 0))
    cylinder("Muzzle", (x, 0.76, -0.04), 0.065, 0.1, m["frame"], 8, (QUARTER, 0, 0))
    box("Nav light", (x, -1.68, -0.04), (0.06, 0.06, 0.06),
        m["starboard"] if side > 0 else m["port"])


def build_turret(m):
    """A dorsal twin-gun turret behind the canopy, with a sensor mast."""
    cylinder("Turret ring", (0, -0.45, 0.6), 0.27, 0.1, m["frame"], 14)
    loft("Turret housing", [
        (-0.7, 0.2, 0.62, 0.8, 0.03),
        (-0.3, 0.2, 0.62, 0.82, 0.03),
        (-0.15, 0.12, 0.64, 0.76, 0.02),
    ], m["armor"], 0.01)
    for side in (-1, 1):
        cylinder("Turret barrel", (side * 0.09, 0.05, 0.71), 0.035, 0.6, m["gun"], 8,
                 (QUARTER, 0, 0))
        cylinder("Turret muzzle", (side * 0.09, 0.36, 0.71), 0.05, 0.06, m["frame"], 8,
                 (QUARTER, 0, 0))
    cylinder("Sensor mast", (0.12, -0.65, 0.95), 0.015, 0.3, m["steel"], 4)
    box("Sensor head", (0.12, -0.65, 1.1), (0.08, 0.08, 0.04), m["port"])


def surface_details(m):
    """Small readable vents, fasteners and service panels on the visible hull."""
    for side in (-1, 1):
        for row in range(7):
            y = -1.38 + row * 0.17
            box("Wing cooling slot", (side * 1.12, y, 0.115),
                (0.35, 0.055, 0.018), m["frame"])
        for row in range(8):
            y = -1.52 + row * 0.2
            for x in (0.79, 1.57):
                box("Armour fastener", (side * x, y, 0.116),
                    (0.045, 0.045, 0.02), m["steel"])
        for row in range(7):
            y = -1.22 + row * 0.29
            box("Cheek panel seam", (side * 0.72, y, 0.18),
                (0.02, 0.045, 0.15), m["steel"])
        for row in range(5):
            y = -2.42 + row * 0.18
            box("Nacelle heat rib", (side * 0.56, y, 0.29),
                (0.37, 0.035, 0.025), m["frame"])
    for y in (0.25, 0.7, 1.08):
        box("Canopy crossbar", (0, y, 0.58 - y * 0.1),
            (0.4 - y * 0.18, 0.035, 0.035), m["steel"])


def build_engines(m):
    """Twin banded engines with bells and glow, a centre exhaust and tail fins."""
    for side in (-1, 1):
        x = side * 0.56
        cylinder("Nacelle", (x, -2.05, 0.0), 0.29, 1.1, m["hull"], 14, (QUARTER, 0, 0))
        for offset in (-0.25, 0.2):
            cylinder("Nacelle band", (x, -2.05 + offset, 0.0), 0.31, 0.07, m["stripe"], 14,
                     (QUARTER, 0, 0))
        cone("Nacelle intake", (x, -1.35, 0.0), 0.29, 0.3, m["hull"], 14, tip=0.17)
        cone("Engine bell", (x, -2.72, 0.0), 0.27, 0.24, m["steel"], 14, tip=0.21)
        cylinder("Engine glow", (x, -2.85, 0.0), 0.23, 0.02, m["engine"], 14, (QUARTER, 0, 0))
        fin = [(0, -2.45, 0.2), (0, -1.6, 0.2), (0, -2.25, 0.8), (0, -2.55, 0.8)]
        width = 0.05
        vertices = [(side * (0.32 + width * k) + side * 0.25 * (v[2] - 0.2), v[1], v[2])
                    for k in (0, 1) for v in fin]
        faces = [(0, 1, 2, 3), (7, 6, 5, 4), (0, 4, 5, 1), (1, 5, 6, 2),
                 (2, 6, 7, 3), (3, 7, 4, 0)]
        if side > 0:
            faces = [tuple(reversed(face)) for face in faces]
        shell("Tail fin", vertices, faces, m["armor"], 0.01)
        box("Fin tip light", (side * (0.52), -2.42, 0.82), (0.05, 0.05, 0.04),
            m["starboard"] if side > 0 else m["port"])
        box("Fin stripe", (side * 0.47, -2.2, 0.62), (0.07, 0.25, 0.05), m["gun"], 0.0)
    cylinder("Centre exhaust", (0, -2.35, 0.0), 0.18, 0.2, m["steel"], 12, (QUARTER, 0, 0))
    cylinder("Centre glow", (0, -2.46, 0.0), 0.14, 0.02, m["engine"], 12, (QUARTER, 0, 0))
    box("Ventral fin", (0, -1.9, -0.45), (0.05, 0.7, 0.3), m["armor"], 0.01)


def build():
    clear_scene()
    parts = palette()
    build_fuselage(parts)
    for side in (-1, 1):
        wing(parts, side)
    build_turret(parts)
    build_engines(parts)
    surface_details(parts)
    stage((0, 0.0, 0.1), 6.6, "")
    return publish("patrol", preview_scale=6.6, sprite_scale=6.0, sprite_pixels=512)


if __name__ == "__main__":
    print(build())
