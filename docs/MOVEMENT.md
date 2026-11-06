# Fleet movement

Movement now uses [navigation runs](NAVIGATION_RUNS.md). **Plot course** queues a
fleet's intent without changing its location, fuel or world time. **Execute
navigation run** advances every assigned fleet together. Long courses span runs;
per-fleet encounters can pause progress. There is no wall-clock simulation.

## Use the controls

1. Double-click a star to inspect its system. Select a body and choose **Preview
   movement to body**, or **Preview docking at trade port**. Plot the course in Navigation, then execute navigation runs.
2. **Navigation** accepts local X/Y/Z waypoints in AU. An inside target
   stays in the system; an outside target exits at the first cube face crossed.
   The six exit buttons choose an axis while retaining the other coordinates.
3. In deep space, enter waypoint coordinates in light-years, or choose a system
   by name. Exact system coordinates explicitly request entry. Passing a system
   along a path never enters or intercepts it.
4. Entry arrives at the near cube boundary. Inspect the destination system, select
   its trade port, plot docking and execute runs until arrival before trading/refuelling.

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
A segment through this sphere rejects; choose a waypoint around it. Body arrival
means a symbolic berth/orbit, not a physics simulation or landing. Planet/moon
radii and collision volumes are not modeled yet. Deep-space paths do not inspect
intermediate systems or their bodies.

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
A solo run requires a course or Hold for each fleet and resolved contacts. Future
multiplayer adapters must collect all players' readiness before shared resolution;
networking and multiplayer readiness are not implemented by this change.

Planet visits and station docking are implemented. Rocky/gas planets and primary
stations have separate resource/ship markets. Planetary commerce takes place in
an orbital berth after a body arrival; waypoints, moons and asteroids have no
markets yet. See [planet economies](PLANET_ECONOMY.md) for finite funds and old saves.
