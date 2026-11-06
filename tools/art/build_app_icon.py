"""Render the application icon: a ringed world on a gunmetal plate with a neon orbit.

blender --background --factory-startup --python tools/art/build_app_icon.py
Writes assets/source/app-icon.blend and assets/app-icon/app-icon.png (1024 px).
"""
from pathlib import Path
import math
import sys
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from celestial_materials import surface

ROOT = Path(__file__).resolve().parents[2]


def emission(name, color, strength):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    emit = nodes.new("ShaderNodeEmission")
    emit.inputs["Color"].default_value = (*color, 1)
    emit.inputs["Strength"].default_value = strength
    mat.node_tree.links.new(emit.outputs["Emission"], out.inputs["Surface"])
    return mat


def metal(name):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    shader = mat.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (0.11, 0.13, 0.14, 1)
    shader.inputs["Metallic"].default_value = 1.0
    shader.inputs["Roughness"].default_value = 0.28
    return mat


def setup():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 64
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = 1024
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.view_settings.view_transform = "AgX"
    scene.world.color = (0.02, 0.025, 0.03)
    bpy.ops.object.camera_add(location=(0, 0, 10))
    camera = bpy.context.object
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 2.2
    scene.camera = camera
    for location, energy, color in [((-4, 4, 6), 900, (1, 0.93, 0.82)),
                                    ((4, -3, 3), 250, (0.4, 0.7, 1.0))]:
        bpy.ops.object.light_add(type="AREA", location=location)
        light = bpy.context.object
        light.data.energy = energy
        light.data.color = color
        light.data.size = 4
        light.rotation_euler = (-light.location).to_track_quat("-Z", "Y").to_euler()
    return scene


def build():
    scene = setup()
    # Chamfered plate echoing the in-game HUD cards.
    bpy.ops.mesh.primitive_cube_add(size=2, location=(0, 0, -0.3))
    plate = bpy.context.object
    plate.scale = (0.98, 0.98, 0.08)
    bevel = plate.modifiers.new("chamfer", "BEVEL")
    bevel.width = 0.16
    bevel.segments = 1
    plate.data.materials.append(metal("gunmetal"))
    # Ringed gas world.
    bpy.ops.mesh.primitive_uv_sphere_add(segments=96, ring_count=48, radius=0.5, location=(0.05, 0.08, 0.3))
    world = bpy.context.object
    world.data.materials.append(surface("icon-world", [(0.18, 0.1, 0.1), (0.55, 0.27, 0.15),
        (0.85, 0.63, 0.4), (0.95, 0.86, 0.68)], "bands"))
    world.rotation_euler = (math.radians(70), 0, math.radians(-25))
    for polygon in world.data.polygons:
        polygon.use_smooth = True
    bpy.ops.mesh.primitive_torus_add(major_radius=0.78, minor_radius=0.025, location=(0.05, 0.08, 0.3))
    ring = bpy.context.object
    ring.scale = (1, 0.36, 1)
    ring.rotation_euler = (0, 0, math.radians(-25))
    ring.data.materials.append(emission("neon", (0.3, 1.0, 0.12), 2.2))
    # A small freighter wedge riding the ring.
    bpy.ops.mesh.primitive_cone_add(vertices=3, radius1=0.12, depth=0.04, location=(-0.62, 0.38, 0.45))
    ship = bpy.context.object
    ship.rotation_euler = (0, 0, math.radians(35))
    ship.data.materials.append(emission("hull", (0.2, 0.85, 0.88), 2.0))
    scene.render.filepath = str(ROOT / "assets/app-icon/app-icon.png")
    bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "assets/source/app-icon.blend"))
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    build()
