"""Shared parts, staging and export for the script-built ship hulls.

Conventions for every hull: Blender units, +Z up, bow along +Y. The game
recentres and rescales each GLB, so only proportions matter.

Budget: the client draws every triangle flat-shaded with egui and sorts faces far
to near without a depth buffer, so parts are chunky, closed and touching, bevels
are single chamfers, and each hull stays under 7,000 triangles. Materials with
emission (engines, lamps) are drawn at full brightness.
"""

import math
from pathlib import Path

import bpy
from mathutils import Vector

ROOT = Path(__file__).resolve().parents[2]
QUARTER = math.pi / 2  # Turns a primitive's +Z axis onto the bow's +Y.
TRIANGLE_BUDGET = 7000


def material(name, color, metallic=0.0, roughness=0.55, emission=0.0):
    """A Principled material; `color` is linear RGB, also the viewport colour."""
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
    """A prism along +Z; `rotation=(-QUARTER, 0, 0)` lays it along the bow axis."""
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation
    )
    return finish(bpy.context.object, name, surface)


def cone(name, location, radius, depth, surface, vertices=6, rotation=(-QUARTER, 0, 0),
         tip=0.02):
    """A cone whose tip points along the bow (+Y) unless `rotation` says otherwise."""
    bpy.ops.mesh.primitive_cone_add(
        vertices=vertices, radius1=radius, radius2=tip, depth=depth,
        location=location, rotation=rotation,
    )
    return finish(bpy.context.object, name, surface)


def shell(name, vertices, faces, surface, bevel=0.0):
    """A custom closed mesh from explicit vertices and faces (outward winding)."""
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    return finish(obj, name, surface, bevel)


def loft(name, sections, surface, bevel=0.0):
    """A closed hull lofted through cross-sections ordered stern to bow.

    Each section is (y, half_width, bottom_z, top_z, chine), a flat-bottomed
    hexagon whose sides bulge out by `chine` at mid-height. Ends are capped.
    """
    vertices = []
    for y, half, low, high, chine in sections:
        mid = (low + high) / 2
        vertices += [
            (-half, y, low), (half, y, low), (half + chine, y, mid),
            (half, y, high), (-half, y, high), (-half - chine, y, mid),
        ]
    faces = [(5, 4, 3, 2, 1, 0)]
    for index in range(len(sections) - 1):
        a, b = 6 * index, 6 * (index + 1)
        for side in range(6):
            nxt = (side + 1) % 6
            faces.append((a + side, a + nxt, b + nxt, b + side))
    last = 6 * (len(sections) - 1)
    faces.append(tuple(last + side for side in range(6)))
    return shell(name, vertices, faces, surface, bevel)


def mirrored(build_side):
    """Calls `build_side(side)` for port (-1) and starboard (+1)."""
    for side in (-1, 1):
        build_side(side)


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


def stage(target, preview_scale, preview_path):
    """Three-quarter orthographic camera, warm key and cool rim, EEVEE render."""
    target = Vector(target)
    camera_data = bpy.data.cameras.new("Preview camera")
    camera = bpy.data.objects.new("Preview camera", camera_data)
    bpy.context.collection.objects.link(camera)
    camera.location = (target[0] + 8.0, target[1] + 8.5, target[2] + 7.0)
    camera.rotation_euler = (target - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = preview_scale
    scene = bpy.context.scene
    scene.camera = camera
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
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    if scene.world is None:
        scene.world = bpy.data.worlds.new("Space")
    scene.world.color = (0.10, 0.12, 0.16)
    set_render(scene, 768, preview_scale, preview_path)


def set_render(scene, pixels, ortho_scale, path):
    scene.render.resolution_x = scene.render.resolution_y = pixels
    scene.camera.data.ortho_scale = ortho_scale
    scene.render.filepath = str(path)


def publish(name, preview_scale, sprite_scale):
    """Saves the .blend, renders preview and 256px sprite, exports the GLB.

    Returns the written paths and triangle count; fails over budget.
    """
    source = ROOT / f"assets/source/{name}.blend"
    model = ROOT / f"assets/models/{name}.glb"
    preview = ROOT / f"assets/source/{name}-preview.png"
    sprite = ROOT / f"assets/sprites/{name}.png"
    scene = bpy.context.scene
    triangles = triangle_count()
    if triangles > TRIANGLE_BUDGET:
        raise RuntimeError(f"{name} has {triangles} triangles, over {TRIANGLE_BUDGET}")
    bpy.ops.wm.save_as_mainfile(filepath=str(source))
    set_render(scene, 768, preview_scale, preview)
    bpy.ops.render.render(write_still=True)
    set_render(scene, 256, sprite_scale, sprite)
    bpy.ops.render.render(write_still=True)
    set_render(scene, 768, preview_scale, preview)
    for obj in scene.objects:
        obj.select_set(obj.type == "MESH")
    bpy.ops.export_scene.gltf(
        filepath=str(model), export_format="GLB", use_selection=True,
        export_apply=True, export_yup=False, export_materials="EXPORT",
        export_cameras=False, export_lights=False, export_animations=False,
    )
    return {
        "source": str(source), "model": str(model), "preview": str(preview),
        "sprite": str(sprite), "triangles": triangles,
    }
