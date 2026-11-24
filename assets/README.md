# Original Blender artwork

Editable `.blend` scenes are in `source/`, with transparent baked PNGs in
`sprites/`. No downloaded textures or external asset packs are used. The one
exception is the interface fonts in `fonts/` (SIL Open Font License, see below).

| Assets | Purpose | Runtime sprite size |
| --- | --- | --- |
| yellow/red dwarfs, blue/red giants, white dwarf | Local star surfaces; procedural halos layered behind | 256 × 256 |
| rocky (temperate), arid and frozen planets, gas giant | Planet surfaces; rocky planets show their biome | 256 × 256 |
| moon, asteroid | Cratered satellite and irregular prospecting destination | 256 × 256 |
| trade port | Habitat ring and modules, counter-ring, reactor and radiators, docking arms, antenna, solar arrays, navigation and docking lights | 512 × 512 |

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

`icons/*.svg` holds 72 original command, status, commodity and planet-trait
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
`source/app-icon.blend`: a ringed world on a gunmetal plate with a neon orbit,
and the game's freighter (imported from `models/freighter.glb`, so the icon
follows hull rebuilds) flying along it. Re-render after changing the hull or
the celestial materials:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_app_icon.py
sips -z 256 256 assets/app-icon/app-icon.png --out assets/app-icon/app-icon-256.png
```

The 256 px PNG is embedded as the window icon and the game chooser's logo. The
window icon covers the taskbar and title bar on Windows and X11; Wayland and
macOS ignore runtime window icons (a runtime macOS Dock icon would need unsafe
AppKit calls, which the workspace forbids). On macOS,
`tools/package_macos.sh` builds an unsigned `target/StarTrader.app` with an
`.icns` file so the Dock and Finder show the icon. It stores saves and settings
under `~/Library/Application Support/StarTrader`.

## Ship hulls

`source/freighter.blend` and `source/patrol.blend` are original editable models,
each built by its own script on the shared part kit `tools/art/hull_kit.py`
(conventions, 7,000-triangle budget, staging and export). One run writes the
`.blend`, the battle model `models/<name>.glb`, the 512px map sprite
`sprites/<name>.png` and a review render `source/<name>-preview.png`:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_freighter.py   # or build_patrol.py
```

The freighter is a bulk container hauler: a yard of about 85 small ribbed
containers (eight rows, four columns, up to three high, mostly orange with ivory,
teal, rust and slate) under hazard-yellow loading gantries, a lofted prow with a
deliberately tiny bridge and lit windows so the hull reads as huge, and four
blue-glow engines between radiator wings. Cargo roof latches and prow service
hatches add visible surface detail (5,592 triangles).
The patrol is an armoured gunship: a lofted fuselage with cheek armour, intakes
and lit windows, swept wings with layered armour plates, under-wing missile racks
and banded wingtip cannon pods, a dorsal twin-gun turret with a sensor mast, chin
guns, canted lit tail fins and banded twin engines. Cooling slots, armour
fasteners, canopy frames and heat ribs bring it to 4,336 triangles. The scout uses the AI
courier: `source/ai-courier.blend` is a vertex-colored model generated locally
from a concept image with ComfyUI/TRELLIS.2, then remeshed and decimated.
`tools/art/clean_ai_courier.py` rebuilds it from the scan and levels the scan's
pitch and roll. `tools/art/repair_scout_orientation.py` applies the same
correction to the checked-in source when the raw scan is unavailable.
`tools/art/export_battle_models.py -- scout` renders the 512px PNG from the
original vertex colours, then paints and exports the battle GLB.
`tools/comfyui/README.md` covers the full pipeline.

### Battle models

`models/*.glb` are runtime glTF exports of the low-poly hull sources for the
battle view and moving NPC markers. The script-built hulls (freighter, patrol,
prospector) export themselves; `models/scout.glb` is the AI courier
collapse-decimated on export from 60,000 to 6,000 triangles.
Its nearly uniform scan paint is restyled on the export copy with an ivory hull,
teal wings and nose markings, a dark canopy and blue engine housings. The loader
gives each face the mean of its exported vertex colours. The `.blend` source is
not modified.
Re-export the scout with:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/export_battle_models.py
```

The export applies modifiers, keeps Blender's axes (+Z up, nose along +Y) and is
byte-for-byte reproducible. The client embeds the file, recentres the model and
draws it flat-shaded as egui triangles, sorted far to near (no depth buffer yet).
A test checks that each embedded model loads with its coloured parts. In battle,
models sit at their current grid cells and turn toward horizontal movement during
glides; the underlying battle rules are unchanged.

### Prospector

`source/prospector.blend` is an editable original heavy mining rig: a stepped,
twisted 12-sided auger on a hazard-striped bow housing, two articulated digger arms, six
ore hoppers heaped with rock and glowing crystals, a conveyor spine, a gantry
crane, radiator wings and a quad-nozzle amber engine block. Its crew bridge and
lit portholes are deliberately tiny so the hull reads as huge.
`models/prospector.glb` is its colored 3,676-triangle export with auger flutes,
hopper fasteners and crane rivets (engines, floodlights and crystals are emissive),
`sprites/prospector.png` its 512px map
sprite, and `source/prospector-preview.png` a render for visual review.
Battle models may use up to 7,000 triangles each (checked by the model test). The model follows the battle-model conventions (+Z up,
bow along +Y, closed chunky parts for the depth-sorted flat renderer) and the
export is byte-for-byte reproducible. The prospector is the fourth purchasable hull
(`ShipKind::Prospector`); the client embeds both the model and the sprite.
Rebuild all four files with:

```sh
/Applications/Blender.app/Contents/MacOS/Blender --background --factory-startup \
  --python tools/art/build_prospector.py
```

The same script can be run through Blender Lab's MCP server (registered as the
`blender` MCP server): `execute_blender_code_for_cli` in background Blender, or
`execute_blender_code` in an open session once the add-on's socket is running
(it needs Blender's Preferences → System → Network → Allow Online Access). Exec
the file with `__file__` set and call `build()`. The MCP bridge is an authoring
convenience; the runtime GLB has no Blender dependency.

## Music

`music/` holds the game's music: Ogg Vorbis blocks and `manifest.json`, which
the game stitches together into an endless, changing soundtrack that rises
from 60 to 120 BPM and settles back. It is an original Strudel composition:
`source/music/library.strudel.js` holds the instruments and layers, and
`main.strudel.js`, `space.strudel.js`, `disco.strudel.js` and
`night.strudel.js` hold the main track and the space, Space Disco and Night Drift families,
and `links.strudel.js` connects them. Paste the generated
`source/music/music.strudel.js` into strudel.cc to play or edit a demo. Pitched
voices are General MIDI instruments from the MIT-licensed FluidR3_GM soundfont
(local copies in `source/music/soundfonts/`); drums and effects are Strudel
synthesizers. `tools/music/README.md` describes the music and its export
(`node tools/music/render.mjs`), which re-renders only the blocks whose sources
changed (`music/render-cache.json`).

## Trade station model

`models/station.glb` is the trade port in 3D (about 11,000 triangles) for the
main menu, where it turns slowly and ships dock at the mouth on top of its spine.
It and the map sprite come from the same scene: `tools/art/build_trade_port.py`
builds `source/trade-port.blend` and renders `sprites/trade-port.png`, then
`tools/art/export_battle_models.py -- station` exports the GLB. Solar panel
frames are open bars, not slabs, because the menu sorts triangles back to front
and a slab just under the cells would show through them.
