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


def finish(obj, name, surface, bevel=True):
    """Names and paints a part. Small details skip the bevel: it would spend
    triangles (the runtime model has a budget) on edges too small to see."""
    obj.name = name
    obj.data.materials.append(surface)
    if bevel:
        modifier = obj.modifiers.new("Soft manufactured edges", "BEVEL")
        modifier.width = 0.06
        modifier.segments = 2
    obj.modifiers.new("Weighted normals", "WEIGHTED_NORMAL")
    return obj


def box(name, location, scale, surface, angle=0, bevel=True, tilt=(0, 0)):
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.scale = scale
    obj.rotation_euler = (tilt[0], tilt[1], angle)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    return finish(obj, name, surface, bevel)


def cylinder(name, radius, depth, z, surface, location=None, vertices=32, bevel=True, rotation=(0, 0, 0)):
    place = location if location is not None else (0, 0, z)
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth,
        location=place, rotation=rotation)
    return finish(bpy.context.object, name, surface, bevel)


def ring(name, radius, thickness, z, surface):
    bpy.ops.mesh.primitive_torus_add(major_segments=64, minor_segments=12,
        major_radius=radius, minor_radius=thickness, location=(0, 0, z))
    obj = bpy.context.object
    obj.name = name
    obj.data.materials.append(surface)
    for polygon in obj.data.polygons:
        polygon.use_smooth = True


def details(hull, dark, cyan, amber, red, green, plating):
    """Structure added for the 3D menu model; also visible in the map sprite."""
    # Pressurised modules around the torus, with a lit window row on each.
    for index in range(12):
        angle = (index + 0.5) * math.tau / 12
        x, y = math.cos(angle), math.sin(angle)
        box(f"Habitat module {index}", (x * 2.05, y * 2.05, 0), (0.55, 0.42, 0.66),
            plating, angle + math.pi / 2, bevel=False)
        for row in [-0.1, 0.12]:
            box(f"Module windows {index} {row}", (x * 2.33, y * 2.33, row), (0.04, 0.34, 0.05),
                cyan, angle, bevel=False)
    # A slower counter-ring below the main torus, braced to the spine.
    ring("Counter ring", 1.3, 0.12, -0.62, plating)
    for index in range(6):
        angle = index * math.tau / 6
        x, y = math.cos(angle), math.sin(angle)
        box(f"Counter spoke {index}", (x * 0.85, y * 0.85, -0.62), (0.95, 0.09, 0.09), dark,
            angle, bevel=False)
    # Reactor under the spine with radiator fins and a glowing core band.
    cylinder("Reactor housing", 0.5, 0.6, -1.15, hull)
    cylinder("Reactor core band", 0.52, 0.06, -1.0, amber, bevel=False)
    cylinder("Reactor cone", 0.28, 0.4, -1.6, dark, vertices=24, bevel=False)
    for index in range(4):
        angle = index * math.tau / 4 + math.pi / 4
        x, y = math.cos(angle), math.sin(angle)
        box(f"Radiator fin {index}", (x * 0.95, y * 0.95, -1.2), (0.9, 0.04, 0.55), plating,
            angle, bevel=False)
    # Three docking arms on the upper deck, each ending in a lit collar.
    for index in range(3):
        angle = index * math.tau / 3 + math.pi / 6
        x, y = math.cos(angle), math.sin(angle)
        box(f"Docking arm {index}", (x * 0.95, y * 0.95, 0.85), (0.8, 0.14, 0.14), dark, angle,
            bevel=False)
        cylinder(f"Docking collar {index}", 0.17, 0.2, 0, hull, location=(x * 1.38, y * 1.38, 0.85),
            vertices=16, bevel=False, rotation=(0, math.pi / 2, angle))
        cylinder(f"Collar light {index}", 0.18, 0.04, 0, cyan, location=(x * 1.5, y * 1.5, 0.85),
            vertices=16, bevel=False, rotation=(0, math.pi / 2, angle))
    # Antenna mast and dish beside the dock mouth.
    box("Antenna mast", (0.42, -0.2, 1.55), (0.05, 0.05, 0.7), plating, bevel=False)
    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=6, radius=0.22,
        location=(0.42, -0.2, 1.95))
    dish = bpy.context.object
    dish.scale = (1, 1, 0.35)
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    finish(dish, "Antenna dish", hull, bevel=False)
    box("Antenna tip", (0.42, -0.2, 2.12), (0.06, 0.06, 0.06), red, bevel=False)
    # Hull detail around the command deck.
    for index in range(16):
        angle = index * math.tau / 16
        x, y = math.cos(angle), math.sin(angle)
        box(f"Deck greeble {index}", (x * 0.86, y * 0.86, 0.62 + 0.05 * (index % 3)),
            (0.12, 0.2, 0.08 + 0.04 * (index % 2)), dark if index % 2 else plating,
            angle, bevel=False)
    # Truss bracing along the solar booms and navigation lights at the tips.
    for side in [-1, 1]:
        for step in range(6):
            x = side * (1.95 + step * 0.4)
            box(f"Boom truss {side} {step}", (x, 0, -0.35), (0.04, 0.42, 0.04), dark,
                tilt=(0.0, 0.7 * (1 if step % 2 else -1)), bevel=False)
        box(f"Nav light {side}", (side * 4.72, 0, -0.35), (0.09, 0.09, 0.09),
            green if side > 0 else red, bevel=False)


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
    red = material("Port navigation red", (1.0, 0.08, 0.05), emission=4)
    green = material("Starboard navigation green", (0.2, 1.0, 0.25), emission=4)
    plating = material("Weathered plating", (0.34, 0.4, 0.45), 0.65)

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
            # An open frame (edge bars and mullions, no backing slab): a
            # slab under the cells sorts badly in the menu's flat 3D renderer.
            cx, cy = side * 3.8, bank * 0.85
            for dy in [-0.7, 0.7]:
                box(f"Panel rail {side} {bank} {dy}", (cx, cy + dy, -0.3), (1.7, 0.06, 0.08),
                    dark, bevel=False)
            for dx in [-0.82, -0.25, 0.25, 0.82]:
                box(f"Panel mullion {side} {bank} {dx}", (cx + dx, cy, -0.3), (0.05, 1.45, 0.08),
                    dark, bevel=False)
            for row in range(4):
                for column in range(3):
                    box("Solar cell", (side * 3.8 + (column - 1) * 0.5,
                        bank * 0.85 + (row - 1.5) * 0.32, -0.27), (0.46, 0.28, 0.035), solar,
                        bevel=False)
    details(hull, dark, cyan, amber, red, green, plating)

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
