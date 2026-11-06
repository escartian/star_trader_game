# Map-first fleet command

Select a fleet by clicking its map symbol or its card in **Fleet Command**. A
symbol represents a fleet, with a ship illustration and member count. Selection
alone never moves ships or advances time. The bottom ship selector still chooses
which member's hold is used for trading/services.

Click a planet, station, moon or asteroid to select a destination. The left-hand
course card shows the approach, total fuel per ship and minimum navigation runs.
Choose **PLOT COURSE**, then **Execute navigation run** once all fleets have
courses or explicit Holds. Target picking, painting and fleet hit boxes share the
same screen positions; overlapping body targets still have an explicit chooser.

A destination in another system builds departure, entry and arrival legs for you.
Double-clicking a system opens its inner view for inspection; choose a body there
to plot the complete trip. Selecting a system alone plots only the trip to its
entry boundary. No coordinate typing or separate axis-exit steps are needed.
Coordinate tools remain under **Navigation → Advanced coordinates (optional)**.

## Inner / outer transitions

- **Back to galaxy** changes the view only; it never undocks or moves a fleet.
- **Depart system → galaxy** and the map's departure marker select a real outward
  course. It must be plotted and progressed through navigation runs.
- An executed exit switches the selected fleet to the outer map. Completed entry
  opens the destination's inner view; local arrival follows the selected fleet.
- Breadcrumbs, INNER SYSTEM / OUTER GALAXY labels and fleet cards distinguish the
  inspected view from the fleet's authoritative location. Holds do not move the camera.

Local labels and input units are AU; galaxy distances are light-years. Rendering
retains symbolic body/fleet sizes and schematic moon spacing. Fleet symbols are
placed away from star/body silhouettes and labels. A roster card remains usable
when a crowded viewport cannot safely place another fleet symbol.

## Stellar clearance

Named local destinations use a deterministic visibility-graph approach around
the stellar exclusion zone. Each route leg is checked by the same authoritative
movement planner. Raw advanced local routes through the star reject. Deep-space
segments crossing any known stellar core also reject, using a conservative core
cube. Passing through the rest of a system does not automatically enter it.

The map draws approved approach segments rather than inventing a direct line
through the star. Fleet symbols are offset/tethered to true course/body anchors;
the screen layout never changes simulated positions. Planet/moon collision volumes
and landing physics are still future work.

## Command-console style and NPC traffic

Native egui panels use a dark blue command-console theme, cyan selections/course
lines, amber traffic/attention, compact fleet cards and collapsible camera tools.
This is static native drawing; Blender sprites remain shared cached artwork.

Fresh sessions spawn **Aster Haul** (cautious) and **Helix Courier** (opportunistic).
Their read-only traffic entries and amber galaxy markers show genuine persisted
traders, not decorative ships. Cautious traders avoid dangerous markets and require
a larger profit margin; opportunists accept smaller margins and riskier markets.
They compare commodity routes including fuel costs, buy/haul/sell real cargo,
refuel with their own credits and use the same safe multi-leg route planner.

NPCs act once per shared navigation run. They have separate wallets/hulls, finite
market stock/funds and saved journey state. Trades conserve money and goods;
production runs once for the world. NPC random encounters, player conversations,
combat and expanded personalities are not implemented yet. Earlier saved sessions
without NPCs retain that population; start a fresh session for the initial traders.
Backward save compatibility is not a prototype requirement.
