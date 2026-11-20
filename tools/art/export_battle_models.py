"""Export ship .blend sources as runtime glTF models for the battle view.

Blender is authoring-only: the game reads the exported .glb files. Models keep
Blender's axes (+Z up, nose along +Y) and plain material colours; modifiers such
as bevels are applied on export. Run from the repository root:

    /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
      --python tools/art/export_battle_models.py

Append `-- freighter` (any names) to export only those ships.
"""
from pathlib import Path
import sys
import bpy

ROOT = Path(__file__).resolve().parents[2]
# Low-poly sources the battle view draws as meshes; other hulls keep sprites.
SHIPS = ["freighter"]


def export(name):
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / "assets/source" / f"{name}.blend"))
    for obj in bpy.context.scene.objects:
        obj.select_set(obj.type == "MESH")
    target = ROOT / "assets/models" / f"{name}.glb"
    target.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(target),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_yup=False,
        export_normals=True,
        export_texcoords=False,
        export_materials="EXPORT",
        export_cameras=False,
        export_lights=False,
        export_animations=False,
    )
    print(f"exported {target.relative_to(ROOT)}")


if __name__ == "__main__":
    names = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else SHIPS
    for name in names:
        export(name)
