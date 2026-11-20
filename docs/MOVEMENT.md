# Fleet movement

Movement now uses [navigation runs](NAVIGATION_RUNS.md). **Plot course** queues a
fleet's intent without changing its location, fuel or world time. **Execute
navigation run** advances every assigned fleet together. Long courses span runs;
per-fleet encounters can pause progress. There is no wall-clock simulation.

## Use the controls

1. Select a fleet on the map or in Fleet Command. Click a planet, station, moon
   or asteroid; use the destination card's PLOT COURSE button.
2. Double-click another system to inspect/select a body there. The itinerary
   automatically includes departure, entry and safe local arrival legs.
3. The departure marker selects an outward course; Back to galaxy only changes
   the view. Actual exits/entries follow the selected fleet between map layers.
4. Execute navigation runs to advance. Long voyages and approach legs continue
   across runs; initial plotting never moves the fleet.

Coordinate waypoints and axis controls remain under Navigation → Advanced
coordinates (optional), rather than the normal interaction flow.

**Focus fleet** in the galaxy and **Focus selected ship** inside a system reveal
ships outside the current view. **Show system boundary** draws the local cube.
Ship illustrations are offset and tethered to body/waypoint anchors for readability.
Moons retain the same schematic display spacing; simulation uses true coordinates.

## Units and costs

`config/movement.json` defines a galaxy half-extent of 100 ly and local system
half-extent of 200,000,000 km on each axis. A cube includes its faces. Exit places
the fleet one kilometer beyond its first crossed face, not at the distant input
point. Entry follows the approach toward the system center and stops on its near
face. Integer component rounding is under one kilometer per axis.

Galaxy anchors are whole light-years. Deep-space locations store absolute whole
kilometers so exit offsets are preserved instead of rounding back onto a star.
Conversion uses 9,460,730,472,581 km per light-year (rounded to the nearest km).
Local positions are a separate type relative to the system center. The client
shows local distances, boundaries and coordinate inputs in AU (1 AU =
149,597,870.7 km); typed AU coordinates round to the nearest integer kilometer.
The default local half-extent is approximately 1.3369 AU. Galaxy views retain ly.

- Local/exit fuel per ship: ceil(distance / 100,000,000 km), at least 1 unit.
- Local/exit duration: ceil(distance / 50,000,000 km), at least 1 tick.
- Deep-space/entry fuel and duration: ceil(distance in ly), at least 1 unit/tick.
- Each ship must pass its own exact squared-distance range check for deep-space
  or entry travel, and have enough fuel for every movement.
- A body arrival attaches to that body; arriving at the primary station docks.
  Attaching/detaching at the same coordinate still costs the minimum fuel/tick.
  A repeated request for the same location rejects instead of consuming time.

The local star has a configurable exclusion radius (default 1,000,000 km).
Deep-space stellar-core crossings reject. Deep-space paths do not inspect
intermediate systems or their bodies.

## Collision volumes

Every body has a collision sphere, sized at map scale rather than physical scale
(a real planet would be invisible in a system 400 million km across). Defaults
in `movement.json` → `body_radii`: rocky planet 600,000 km, gas giant 2,000,000,
moon 250,000, asteroid 150,000, station 50,000. A saved world must keep every
volume inside the system cube and clear of the star's exclusion zone and of every
other body (so routes can always pass between them).

New galaxies give bodies circular paths around the vertical axis; moons circle
their parent planet. Their positions come from the world tick using integer
arithmetic. The map glides between ticks after each navigation run. Saved bodies
without orbit definitions stay fixed. Collision checks cover each body's swept
path, including moons, so a valid world stays clear as time advances.

- A local leg may not pass through the star's sphere or any body's, except a body
  it leaves from or arrives at. Raw advanced segments that do are rejected
  ("passes through a planet, moon or station"), and so are waypoints inside a
  body.
- Named destinations route around everything automatically: a small visibility
  graph over points around each obstacle (cube corners at twice its radius) finds
  the shortest clear chain of waypoints.
