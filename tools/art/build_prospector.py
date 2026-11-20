"""Build the prospector, an original heavy mining rig, and export its assets.

Writes the editable scene (assets/source/prospector.blend), the runtime model
(assets/models/prospector.glb), the 256px map sprite (assets/sprites/prospector.png)
and a review render (prospector-preview.png).

Axes and units: Blender units, +Z up, the drill (bow) points along +Y. The game
recentres and rescales the GLB, so absolute size only matters relative to its
parts. The bridge is kept deliberately small so the rig reads as huge.

Budget: the client draws every triangle flat-shaded with egui and sorts faces far
to near without a depth buffer, so parts are chunky, closed and touching, bevels
are single chamfers, and the whole rig stays under 7,000 triangles. Materials with
emission (engines, floodlights, ore crystals) are drawn at full brightness.

Run from the repository root:

    blender --background --factory-startup --python tools/art/build_prospector.py

or through Blender Lab's MCP by exec-ing this file with ``__file__`` set and
calling ``build()``.
"""

import math
from pathlib import Path

import bpy

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets/source/prospector.blend"
MODEL = ROOT / "assets/models/prospector.glb"
PREVIEW = ROOT / "assets/source/prospector-preview.png"
SPRITE = ROOT / "assets/sprites/prospector.png"

QUARTER = math.pi / 2  # Turns a primitive's +Z axis onto the bow's +Y.
DRILL_SIDES = 12  # Facets on the auger and its housing; reads as round in battle.


def material(name, color, metallic=0.0, roughness=0.55, emission=0.0):
    item = bpy.data.materials.new(name)
    item.diffuse_color = (*color, 1.0)
    item.use_nodes = True
    shader = item.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = (*color, 1.0)
    shader.inputs["Metallic"].default_value = metallic
    shader.inputs["Roughness"].default_value = roughness
    if emission:
        shader.inputs["Emission Color"].default_value = (*color, 1.0)
        shader.inputs["Emission Strength"].default_value = emission
    return item


def palette():
    """Linear colours: hazard-yellow paint over gunmetal frames and worn steel."""
    return {
        "paint": material("Hazard yellow", (0.85, 0.52, 0.05), 0.3, 0.5),
        "frame": material("Gunmetal frame", (0.11, 0.12, 0.14), 0.6, 0.45),
        "steel": material("Worn drill steel", (0.48, 0.50, 0.52), 0.85, 0.35),
        "tempered": material("Tempered steel", (0.25, 0.26, 0.28), 0.85, 0.4),
        "stripe": material("Hazard black", (0.03, 0.03, 0.035), 0.2, 0.6),
        "ore": material("Raw ore", (0.36, 0.18, 0.09), 0.0, 0.9),
        "dust": material("Ore dust", (0.07, 0.045, 0.03), 0.0, 1.0),
        "crystal": material("Mineral glow", (0.15, 0.85, 0.72), emission=2.0),
        "glass": material("Bridge glass", (0.02, 0.18, 0.24), 0.3, 0.15),
        "engine": material("Engine amber", (1.0, 0.50, 0.12), emission=3.0),
        "lamp": material("Floodlight", (1.0, 0.94, 0.78), emission=2.0),
    }


def finish(obj, name, surface, bevel=0.0):
    obj.name = name
    obj.data.materials.append(surface)
    if bevel:
        edge = obj.modifiers.new("Chamfer", "BEVEL")
        edge.width = bevel
        edge.segments = 1
        obj.modifiers.new("Weighted normals", "WEIGHTED_NORMAL")
    return obj


def box(name, location, size, surface, bevel=0.0, turn=0.0):
    """An axis-aligned block, optionally yawed by `turn` radians about +Z."""
    bpy.ops.mesh.primitive_cube_add(size=1, location=location)
    obj = bpy.context.object
    obj.dimensions = size
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    obj.rotation_euler.z = turn
    return finish(obj, name, surface, bevel)


def cylinder(name, location, radius, depth, surface, vertices=8, rotation=(0, 0, 0)):
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, surface)


