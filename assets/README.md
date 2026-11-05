# Trade-port artwork

Original model created for this project in Blender. No downloaded assets.

- `source/trade-port.blend`: editable model, materials, lighting and camera.
- `sprites/trade-port.png`: 512 × 512 transparent orthographic render.
- `../tools/art/build_trade_port.py`: recreates the scene and sprite from scratch.

Regenerate from the repository root:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_trade_port.py
```

On other platforms, replace the executable path with `blender`.
The script replaces the generated model and image; manual edits to the `.blend`
should be preserved separately or incorporated into the script first.
Blender is only required when changing/rebuilding the artwork. The native client
embeds the PNG and decodes it once per local view, with a fallback marker if loading
fails. The map uses a fixed screen-facing illustration with a symbolic size, not
a physical-scale 3D model. Selecting the station also displays a larger preview.

The design has a titanium habitat ring, central docking spine, orange freight
bays, blue solar arrays, cyan guidance lights and amber approach beacons.

Local-view size hierarchy is symbolic: enlarged procedural star, then gas giant,
rocky planet, and a small station sprite (22 logical points at default zoom).
The 160-point station inspector is a detail illustration, not its map scale.
Stars use egui glow geometry and planets use shaded circles; neither is a Blender
model. Both orbital distances and physical sizes would need separate treatment
for a true astronomical scale, which this navigation map does not claim.
