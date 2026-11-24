# Flat space map

## Starting presentation

Use an orthographic camera with a fixed tilt over the `y = 0` plane. Stars are
screen-size markers projected from authoritative X/Z coordinates, with labels
drawn in screen space.
The map is an interactive instrument for choosing journeys, not a piloting view.
No meshes, realistic lighting or planetary surfaces are required.

Keep market/fleet panels fixed while the map camera moves. Prioritize the selected
system and proposed route; reduce unrelated labels before reducing readability.

## Controls

| Action | Desktop starting binding | Touch starting binding |
| --- | --- | --- |
| Select | Primary click | Tap |
| Inspect overlap | Candidate list | Candidate list |
| Pan | Drag | One-finger drag |
| Zoom | Wheel/trackpad | Pinch |
| Focus selection | Button / F | Button |
| Find a star or planet | Search button or Ctrl/Cmd+F | Search button |
| Reset/overview | Visible controls | Visible controls |

The galaxy map's **Search** button (top-left) opens a box that lists up to eight stars and bodies whose
names contain the typed text; picking one selects and centres its system
without plotting a course. Every UI control plays the click cue; map, system
and battle canvases stay silent on empty space and play a brighter selection
cue when a click lands on a star, planet, fleet or battle target.

Treat these as prototype bindings: test gesture conflicts, especially pinch versus
drag and tap versus pan. Never require hover, a keyboard or a middle
mouse button to complete an essential action. UI panels consume input before the map.

## Routes and selection

- Actual plane distance, fuel, estimated time and reachability are shown on selection.
  Screen proximity never substitutes for travel distance.
- Pick within a screen-space radius suited to mouse or touch. If several stars
  overlap, list candidates with name and distance instead of choosing arbitrarily.
- Highlight reachable neighbors and draw the selected journey; use text/symbols as
  well as color to distinguish states.
- Use focus, zoom and the overlap candidate list for dense regions.
- Keep pan and zoom separate from world state.

## Levels of detail

Galaxy view shows systems and interstellar routes. System view shows planets,
fleets and local destinations using its own scale. Use a breadcrumb/back action;
preserve camera state on return. Do not infer simulation travel distances from the
visual spacing used to make a local system readable.

## Prototype acceptance scenarios

1. Two systems with the same projected position remain selectable from a list.
2. Panning and zooming never change travel distance or route cost.
3. A player can select and travel to a destination using touch without hover.
4. Dense labels do not obscure the selected destination or route.
5. Reset recovers the map; entering and leaving a system preserves context.
6. A minimum-size phone viewport keeps labels and selection controls usable.

## Implemented navigation baseline

The galaxy's **Open system** button enters a separately scaled local
map using fixed snapshot destinations. Local rocky planets, gas giants and trade
stations are selectable by marker or list; overlapping candidates are listed
explicitly. **← Galaxy** / Escape returns to the previous galaxy camera, selection
and overlap candidates. Each view exposes pan, focus and zoom.
Local distances use kilometer positions; interstellar distances use galaxy
light-years. Travel and docking use authoritative session commands.

## Celestial artwork and gameplay panels

Local stars and destinations use shared preloaded Blender-baked surfaces:
nine star classes, seven rocky biomes, three gas giant looks, two moon looks,
two asteroid looks and the trade port. Generated worlds vary the star class and
planet biome, including different rocky biomes within one generated system.
Moon and asteroid looks follow their surface profile: frozen moons are ice
moons, metallic asteroids are metallic (see [planet economy](PLANET_ECONOMY.md)). Blue stars
have no cross-shaped marker. Procedural halos and selection remain overlaid;
galaxy markers retain their small geometric
symbols. These are static billboards with symbolic sizes, not live 3D models or
physical-scale radii. The fixed system tilt separates the small port from the
star. Moon/planet overlaps can be resolved with the candidate list.

The ship panel always reports actual ship location, cargo, credits and fuel.
Inspecting or double-clicking a system does not move the ship. Market controls
operate at the ship's current system, independently of the inspected target.
Travel uses an explicit authority-validated command and reports tick progress.
Continuing a saved journey selects its destination for route inspection.

## Moon spacing and rotation

Moons too close to a planet to read as separate markers receive an exaggerated
separation in the system diagram. The nearest planet is used as the presentation
anchor; saved positions and kilometer distances are not changed or migrated.
Drawing, picking, routes and Focus share the displayed positions. Label placement
reserves all body markers, preventing a moon from being hidden by a planet's name.
Projection overlap is still possible when targets share a map position; the chooser remains.

Slow planet rotation could add life, but it is deferred: the current sprites have
lighting baked into a single view. A future surface shader or properly baked
rotation frames should keep illumination consistent; spinning the sprite in 2D
would not. Do not introduce an idle animation loop for this decorative effect now.