def cone(name, location, radius, depth, surface, vertices=6, rotation=(-QUARTER, 0, 0)):
    """A cone whose tip points along the bow (+Y) unless `rotation` says otherwise."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices, radius1=radius, radius2=0.02, depth=depth,
        location=location, rotation=rotation,
    )
    return finish(bpy.context.object, name, surface)


def rock(name, location, radius, surface, squash=0.6, yaw=0.0):
    bpy.ops.mesh.primitive_ico_sphere_add(subdivisions=1, radius=radius, location=location)
    obj = bpy.context.object
    obj.scale = (1.0, 0.85, squash)
    obj.rotation_euler = (0.3, 0.2, yaw)
    bpy.ops.object.transform_apply(location=False, rotation=True, scale=True)
    return finish(obj, name, surface)


def build_keel(m):
    """Spine, belly and the conveyor that carries ore from the drill aft."""
    box("Keel spine", (0, -0.4, 0), (1.2, 4.4, 0.6), m["frame"], 0.06)
    box("Ventral keel", (0, -0.5, -0.42), (1.5, 3.6, 0.35), m["frame"], 0.06)
    box("Ore conveyor", (0, -0.25, 0.32), (0.55, 3.4, 0.05), m["stripe"])
    for index in range(9):
        box("Conveyor roller", (0, 1.3 - 0.4 * index, 0.37), (0.66, 0.07, 0.05), m["steel"])


def build_drill(m):
    """The bow auger: a stepped, twisted bit on a hazard-striped housing."""
    box("Drill neck", (0, 1.95, 0), (1.0, 0.5, 0.75), m["frame"], 0.05)
    cylinder("Drill housing", (0, 2.35, 0), 0.85, 0.35, m["paint"], DRILL_SIDES, (-QUARTER, 0, 0))
    cylinder("Housing hazard band", (0, 2.58, 0), 0.9, 0.12, m["stripe"], DRILL_SIDES,
             (-QUARTER, 0, 0))
    for index, radius in enumerate((0.7, 0.58, 0.46, 0.34, 0.22)):
        cylinder(
            "Auger step", (0, 2.78 + 0.28 * index, 0), radius, 0.28,
            m["steel"] if index % 2 == 0 else m["tempered"], DRILL_SIDES,
            (-QUARTER, 0.22 * index, 0),  # Each step twists about the bow axis.
        )
    cone("Auger tip", (0, 4.27, 0), 0.16, 0.42, m["steel"], DRILL_SIDES)
    for index in range(6):
        angle = index * math.tau / 6 + math.pi / 6
        spot = (0.8 * math.cos(angle), 2.75, 0.8 * math.sin(angle))
        cone("Cutter tooth", spot, 0.07, 0.26, m["steel"], 4)


def build_arm(m, side):
    """A two-segment digger arm on the bow shoulder, ending in a small drill.

    `side` is +1 for starboard (+X) and -1 for port. A yaw of `turn` radians
    swings the segment's +Y end toward -X, so outward is -side * angle.
    """
    shoulder = (side * 0.75, 1.4)
    cylinder("Arm shoulder", (*shoulder, 0.05), 0.2, 0.34, m["frame"], 8, (0, QUARTER, 0))
    out = 0.55
    reach = 1.1
    elbow = (shoulder[0] + side * math.sin(out) * reach, shoulder[1] + math.cos(out) * reach)
    box(
        "Arm boom", ((shoulder[0] + elbow[0]) / 2, (shoulder[1] + elbow[1]) / 2, 0.05),
        (0.24, reach, 0.24), m["paint"], 0.03, -side * out,
    )
    cylinder("Arm elbow", (*elbow, 0.05), 0.17, 0.32, m["frame"], 8)
    inward = 0.12
    forearm = 1.0
    wrist = (elbow[0] - side * math.sin(inward) * forearm, elbow[1] + math.cos(inward) * forearm)
    box(
        "Arm forearm", ((elbow[0] + wrist[0]) / 2, (elbow[1] + wrist[1]) / 2, 0.05),
        (0.2, forearm, 0.2), m["paint"], 0.03, side * inward,
    )
    box("Forearm hazard band", ((elbow[0] + wrist[0]) / 2, (elbow[1] + wrist[1]) / 2, 0.05),
        (0.22, 0.14, 0.22), m["stripe"], 0.0, side * inward)
    tip = (wrist[0] - side * math.sin(inward) * 0.22, wrist[1] + math.cos(inward) * 0.22)
    cone("Arm drill", (*tip, 0.05), 0.17, 0.44, m["steel"], 6, (-QUARTER, 0, side * inward))


def build_hoppers(m):
    """Six ore hoppers flanking the spine, heaped with rock and a few crystals."""
    heaps = ((-0.18, 0.2, 0.42), (0.17, -0.06, 0.4), (-0.02, -0.28, 0.39))
    for side in (-1, 1):
        for row, y in enumerate((0.75, -0.35, -1.45)):
            x = side * 1.05
            box("Ore hopper", (x, y, 0.0), (0.95, 1.0, 0.6), m["paint"], 0.04)
            box("Ore bed", (x, y, 0.31), (0.8, 0.85, 0.04), m["dust"])
            for index, (dx, dy, z) in enumerate(heaps):
                rock("Ore lump", (x + side * dx, y + dy, z), 0.22 - 0.03 * index, m["ore"],
                     yaw=row + index + side)
            if (row + (side > 0)) % 2 == 0:
                cone("Ore crystal", (x - side * 0.24, y + 0.26, 0.5), 0.09, 0.4,
                     m["crystal"], 4, (0.25 * side, 0.2, 0))
        box("Hopper rail", (side * 1.58, -0.35, 0.22), (0.1, 3.3, 0.1), m["frame"])
        box("Hopper rail", (side * 1.58, -0.35, -0.22), (0.1, 3.3, 0.1), m["frame"])
        box("Floodlight", (side * 1.05, 1.27, 0.15), (0.3, 0.04, 0.1), m["lamp"])


def build_portholes(m):
    """Rows of tiny lit crew windows on the exposed hull sides, for scale."""
    rows = (
        (0.505, (1.85, 2.0, 2.15), 0.12),  # Drill neck.
        (0.605, (1.4, 1.55), 0.12),  # Spine ahead of the hoppers.
        (0.955, (-3.2, -3.05, -2.9, -2.75), 0.2),  # Engine block.
    )
    for x, ys, z in rows:
        for side in (-1, 1):
            for y in ys:
                box("Porthole", (side * x, y, z), (0.03, 0.07, 0.05), m["lamp"])


def build_crane(m):
    """A gantry crane straddling the hoppers, with a trolley and hook block."""
    for side in (-1, 1):
        box("Crane post", (side * 1.62, -0.9, 0.55), (0.14, 0.14, 1.1), m["frame"])
    box("Crane beam", (0, -0.9, 1.12), (3.45, 0.18, 0.16), m["paint"], 0.02)
    box("Crane trolley", (0.7, -0.9, 0.99), (0.3, 0.3, 0.14), m["stripe"])
    box("Crane cable", (0.7, -0.9, 0.8), (0.03, 0.03, 0.3), m["steel"])
    box("Hook block", (0.7, -0.9, 0.6), (0.16, 0.16, 0.12), m["steel"])


def build_bridge(m):
    """A small crew bridge on a pylon aft; its scale sells the rig's size."""
    box("Bridge pylon", (0, -2.25, 0.52), (0.4, 0.4, 0.45), m["frame"], 0.03)
    box("Bridge cabin", (0, -2.18, 0.86), (0.7, 0.42, 0.24), m["paint"], 0.03)
    box("Bridge windows", (0, -1.97, 0.88), (0.6, 0.05, 0.08), m["glass"])
    cylinder("Antenna", (0.32, -2.3, 1.22), 0.025, 0.42, m["steel"], 4)


