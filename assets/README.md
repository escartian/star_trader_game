# Original Blender artwork

Editable `.blend` scenes are in `source/`, with transparent baked PNGs in
`sprites/`. No downloaded textures or external asset packs are used. The one
exception is the interface fonts in `fonts/` (SIL Open Font License, see below).

| Assets | Purpose | Runtime sprite size |
| --- | --- | --- |
| yellow/red dwarfs, blue/red giants, white dwarf | Local star surfaces; procedural halos layered behind | 256 × 256 |
| rocky (temperate), arid and frozen planets, gas giant | Planet surfaces; rocky planets show their biome | 256 × 256 |
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
Append `-- arid-planet frozen-planet` (any export names) to the celestial command
to rebuild only those sprites. The scripts overwrite generated sources/images; preserve hand edits separately
or incorporate them into the authoring scripts before regenerating.

The runtime embeds all twelve celestial/station sprites, decodes them once per egui context and shares
texture handles across system views. Recreated graphics contexts receive new
handles. Their raw RGBA pixels total about 3.75 MiB, excluding fonts and driver
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

## Interface icons

`icons/*.svg` holds 66 original command, status, commodity and planet-trait
icons. They are authored in Blender as 2D curves (`source/icons.blend`: one
`icon:<name>` Empty per icon, parenting stroke/fill curves). The geometry comes
from `tools/art/icon_shapes.py`. Rebuild the scene and export every SVG:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_icons.py
```

To keep hand edits made in Blender, export from the saved scene without rebuilding:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background assets/source/icons.blend \
  --python tools/art/build_icons.py -- --export-only
```

The SVGs are white on a 20-unit grid. The client embeds them and rasterizes each
icon with `resvg` at its on-screen pixel size (DPI aware, cached per egui
context), then tints it. A test checks that every `Icon` has an SVG that renders.
If you add an icon, regenerate `crates/game-client/src/theme/icons/sources.rs`
and add the variant to `svg::name`. Icon-only controls keep their text as hover
help and accessible names. Danger is shown as a shield plus 1–6 pips, so it does
not rely on color alone.

## Fonts

`fonts/` bundles **Rajdhani** (Indian Type Foundry; Medium and SemiBold) for all
interface text and **Orbitron** (The League of Moveable Type) for display
headings. Both are SIL Open Font License 1.1 (`*-OFL.txt`) and were downloaded
from the google/fonts repository on 2026-11-05. egui's built-in fonts remain
glyph fallbacks.

## App icon

`app-icon/app-icon.png` (1024 px) and `app-icon-256.png` are rendered from
`source/app-icon.blend`:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_app_icon.py
sips -z 256 256 assets/app-icon/app-icon.png --out assets/app-icon/app-icon-256.png
```

The 256 px PNG is embedded as the window icon. That covers the taskbar and title
bar on Windows and X11; Wayland and macOS ignore runtime window icons. On macOS,
`tools/package_macos.sh` builds an unsigned `target/StarTrader.app` with an
`.icns` file so the Dock and Finder show the icon. It stores saves and settings
under `~/Library/Application Support/StarTrader`.

## Ship hulls

`source/freighter.blend` and `source/patrol.blend` are original editable models.
`sprites/` contains their transparent 256px exports. Rebuild using Blender in
background mode:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --python tools/art/build_ships.py
```

Freighters have cargo pods and patrol ships hardpoints. The scout uses the AI
courier: `source/ai-courier.blend` is a vertex-colored model generated locally
from a concept image with ComfyUI/TRELLIS.2, then remeshed and decimated.
`tools/art/clean_ai_courier.py` rebuilds it and renders `sprites/scout.png` with
the same camera and lights; `tools/comfyui/README.md` covers the full pipeline.
These are static ship illustrations; combat equipment is not implemented yet.
