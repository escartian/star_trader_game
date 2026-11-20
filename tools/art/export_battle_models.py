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

sys.path.insert(0, str(Path(__file__).resolve().parent))
from clean_ai_courier import SPRITE_RESOLUTION, SPRITE_SCALE

ROOT = Path(__file__).resolve().parents[2]
# Export name -> (source .blend, triangle budget or None to keep every face).
# The freighter, patrol and prospector are script-built and export themselves
# (tools/art/build_<name>.py); only scanned hulls are exported here. Battle
# models may use up to 7,000 triangles each.
SHIPS = {
    "scout": ("ai-courier.blend", 6000),
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


def paint_scout():
    """Give the scanned scout readable hull, canopy, wing and engine paint.

    The source scan's vertex colours are nearly uniform brown. Repaint only the
    export copy; the editable .blend remains the original scan. Blender stores
    these FLOAT_COLOR values in linear space, which glTF exports as COLOR_0.
    """
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH":
            continue
        paint = obj.data.color_attributes.active_color
        if paint is None or paint.domain != "POINT":
            raise RuntimeError("Scout source needs per-vertex paint")
        for vertex in obj.data.vertices:
            x, y, z = vertex.co
            if y > 0.35 and abs(x) < 0.48 and z > 0.18:
                base = (0.04, 0.22, 0.27)  # canopy
            elif (y > 1.1 and abs(x) < 0.28 and z > -0.05) or (
                abs(x) > 0.65 and y > -0.6
            ):
                base = (0.08, 0.42, 0.38)  # nose and wing markings
            elif y < -1.0 and abs(x) > 0.45:
                base = (0.34, 0.53, 0.58)  # engine housings
            else:
                base = (0.69, 0.75, 0.70)  # pale hull
            old = paint.data[vertex.index].color
            variation = max(0.78, min(1.1, 0.85 + 0.5 * sum(old[:3]) / 3))
            paint.data[vertex.index].color = (*[channel * variation for channel in base], 1.0)


def export(name):
    source, budget = SHIPS[name]
    bpy.ops.wm.open_mainfile(filepath=str(ROOT / "assets/source" / source))
    if name == "scout":
        # The map sprite keeps the scan's detailed vertex colours. Battle paint
        # is applied only to the exported GLB after this render.
        bpy.context.scene.render.resolution_x = 768
        bpy.context.scene.render.resolution_y = 768
        bpy.context.scene.camera.data.ortho_scale = SPRITE_SCALE
        bpy.context.scene.render.filepath = str(ROOT / "assets/source/scout-preview.png")
        bpy.ops.render.render(write_still=True)
        bpy.context.scene.render.resolution_x = SPRITE_RESOLUTION
        bpy.context.scene.render.resolution_y = SPRITE_RESOLUTION
        bpy.context.scene.render.filepath = str(ROOT / "assets/sprites/scout.png")
        bpy.ops.render.render(write_still=True)
        paint_scout()
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
