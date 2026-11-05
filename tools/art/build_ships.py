"""Original ship models and transparent map sprites; Blender is authoring-only."""
from pathlib import Path
import sys
import bpy

sys.path.insert(0, str(Path(__file__).resolve().parent))
from build_trade_port import material, box, light

ROOT = Path(__file__).resolve().parents[2]


def hull(name, surface):
    vertices = [(-0.6, -1.8, -0.3), (0.6, -1.8, -0.3), (0.7, 0.9, -0.2),
                (0, 2.4, 0), (-0.7, 0.9, -0.2), (-0.5, -1.6, 0.5),
                (0.5, -1.6, 0.5), (0, 1.6, 0.5)]
    faces = [(0, 1, 2, 3, 4), (5, 7, 6), (0, 5, 6, 1),
             (1, 6, 7, 3, 2), (0, 4, 3, 7, 5)]
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    obj.data.materials.append(surface)
    bevel = obj.modifiers.new("Manufactured edges", "BEVEL")
    bevel.width = 0.1
    bevel.segments = 3
    obj.modifiers.new("Weighted normals", "WEIGHTED_NORMAL")


def build(kind):
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    titanium = material("Titanium hull", (0.32, 0.43, 0.53), 0.6)
    blue = material("Blue panels", (0.035, 0.24, 0.45), 0.45)
    orange = material("Cargo pods", (0.88, 0.32, 0.06), 0.35)
    canopy = material("Canopy", (0.025, 0.16, 0.21), 0.8)
    glow = material("Engine blue", (0.08, 0.6, 1), emission=3)
    hull(kind, blue if kind == "scout" else titanium)
    box("Cockpit", (0, 0.65, 0.48), (0.48, 1.0, 0.2), canopy)
    for side in [-1, 1]:
        box("Engine housing", (side * 0.65, -1.35, 0.1), (0.5, 1.1, 0.5), titanium)
        box("Engine nozzle", (side * 0.65, -1.93, 0.1), (0.32, 0.06, 0.3), glow)
        if kind == "freighter":
            for index in range(3):
                box("Freight container", (side * 1.05, index * 0.8 - 1, 0.05), (0.65, 0.7, 0.6), orange)
        elif kind == "scout":
            box("Swept wing", (side * 1.0, -0.25, -0.05), (1.6, 0.65, 0.13), blue, side * 0.5)
            box("Sensor boom", (side * 0.45, 1.6, 0), (0.1, 1.0, 0.1), titanium)
        else:
            box("Armored wing", (side * 1.0, -0.2, 0), (1.1, 1.3, 0.3), titanium, side * 0.3)
            box("Forward hardpoint", (side * 1.1, 0.9, 0.1), (0.15, 1.3, 0.15), orange)
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
    scene.world.color = (0.08, 0.08, 0.08)
    light("Key", (-3, 2, 8), 1400, (1, 0.88, 0.7), 5)
    light("Rim", (4, -4, 4), 1600, (0.3, 0.65, 1), 4)
    bpy.ops.object.camera_add(location=(6, 8, 7))
    camera = bpy.context.object
    camera.rotation_euler = (-camera.location).to_track_quat('-Z', 'Y').to_euler()
    camera.data.type = "ORTHO"
    camera.data.ortho_scale = 6
    scene.camera = camera
    scene.render.filepath = f"//../sprites/{kind}.png"
    bpy.ops.wm.save_as_mainfile(filepath=str(ROOT / "assets/source" / f"{kind}.blend"))
    bpy.ops.render.render(write_still=True)


if __name__ == "__main__":
    for kind in ["freighter", "scout", "patrol"]:
        build(kind)
