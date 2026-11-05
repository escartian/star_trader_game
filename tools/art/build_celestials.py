"""Export editable celestial models and transparent static sprites.

blender --background --factory-startup --python tools/art/build_celestials.py
No downloaded textures, animation frame sets, or runtime Blender dependency.
"""
from pathlib import Path
import math
import sys
import bpy
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
from celestial_materials import STAR_COLORS, surface

ROOT = Path(__file__).resolve().parents[2]


def setup():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 32
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = 256
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.image_settings.color_depth = "8"
    scene.world.color = (0.025, 0.025, 0.025)
    scene.view_settings.view_transform = "AgX"
    bpy.ops.object.camera_add(location=(0, -6, 1.8))
    camera = bpy.context.object
    camera.rotation_euler = (-camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 2.25
    scene.camera = camera
    for location, energy, color, size in [((-3, -5, 5), 650, (1, 0.91, 0.77), 3),
                                        ((3, 2, 1), 110, (0.35, 0.55, 1), 2)]:
        bpy.ops.object.light_add(type="AREA", location=location)
        light = bpy.context.object
        light.data.energy = energy
        light.data.color = color
        light.data.size = size
        light.rotation_euler = (-light.location).to_track_quat('-Z', 'Y').to_euler()
    return scene


def sphere(name, material, rough=False):
    if rough:
        bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=4, radius=0.85)
    else:
        bpy.ops.mesh.primitive_uv_sphere_add(segments=64, ring_count=32, radius=1)
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(material)
    if rough:
        for vertex in obj.data.vertices:
            p = vertex.co.normalized()
            amount = 1 + 0.12 * math.sin(8 * p.x + 3 * p.z) * math.cos(9 * p.y)
            vertex.co *= amount
        obj.scale = (1.0, 0.78, 0.87)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True
    return obj


def moon_craters(obj):
    craters = [(Vector(point).normalized(), size) for point, size in [
        ((-0.4, -1, 0.5), 0.22), ((0.5, -1, 0.1), 0.32),
        ((-0.1, -1, -0.5), 0.18), ((0.8, -0.5, 0.7), 0.15)]]
    for vertex in obj.data.vertices:
        direction = vertex.co.normalized()
        delta = 0
        for center, width in craters:
            distance = (direction - center).length / width
            delta -= 0.07 * math.exp(-distance ** 4 * 3)
            delta += 0.025 * math.exp(-((distance - 1) * 7) ** 2)
        vertex.co *= 1 + delta


def export(name, colors, style):
    scene = setup()
    obj = sphere(name, surface(name, colors, style), rough=style == "asteroid")
    if style == "moon":
        moon_craters(obj)
    scene.render.filepath = f"//../sprites/{name}.png"
    bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "assets/source" / f"{name}.blend"))
    bpy.ops.render.render(write_still=True)


def main():
    for name, colors in STAR_COLORS.items():
        export(name, colors, "star")
    export("rocky-planet", [(0.015, 0.06, 0.16), (0.035, 0.2, 0.35),
        (0.15, 0.32, 0.16), (0.5, 0.46, 0.27), (0.8, 0.85, 0.75)], "ocean")
    export("gas-giant", [(0.18, 0.1, 0.1), (0.55, 0.27, 0.15),
        (0.85, 0.63, 0.4), (0.95, 0.86, 0.68)], "bands")
    export("asteroid", [(0.055, 0.04, 0.03), (0.25, 0.19, 0.13), (0.55, 0.44, 0.3)], "asteroid")
    export("moon", [(0.07, 0.08, 0.1), (0.33, 0.35, 0.38), (0.65, 0.67, 0.7)], "moon")


if __name__ == "__main__":
    main()
