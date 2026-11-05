"""Build an original trade-port model and transparent map sprite in Blender.

Run from anywhere with:
  blender --background --factory-startup --python tools/art/build_trade_port.py
"""
from pathlib import Path
import math
import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]


def material(name, color, metallic=0.0, emission=0.0):
    result = bpy.data.materials.new(name)
    result.diffuse_color = (*color, 1)
    result.use_nodes = True
    shader = result.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = 0.32
    shader.inputs["Emission Color"].default_value = (*color, 1)
    shader.inputs["Emission Strength"].default_value = emission
    return result


def finish(obj, name, surface):
    obj.name = name
    obj.data.materials.append(surface)
    bevel = obj.modifiers.new("Soft manufactured edges", "BEVEL")
    bevel.width = 0.06
    bevel.segments = 2
    obj.modifiers.new("Weighted normals", "WEIGHTED_NORMAL")
    return obj


def box(name, location, scale, surface, angle=0):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.scale = scale
    obj.rotation_euler.z = angle
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return finish(obj, name, surface)


def cylinder(name, radius, depth, z, surface):
    bpy.ops.mesh.primitive_cylinder_add(vertices=32, radius=radius, depth=depth, location=(0, 0, z))
    return finish(bpy.context.object, name, surface)


def ring(name, radius, thickness, z, surface):
    bpy.ops.mesh.primitive_torus_add(major_segments=64, minor_segments=12,
        major_radius=radius, minor_radius=thickness, location=(0, 0, z))
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(surface)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True


def light(name, location, energy, color, size):
    bpy.ops.object.light_add(type="AREA", location=location)
    obj = bpy.context.object
    obj.name = name
    obj.data.energy = energy
    obj.data.color = color
    obj.data.shape = "DISK"
    obj.data.size = size
    obj.rotation_euler = (Vector((0, 0, 0)) - obj.location).to_track_quat('-Z', 'Y').to_euler()


def build():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    hull = material("Ceramic titanium", (0.48, 0.58, 0.66), 0.7)
    dark = material("Graphite structure", (0.055, 0.085, 0.12), 0.6)
    solar = material("Photovoltaic blue", (0.035, 0.18, 0.37), 0.55)
    cargo = material("Cargo ochre", (0.88, 0.34, 0.065), 0.35)
    cyan = material("Dock guidance cyan", (0.08, 0.9, 1.0), emission=3)
    amber = material("Approach beacons", (1.0, 0.42, 0.06), emission=4)

    cylinder("Docking spine", 0.67, 1.8, 0, dark)
    cylinder("Command deck", 0.82, 0.28, 0.75, hull)
    cylinder("Guidance crown", 0.72, 0.07, 0.93, cyan)
    cylinder("Upper dock", 0.5, 0.32, 1.05, hull)
    cylinder("Dock mouth", 0.33, 0.03, 1.22, dark)
    ring("Habitat torus", 2.05, 0.28, 0, hull)
    ring("Continuous window band", 2.08, 0.045, 0.23, cyan)
    for index in range(4):
        angle = index * math.tau / 4
        x, y = math.cos(angle), math.sin(angle)
        box(f"Spoke {index}", (x * 1.25, y * 1.25, 0), (2.0, 0.24, 0.24), dark, angle)
        box(f"Freight bay {index}", (x * 2.0, y * 2.0, 0.3), (0.8, 0.5, 0.42), cargo, angle)
        box(f"Approach light {index}", (x * 2.4, y * 2.4, 0.5), (0.18, 0.18, 0.12), amber)
    for side in [-1, 1]:
        box(f"Solar boom {side}", (side * 3.0, 0, -0.35), (2.5, 0.15, 0.15), hull)
        for bank in [-1, 1]:
            box(f"Panel frame {side} {bank}", (side * 3.8, bank * 0.85, -0.35), (1.7, 1.45, 0.12), dark)
            for row in range(4):
                for column in range(3):
                    box("Solar cell", (side * 3.8 + (column - 1) * 0.5,
                        bank * 0.85 + (row - 1.5) * 0.32, -0.27), (0.46, 0.28, 0.035), solar)

    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 32
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = 512
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.world.color = (0.12, 0.12, 0.12)
    light("Warm key", (1, -4, 8), 1700, (1.0, 0.84, 0.69), 7)
    light("Cool rim", (-5, 3, 4), 2200, (0.38, 0.67, 1.0), 5)
    light("Front fill", (5, -7, 1), 700, (0.65, 0.9, 1.0), 4)
    bpy.ops.object.camera_add(location=(8, -11, 10))
    camera = bpy.context.object
    camera.rotation_euler = (-camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 11
    scene.camera = camera
    scene.render.filepath = "//../sprites/trade-port.png"
    bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "assets/source/trade-port.blend"))
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    build()
