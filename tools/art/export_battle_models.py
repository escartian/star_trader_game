"""Export ship .blend sources as runtime glTF models for the battle view.

Blender is authoring-only: the game reads the exported .glb files. Models keep
Blender's axes (+Z up, nose along +Y) and their material or per-vertex colours;
modifiers such as bevels are applied on export. Dense scans (the AI courier that
is the scout's art) are decimated to a battle-sized triangle budget on export;
the .blend source itself is never modified. Run from the repository root:

    /Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
      --python tools/art/export_battle_models.py

Append `-- scout` (any names) to export only those ships.
"""
from pathlib import Path
import sys
import bpy

ROOT = Path(__file__).resolve().parents[2]
# Export name -> (source .blend, triangle budget or None to keep every face).
# The freighter, patrol and prospector are script-built and export themselves
# (tools/art/build_<name>.py); only scanned hulls are exported here. Battle
# models may use up to 7,000 triangles each.
SHIPS = {
    "scout": ("ai-courier.blend", 1600),
}


def triangles(obj):
    return sum(len(polygon.vertices) - 2 for polygon in obj.data.polygons)


def decimate(budget):
    """Collapse-decimates every mesh so the model has about `budget` triangles."""
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    total = sum(triangles(obj) for obj in meshes)
    if total <= budget:
        return
    for obj in meshes:
        modifier = obj.modifiers.new("Battle budget", "DECIMATE")
        modifier.decimate_type = "COLLAPSE"
        modifier.ratio = budget / total
        modifier.use_collapse_triangulate = True


def export(name):
    source, budget = SHIPS[name]
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / "assets/source" / source))
    if budget:
        decimate(budget)
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
        export_vertex_color="MATERIAL",
        export_cameras=False,
        export_lights=False,
        export_animations=False,
    )
    print(f"exported {target.relative_to(ROOT)}")


if __name__ == "__main__":
    names = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else SHIPS
    for name in names:
        export(name)
