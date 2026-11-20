"""Level the existing editable Scout scan when its original .npz is unavailable.

Run once with Blender from the repository root. The shared correction also runs
for future raw-scan rebuilds in clean_ai_courier.py.
"""

from pathlib import Path
import sys

import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from clean_ai_courier import SHIP_LENGTH, SPRITE_SCALE, level_attitude


SOURCE = Path(__file__).resolve().parents[2] / "assets/source/ai-courier.blend"


def main():
    bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
    ship = next(obj for obj in bpy.context.scene.objects if obj.type == "MESH")
    vertices = np.array([tuple(vertex.co) for vertex in ship.data.vertices])
    vertices = level_attitude(vertices)
    vertices -= (vertices.min(0) + vertices.max(0)) / 2
    vertices *= SHIP_LENGTH / np.ptp(vertices[:, 1])
    for vertex, position in zip(ship.data.vertices, vertices):
        vertex.co = position
    ship.data.update()
    bpy.context.scene.camera.data.ortho_scale = SPRITE_SCALE
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))
    print(f"Levelled {ship.name}: {tuple(round(d, 2) for d in ship.dimensions)}")


if __name__ == "__main__":
    main()