def build_engines(m):
    """A quad-nozzle engine block with radiator wings."""
    box("Engine block", (0, -2.95, 0), (1.9, 0.8, 0.95), m["frame"], 0.06)
    for x in (-0.48, 0.48):
        for z in (-0.24, 0.24):
            cylinder("Engine nozzle", (x, -3.48, z), 0.22, 0.3, m["steel"], 8, (QUARTER, 0, 0))
            cylinder("Engine glow", (x, -3.64, z), 0.17, 0.02, m["engine"], 8, (QUARTER, 0, 0))
    for side in (-1, 1):
        box("Radiator wing", (side * 1.45, -2.9, 0.25), (1.0, 0.8, 0.05), m["steel"], 0.02)
        for index in range(3):
            box("Radiator fin", (side * 1.45, -3.15 + 0.25 * index, 0.29),
                (0.9, 0.05, 0.03), m["stripe"])


def add_camera_and_lights(target):
    camera_data = bpy.data.cameras.new("Preview camera")
    camera = bpy.data.objects.new("Preview camera", camera_data)
    bpy.context.collection.objects.link(camera)
    camera.location = (target[0] + 8.0, target[1] + 8.5, target[2] + 7.0)
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = 8.4
    bpy.context.scene.camera = camera
    for name, offset, energy, color in [
        ("Warm key", (-4, 5, 8), 1400, (1.0, 0.86, 0.70)),
        ("Cool rim", (5, -6, 5), 1600, (0.55, 0.8, 1.0)),
    ]:
        lamp = bpy.data.lights.new(name, "AREA")
        lamp.energy = energy
        lamp.shape = "DISK"
        lamp.size = 6
        lamp.color = color
        obj = bpy.data.objects.new(name, lamp)
        bpy.context.collection.objects.link(obj)
        obj.location = offset
        obj.rotation_euler = (target - obj.location).to_track_quat("-Z", "Y").to_euler()


