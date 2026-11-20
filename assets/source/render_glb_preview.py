"""Render a GLB preview with Blender: blender -b -P this_file -- input.glb output.png."""

import sys
from pathlib import Path

import bpy
from mathutils import Vector


def script_args():
    if "--" not in sys.argv:
        raise SystemExit("Usage: blender -b -P render_glb_preview.py -- input.glb output.png")
    paths = sys.argv[sys.argv.index("--") + 1 :]
    if len(paths) != 2:
        raise SystemExit("Expected input GLB and output PNG paths")
    return (str(Path(path).resolve()) for path in paths)


def add_light(name, center, offset, extent, energy):
    data = bpy.data.lights.new(name, "AREA")
    data.energy = energy
    data.shape = "DISK"
    data.size = 5
    light = bpy.data.objects.new(name, data)
    bpy.context.collection.objects.link(light)
    light.location = center + Vector(offset).normalized() * extent * 2
    light.rotation_euler = (center - light.location).to_track_quat("-Z", "Y").to_euler()


def main():
    source, output = script_args()
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    bpy.ops.import_scene.gltf(filepath=source)
    meshes = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not meshes:
        raise SystemExit("GLB contains no mesh objects")

    corners = [obj.matrix_world @ Vector(corner) for obj in meshes for corner in obj.bound_box]
    low = Vector(tuple(min(c[i] for c in corners) for i in range(3)))
    high = Vector(tuple(max(c[i] for c in corners) for i in range(3)))
    center = (low + high) / 2
    extent = max(high - low)

    camera_data = bpy.data.cameras.new("Preview Camera")
    camera = bpy.data.objects.new("Preview Camera", camera_data)
    bpy.context.collection.objects.link(camera)
    camera.location = center + Vector((2.2, -3.2, 2.0)).normalized() * extent * 2.6
    camera.rotation_euler = (center - camera.location).to_track_quat("-Z", "Y").to_euler()
    camera_data.type = "ORTHO"
    camera_data.ortho_scale = extent * 1.35
    bpy.context.scene.camera = camera

    add_light("Key", center, (2, -3, 5), extent, 900)
    add_light("Fill", center, (-3, 1, 2), extent, 700)
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 768
    scene.render.resolution_y = 768
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.world.color = (0.12, 0.12, 0.12)
    scene.render.filepath = output
    bpy.ops.render.render(write_still=True)
    print(f"Rendered {output} from {len(meshes)} mesh object(s)")


if __name__ == "__main__":
    main()
