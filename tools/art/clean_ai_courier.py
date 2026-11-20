"""Build the AI courier (the scout's art) from the remeshed TRELLIS.2 scan.

Step 1 (ComfyUI's Python): tools/comfyui/remesh_scan.py raw.glb -> courier.npz
Step 2 (Blender is authoring-only):
  Blender -b -P tools/art/clean_ai_courier.py -- <courier.npz>

The .npz holds glTF Y-up vertices in the [-0.5, 0.5] cube, faces and per-vertex
colors. Output follows the hull_kit.py conventions: Z up, nose toward +Y,
SHIP_LENGTH units long, centred on the origin. Writes assets/source/ai-courier.blend
and renders assets/sprites/scout.png with the same camera and lights as the other
ship sprites.
"""
from pathlib import Path
import math
import sys

import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_trade_port import light

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets/source"
SHIP_LENGTH = 4.3            # Blender units, matches the hand-built ships
SPRITE_RESOLUTION = 512
SPRITE_SCALE = 5.2
SMOOTH_ANGLE = math.radians(35)


def load_scan(path):
    scan = np.load(path)
    gltf = scan["vertices"].astype(np.float64)
    # glTF Y-up -> Blender Z-up, the same conversion Blender's glTF importer applies.
    vertices = np.stack([gltf[:, 0], -gltf[:, 2], gltf[:, 1]], axis=1)
    vertices = align_heading(vertices)
    vertices = level_attitude(vertices)
    vertices -= (vertices.min(0) + vertices.max(0)) / 2
    vertices *= SHIP_LENGTH / np.ptp(vertices[:, 1])
    return vertices, scan["faces"], scan["colors"]


def align_heading(vertices):
    """Yaw the hull onto +Y. The scan keeps the reference image's camera yaw, so
    the long (principal) axis of the vertex cloud in the XY plane is rotated onto Y,
    and the narrower end (the nose, not the paired nacelles) is put at +Y."""
    plane = vertices[:, :2] - vertices[:, :2].mean(0)
    eigenvalues, eigenvectors = np.linalg.eigh(np.cov(plane.T))
    long_axis = eigenvectors[:, np.argmax(eigenvalues)]
    angle = math.atan2(long_axis[0], long_axis[1])   # rotation that maps long_axis onto +Y
    cos, sin = math.cos(angle), math.sin(angle)
    x, y = vertices[:, 0], vertices[:, 1]
    vertices = np.stack([cos * x - sin * y, sin * x + cos * y, vertices[:, 2]], axis=1)
    y = vertices[:, 1]
    span = np.ptp(y)
    front = vertices[y > y.max() - 0.15 * span]
    back = vertices[y < y.min() + 0.15 * span]
    if np.ptp(front[:, 0]) > np.ptp(back[:, 0]):   # wider end is the engine pair
        vertices[:, :2] *= -1                       # turn 180 degrees about Z
    return vertices


def level_attitude(vertices):
    """Remove the scan camera's pitch and roll after its heading is aligned."""
    for _ in range(3):
        centred = vertices - vertices.mean(0)
        slope_y = np.dot(centred[:, 1], centred[:, 2]) / np.dot(centred[:, 1], centred[:, 1])
        pitch = math.atan(-slope_y)
        y, z = vertices[:, 1].copy(), vertices[:, 2].copy()
        vertices[:, 1] = math.cos(pitch) * y - math.sin(pitch) * z
        vertices[:, 2] = math.sin(pitch) * y + math.cos(pitch) * z
        centred = vertices - vertices.mean(0)
        slope_x = np.dot(centred[:, 0], centred[:, 2]) / np.dot(centred[:, 0], centred[:, 0])
        roll = math.atan(slope_x)
        x, z = vertices[:, 0].copy(), vertices[:, 2].copy()
        vertices[:, 0] = math.cos(roll) * x + math.sin(roll) * z
        vertices[:, 2] = -math.sin(roll) * x + math.cos(roll) * z
    return vertices


def build_mesh(vertices, faces, colors):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    mesh = bpy.data.meshes.new("AI courier")
    mesh.from_pydata(vertices.tolist(), [], faces.tolist())
    mesh.validate()
    if len(colors) == len(vertices):
        rgba = np.ones((len(colors), 4), np.float32)
        rgba[:, :colors.shape[1]] = colors[:, :4]
        attribute = mesh.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
        attribute.data.foreach_set("color", rgba.ravel())
    ship = bpy.data.objects.new("AI courier", mesh)
    bpy.context.collection.objects.link(ship)
    bpy.context.view_layer.objects.active = ship
    ship.select_set(True)
    return ship


def shade(ship):
    bpy.ops.object.shade_smooth_by_angle(angle=SMOOTH_ANGLE)
    surface = bpy.data.materials.new("Scanned paint")
    surface.use_nodes = True
    nodes = surface.node_tree.nodes
    bsdf = nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = 0.55
    bsdf.inputs["Metallic"].default_value = 0.25
    if ship.data.color_attributes:
        color = nodes.new("ShaderNodeVertexColor")
        color.layer_name = ship.data.color_attributes[0].name
        surface.node_tree.links.new(color.outputs["Color"], bsdf.inputs["Base Color"])
    ship.data.materials.append(surface)


def setup_sprite_scene():
    """Same render settings, lights and camera as the original hand-built ship sprites."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 32
    scene.cycles.use_denoising = True
    scene.render.resolution_x = scene.render.resolution_y = SPRITE_RESOLUTION
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = True
    scene.render.image_settings.file_format = "PNG"
    scene.render.image_settings.color_mode = "RGBA"
    scene.world = bpy.data.worlds.new("World")
    scene.world.color = (0.08, 0.08, 0.08)
    light("Key", (-3, 2, 8), 1400, (1, 0.88, 0.7), 5)
    light("Rim", (4, -4, 4), 1600, (0.3, 0.65, 1), 4)
    bpy.ops.object.camera_add(location=(6, 8, 7))
    camera = bpy.context.object
    camera.rotation_euler = (-camera.location).to_track_quat("-Z", "Y").to_euler()
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = SPRITE_SCALE
    scene.camera = camera
    scene.render.filepath = "//../sprites/scout.png"


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if not args:
        raise SystemExit("usage: Blender -b -P clean_ai_courier.py -- <courier.npz>")
    ship = build_mesh(*load_scan(args[0]))
    shade(ship)
    print(f"clean: {len(ship.data.polygons)} faces, {len(ship.data.vertices)} vertices, "
          f"dims {tuple(round(d, 2) for d in ship.dimensions)}")
    setup_sprite_scene()
    bpy.ops.wm.save_as_mainfile(filepath=str(SOURCE / "ai-courier.blend"))
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    main()
