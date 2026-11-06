"""Original Blender materials shared by the celestial export script."""
import bpy


def surface(name, colors, style="rock"):
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    shader = nodes.get("Principled BSDF")
    shader.inputs["Roughness"].default_value = 0.72
    coordinates = nodes.new("ShaderNodeTexCoord")
    noise = nodes.new("ShaderNodeTexNoise")
    noise.inputs["Scale"].default_value = {"ocean": 4, "desert": 7, "ice": 3}.get(style, 13)
    noise.inputs["Detail"].default_value = 5
    links.new(coordinates.outputs["Generated"], noise.inputs["Vector"])
    factor = noise.outputs["Fac"]
    if style == "ice":
        # Stretched noise reads as glacial streaks running toward the poles.
        stretch = nodes.new("ShaderNodeMapping")
        stretch.inputs["Scale"].default_value = (1, 1, 4)
        links.new(coordinates.outputs["Generated"], stretch.inputs["Vector"])
        links.new(stretch.outputs["Vector"], noise.inputs["Vector"])
    if style == "bands":
        wave = nodes.new("ShaderNodeTexWave")
        wave.bands_direction = "Z"
        wave.inputs["Scale"].default_value = 5
        wave.inputs["Distortion"].default_value = 7
        wave.inputs["Detail Scale"].default_value = 2
        links.new(coordinates.outputs["Generated"], wave.inputs["Vector"])
        factor = wave.outputs["Fac"]
    ramp = nodes.new("ShaderNodeValToRGB")
    ramp.color_ramp.elements.remove(ramp.color_ramp.elements[1])
    for index, color in enumerate(colors):
        element = ramp.color_ramp.elements[0] if index == 0 else ramp.color_ramp.elements.new(index / (len(colors) - 1))
        element.color = (*color, 1)
    links.new(factor, ramp.inputs["Fac"])
    links.new(ramp.outputs["Color"], shader.inputs["Base Color"])
    bump = nodes.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = {"ocean": 0.15, "bands": 0.15, "desert": 0.6, "ice": 0.3}.get(style, 0.45)
    bump.inputs["Distance"].default_value = 0.08
    links.new(noise.outputs["Fac"], bump.inputs["Height"])
    links.new(bump.outputs["Normal"], shader.inputs["Normal"])
    if style == "star":
        links.new(ramp.outputs["Color"], shader.inputs["Emission Color"])
        shader.inputs["Emission Strength"].default_value = 1.2
    return material


STAR_COLORS = {
    "yellow-dwarf": [(0.7, 0.17, 0.015), (1.0, 0.65, 0.08), (1.0, 0.95, 0.55)],
    "red-dwarf": [(0.25, 0.008, 0.003), (0.95, 0.08, 0.015), (1.0, 0.42, 0.08)],
    "blue-giant": [(0.03, 0.12, 0.5), (0.12, 0.45, 1.0), (0.65, 0.88, 1.0)],
    "red-giant": [(0.45, 0.025, 0.003), (1.0, 0.22, 0.02), (1.0, 0.65, 0.2)],
    "white-dwarf": [(0.25, 0.45, 0.6), (0.65, 0.85, 1.0), (1.0, 1.0, 1.0)],
}