- Arrival at a body is still a symbolic orbit or berth at the body's centre.

A queued course is recalculated if the world clock changes before departure.
After each leg, remaining detours are checked against the bodies' new positions;
if no clear path remains, the fleet stops and asks for a new course.

## Orbits

Bodies move. Each circles the star (the Moon circles its planet) on a fixed
circular orbit around the system's vertical (y) axis, keeping its height. A
body's position is a pure function of the world tick (`game_core::orbits`):

- Each orbiting body stores its tick-0 offset from what it circles and its
  period; nothing else is saved. Positions are rotated with integer fixed-point
  trigonometry, so every machine computes identical kilometres.
- Periods follow a Kepler-like rule: 400 ticks (100 navigation runs) at 50
  million km from the star, scaling with radius^1.5. Per lap: the trade port
  about 89 runs, the Inner world 98, the asteroid 148 and the outer gas giant
  410; the Moon circles the Inner world every 12 runs.
- Rules always use bodies where they stand at the planning state's own tick.
  Each plan records its tick, and saves re-check a course at that tick.
- Courses re-aim at each leg: a leg to a body is planned to where it is when the
  leg starts. If a moving body now blocks a detour, the stale waypoints are
  dropped and the rest of the course is re-routed from where the fleet is; if no
  clear way exists the course ends ("a moving body blocks the rest of the course")
  and the fleet needs new orders. A run never fails because a body moved.
- Saved worlds must stay valid at every tick, not just the current one: each
  body's ring around the star (a planet's ring widened by its moons) must stay
  inside the cube and clear of the star and of every other ring.
- The map shows positions for the current tick; while a run's glide plays, bodies
  glide from where they were to where they are, along their curves.

Bodies without an orbit (older saves) stay where they are.

## Landing

Rocky planets, moons and asteroids can be landed on; gas giants and stations
cannot. From orbit, **Land** on the fleet card sets the whole fleet down; **Lift
off** returns it to orbit. Both are immediate (no navigation run) and cost every
ship the body's fuel each way (`movement.json` → `landing`: rocky planet 2, moon
1, asteroid 0). A fleet with a course or an open contact cannot land or lift off.
Plotting a course from the surface lifts off automatically: the first leg's fuel
includes the lift-off. A planet's market serves landed fleets just as it serves
fleets in orbit.

**Mining.** A fleet landed on a moon or asteroid with a Wait order mines ore every
navigation run: 1 ore per ship on a moon, 2 on an asteroid, each limited by the
ship's free cargo space. The card shows **MINING**; a run with full holds reports
"holds full, mining stopped". Advance keeps running while a mining fleet has room
and stops once its holds fill.

## Persistence and authority

World save schema 5 stores movement rules and local/body/deep-space locations.
Schema 1–4 saves migrate without changing balances, fleet membership or
positions. Older active journeys retain their paid fuel and finish through
navigation runs; their fuel is not charged again. New movement uses explicit
exit/entry/docking courses.

Live JSON edits do not change a session. New games read the project movement
configuration if present; embedded defaults support packaged builds. Continuing
uses saved definitions. New Session in the GUI preserves the current definitions.

Client commands contain a ship, target and expected revision. The host rebuilds
the plan from authoritative world coordinates and validates all fleet members.
Both in-process and authenticated session adapters use the same command rules.
A solo run requires a course or Wait order for each fleet and resolved contacts. Future
multiplayer adapters must collect all players' readiness before shared resolution;
networking and multiplayer readiness are not implemented by this change.

Planet visits and station docking are implemented. Rocky/gas planets and primary
stations have separate resource/ship markets. Planetary commerce takes place in
an orbital berth (or on the surface) after a body arrival; waypoints, moons and
asteroids have no markets, but moons and asteroids can be mined after landing. See [planet economies](PLANET_ECONOMY.md) for finite funds and old saves.