def clear_scene():
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.data.orphans_purge(do_recursive=True)


def triangle_count():
    depsgraph = bpy.context.evaluated_depsgraph_get()
    total = 0
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            mesh = obj.evaluated_get(depsgraph).to_mesh()
            total += sum(len(polygon.vertices) - 2 for polygon in mesh.polygons)
            obj.evaluated_get(depsgraph).to_mesh_clear()
    return total


def configure_render(scene):
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 768
    scene.render.resolution_y = 768
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.render.filepath = str(PREVIEW)
    if scene.world is None:
        scene.world = bpy.data.worlds.new("Space")
    scene.world.color = (0.10, 0.12, 0.16)


def render_sprite(scene):
    """The map sprite: the preview's three-quarter view, framed tight at 256px."""
    scene.render.resolution_x = scene.render.resolution_y = 256
    scene.camera.data.ortho_scale = 7.6
    scene.render.filepath = str(SPRITE)
    bpy.ops.render.render(write_still=True)
    scene.render.resolution_x = scene.render.resolution_y = 768
    scene.camera.data.ortho_scale = 8.4
    scene.render.filepath = str(PREVIEW)


def build():
    from mathutils import Vector

    clear_scene()
    parts = palette()
    build_keel(parts)
    build_drill(parts)
    for side in (-1, 1):
        build_arm(parts, side)
    build_hoppers(parts)
    build_portholes(parts)
    build_crane(parts)
    build_bridge(parts)
    build_engines(parts)

    add_camera_and_lights(Vector((0, 0.5, 0.2)))
    scene = bpy.context.scene
    configure_render(scene)
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE))
    bpy.ops.render.render(write_still=True)
    render_sprite(scene)
    for obj in scene.objects:
        obj.select_set(obj.type == "MESH")
    bpy.ops.export_scene.gltf(
        filepath=str(MODEL), export_format="GLB", use_selection=True,
        export_apply=True, export_yup=False, export_materials="EXPORT",
        export_cameras=False, export_lights=False, export_animations=False,
    )
    return {
        "source": str(SOURCE), "model": str(MODEL), "preview": str(PREVIEW),
        "sprite": str(SPRITE),
        "triangles": triangle_count(),
    }


if __name__ == "__main__":
    print(build())
