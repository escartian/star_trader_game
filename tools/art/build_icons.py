"""Build the editable interface-icon scene and export one SVG per icon.

Rebuild everything (overwrites assets/source/icons.blend and assets/icons/*.svg):
  blender --background --factory-startup --python tools/art/build_icons.py
Export SVGs from a hand-edited icons.blend without rebuilding the scene:
  blender --background assets/source/icons.blend --python tools/art/build_icons.py -- --export-only

Scene layout: one Empty per icon (named "icon:<name>") on a grid, parenting 2D
POLY curve objects in grid units with y up. Each curve carries custom properties
"kind" ("stroke" or "fill") and "opacity". Strokes get a bevel so the scene reads
in Blender's viewport; the exporter writes white paths that the game tints.
"""
from pathlib import Path
import sys
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from icon_shapes import ICONS, STROKE, elements

ROOT = Path(__file__).resolve().parents[2]
SVG_DIR = ROOT / "assets/icons"
BLEND = ROOT / "assets/source/icons.blend"
COLUMNS = 10
SPACING = 24.0


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for curve in list(bpy.data.curves):
        bpy.data.curves.remove(curve)


def material(name, color):
    mat = bpy.data.materials.get(name) or bpy.data.materials.new(name)
    mat.diffuse_color = color
    return mat


def add_curve(parent, index, kind, points, closed, opacity):
    data = bpy.data.curves.new(f"{parent.name}:{index}", "CURVE")
    data.dimensions = "2D"
    data.fill_mode = "BOTH" if kind == "fill" else "NONE"
    if kind == "stroke":
        data.bevel_depth = STROKE / 2
    spline = data.splines.new("POLY")
    spline.points.add(len(points) - 1)
    for point, (x, y) in zip(spline.points, points):
        point.co = (x, -y, 0.0, 1.0)  # Blender is y-up; SVG grid is y-down.
    spline.use_cyclic_u = closed or kind == "fill"
    obj = bpy.data.objects.new(data.name, data)
    obj["kind"] = kind
    obj["opacity"] = float(opacity)
    obj.data.materials.append(material("icon-" + kind, (0.58, 1.0, 0.33, opacity)))
    bpy.context.collection.objects.link(obj)
    obj.parent = parent
    return obj


def build():
    clear_scene()
    for index, name in enumerate(ICONS):
        empty = bpy.data.objects.new(f"icon:{name}", None)
        empty.empty_display_size = 10
        empty.location = ((index % COLUMNS) * SPACING, -(index // COLUMNS) * SPACING, 0)
        bpy.context.collection.objects.link(empty)
        for item_index, (kind, points, extra) in enumerate(elements(name)):
            closed = extra if kind == "stroke" else True
            opacity = 1.0 if kind == "stroke" else extra
            add_curve(empty, item_index, kind, points, closed, opacity)
    BLEND.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(BLEND))


def fmt(value):
    return f"{value:.3f}".rstrip("0").rstrip(".")


def export():
    SVG_DIR.mkdir(parents=True, exist_ok=True)
    for stale in SVG_DIR.glob("*.svg"):
        stale.unlink()
    count = 0
    for empty in bpy.data.objects:
        if empty.type != "EMPTY" or not empty.name.startswith("icon:"):
            continue
        name = empty.name[len("icon:"):]
        paths = []
        for child in sorted(empty.children, key=lambda obj: int(obj.name.rsplit(":", 1)[-1])):
            ox, oy = child.location.x, child.location.y
            for spline in child.data.splines:
                coords = [(p.co.x + ox, -(p.co.y + oy)) for p in spline.points]
                d = "M" + " L".join(f"{fmt(x)} {fmt(y)}" for x, y in coords)
                if spline.use_cyclic_u:
                    d += " Z"
                if child.get("kind") == "fill":
                    paths.append(f'<path d="{d}" stroke="none" fill="#fff" fill-opacity="{fmt(child.get("opacity", 1.0))}"/>')
                else:
                    paths.append(f'<path d="{d}"/>')
        body = "\n  ".join(paths)
        svg = (
            '<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10 -10 20 20" width="20" height="20">\n'
            f'  <g fill="none" stroke="#fff" stroke-width="{STROKE}" stroke-linecap="round" stroke-linejoin="round">\n'
            f"  {body}\n  </g>\n</svg>\n"
        )
        (SVG_DIR / f"{name}.svg").write_text(svg)
        count += 1
    print(f"Exported {count} icons to {SVG_DIR}")


if __name__ == "__main__":
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if "--export-only" not in args:
        build()
    export()
