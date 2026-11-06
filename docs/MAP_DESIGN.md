# Viewing a 3D universe on a 2D screen

## Starting presentation

Use an orthographic camera tilted to reveal all three axes. Stars are screen-size
markers projected from true 3D coordinates, with labels drawn in screen space.
The map is an interactive instrument for choosing journeys, not a piloting view.
No meshes, realistic lighting or planetary surfaces are required.

Keep market/fleet panels fixed while the map camera moves. Prioritize the selected
system and proposed route; reduce unrelated labels before reducing readability.

## Controls

| Action | Desktop starting binding | Touch starting binding |
| --- | --- | --- |
| Select | Primary click | Tap |
| Inspect overlap | Candidate popover | Candidate popover |
| Orbit around focus | Secondary drag | One-finger drag after movement threshold |
| Pan | Middle drag, with visible pan tool alternative | Two-finger drag |
| Zoom | Wheel/trackpad | Pinch |
| Focus selection | Button / F | Button |
| Reset/top/front/side | Visible controls | Visible controls |

Treat these as prototype bindings: test gesture conflicts, especially pinch versus
two-finger pan and tap versus orbit. Never require hover, a keyboard or a middle
mouse button to complete an essential action. UI panels consume input before the map.

## Depth, routes and selection

- Actual 3D distance, fuel, estimated time and reachability are shown on selection.
  Screen proximity never substitutes for travel distance.
- A small axis indicator rotates with the camera. Named view presets allow exact
  alignment and a reset recovers from a confusing camera position.
- Pick within a screen-space radius suited to mouse or touch. If several stars
  overlap, list candidates with name and distance instead of choosing arbitrarily.
- Highlight reachable neighbors and draw the selected journey; use text/symbols as
  well as color to distinguish states.
- Use orbit/presets, focus and the overlap candidate list for dense regions.
  Depth slicing is deliberately omitted by user direction.
- Keep rotation around the selected/focused point and pan separate from world state.

## Levels of detail

Galaxy view shows systems and interstellar routes. System view shows planets,
fleets and local destinations using its own scale. Use a breadcrumb/back action;
preserve camera state on return. Do not infer simulation travel distances from the
visual spacing used to make a local system readable.

## Prototype acceptance scenarios

1. Two systems have the same projected position but different depths: both can be selected.
2. Rotating the camera changes projection but never travel distance or route cost.
3. A player can select and travel to a destination using touch without hover.
4. Dense labels do not obscure the selected destination or route.
5. Reset recovers the map; entering and leaving a system preserves context.
6. A minimum-size phone viewport keeps labels and selection controls usable.

## Implemented navigation baseline

The galaxy's **Open system** button enters a separately scaled local
map using fixed snapshot destinations. Local rocky planets, gas giants and trade
stations are selectable by marker or list; overlapping candidates are listed
explicitly. **← Galaxy** / Escape returns to the previous galaxy camera, selection
and overlap candidates. Each view exposes pan, focus, zoom and orientation tools.
Local distances use kilometer positions; interstellar distances use galaxy
light-years. Physical travel and docking rules are not yet implemented.

## Celestial artwork and gameplay panels

Local stars and destinations now use shared preloaded Blender-baked surfaces:
five star classes, rocky/gas planets, a moon, asteroid and trade port. Procedural
halos and selection remain overlaid; galaxy markers retain their small geometric
symbols. These are static billboards with symbolic sizes, not live 3D models or
physical-scale radii. A steeper default system camera separates the small port
from the star. Moon/planet overlaps can be resolved with the candidate list.

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
Projection overlap is still possible from some camera angles; the chooser remains.

Slow planet rotation could add life, but it is deferred: the current sprites have
lighting baked into a single view. A future surface shader or properly baked
rotation frames should keep illumination consistent; spinning the sprite in 2D
would not. Do not introduce an idle animation loop for this decorative effect now.
