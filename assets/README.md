# Original Blender artwork

Editable `.blend` scenes are in `source/`, with transparent baked PNGs in
`sprites/`. No downloaded textures or external asset packs are used.

| Assets | Purpose | Runtime sprite size |
| --- | --- | --- |
| yellow/red dwarfs, blue/red giants, white dwarf | Local star surfaces; procedural halos layered behind | 256 × 256 |
| rocky planet, gas giant | Planet silhouettes/surfaces | 256 × 256 |
| moon, asteroid | Cratered satellite and irregular prospecting destination | 256 × 256 |
| trade port | Habitat ring, cargo bays, solar arrays and docking guidance | 512 × 512 |

Regenerate from the repository root:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_trade_port.py
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_celestials.py
```

On other platforms, replace the executable path with `blender`. The shared
`celestial_materials.py` module creates original noise/band materials in Blender.
The scripts overwrite generated sources/images; preserve hand edits separately
or incorporate them into the authoring scripts before regenerating.

The runtime embeds all ten sprites, decodes them once per egui context and shares
texture handles across system views. Recreated graphics contexts receive new
handles. Their raw RGBA pixels total about 3.25 MiB, excluding fonts and driver
allocation overhead. PNG checks validate transparency/decoding and cache reuse.

This is a hybrid renderer: detailed local surfaces use the baked artwork; tiny
galaxy markers, star halos, route lines and selection effects remain procedural.
The images are static billboards; orbiting the camera changes projected positions,
not the object's lighting or visible face. No animation frame sets are currently
justified. No claim of measured performance improvement is made.

Symbolic sizes retain stars > planets > stations. The local station is a small
marker, with a larger detail illustration when selected. A moon next to its
planet may overlap in projection; use the candidate chooser or destination list.
Blender is needed only for authoring/export, never to run the game.

## Ship hulls

`source/freighter.blend`, `source/scout.blend` and `source/patrol.blend` are
original editable models. `sprites/` contains their transparent 256px exports.
Rebuild using Blender in background mode:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --python tools/art/build_ships.py
```

Freighters have cargo pods, scouts swept wings and patrol ships hardpoints.
These are static ship illustrations; combat equipment is not implemented yet.
