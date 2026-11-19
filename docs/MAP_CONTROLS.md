# Map-first fleet command

Select a fleet by clicking its map symbol or its card in **Fleet Command**. A
symbol represents a fleet, with a ship illustration and member count. Selection
alone never moves ships or advances time. The bottom ship selector still chooses
which member's hold is used for trading/services.

Click a planet, station, moon or asteroid to select a destination. The left-hand
course card shows the approach, total fuel per ship and minimum navigation runs.
Choose **PLOT COURSE**, then **Execute navigation run** once all fleets have
courses or explicit Wait orders (fleet cards that need orders offer **Wait**).
Target picking, painting and fleet hit boxes share the same screen positions;
overlapping body targets still have an explicit chooser.

A destination in another system builds departure, entry and arrival legs for you.
Double-clicking a system opens its inner view for inspection; choose a body there
to plot the complete trip. Selecting a system alone plots only the trip to its
entry boundary. No coordinate typing or separate axis-exit steps are needed.
Coordinate tools remain under **Navigation → Advanced coordinates (optional)**.

## Inner / outer transitions

- **Galaxy** changes the view only; it never undocks or moves a fleet.
- The map's **DEPART → GALAXY** marker selects a real outward
  course. It must be plotted and progressed through navigation runs.
- An executed exit switches the selected fleet to the outer map. Completed entry
  opens the destination's inner view. Arrival and transit framing keep the star,
  destinations and selected fleet together instead of centering on the entry point.
- Breadcrumbs, INNER SYSTEM / OUTER GALAXY labels and fleet cards distinguish the
  inspected view from the fleet's authoritative location. Wait orders do not move the camera.

Local labels and input units are AU; galaxy distances are light-years. Rendering
retains symbolic body/fleet sizes and schematic moon spacing. Fleet symbols are
placed away from star/body silhouettes and labels. In free space they sit directly
on their physical course position; displaced berth/orbit symbols have an anchor
and tether. A roster card remains usable
when a crowded viewport cannot safely place another fleet symbol.
**System overview** refits the scene; **Focus selected fleet** provides a close
view. The simulation's system limits have no boundary cube or display toggle.

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

The map fills the window. Every control floats above it on a polished gunmetal
card (a tone ramp with a soft diagonal sheen, chamfered corners and bevelled
edges; static, no brushed lines):

| Card | Position | Contents |
| --- | --- | --- |
| Roster | top-left | fleets, status, destination, NPC traffic |
| View toolbar | top-center | breadcrumb/Depart (system view), camera tools, zoom, guide |
| Status | top-right | Fleets / Navigation / Market / Session buttons, ship, stats |
| Run | bottom-left | Execute run, latest event |
| Selection | bottom-center | selected system or body, destination tiles, traits |

Market, Fleets, Navigation and Session open as centered, movable metal windows.
The system overview framing reserves the card insets, so bodies are framed into
the clear middle of the screen.

**Resting states.** A fleet at a body is either **docked** at the system's
trade port or **in orbit** around a body (that body's orbital berth). There is
no separate landed state.

| State | Map marker | Chip |
| --- | --- | --- |
| Docked | green `F1 DOCKED`, clamp badge, solid clamp line to a bracketed port | `Docked · Sol port` (green) |
| In orbit | cyan `F1 ORBIT`, orbit badge, dashed orbit ring around the body | `Orbit · Inner world` (cyan) |
| Flying | `F1 · n`, thin tether | course / position |
| Paused by a contact | amber `F1 CONTACT` with contact badge | `Underway`; the decision appears under **Execute run** |

A green market badge on the marker, and a **TRADE HERE** button in the
selection card, appear only when the authoritative economy allows that ship to
trade where it is. Moons and asteroids have no market. The status card's panel
buttons are labelled **Fleets**, **Nav**, **Trade** and **Save**.

**Zoom:** the mouse wheel, two-finger trackpad scroll and trackpad pinch zoom
the hovered map. The map is never inside a scrolling page, so a scroll gesture
cannot both scroll the page and zoom. Cards and windows take input before the map.

Text uses one face throughout (Rajdhani), with Orbitron for display headings.
Icons are Blender-authored SVGs (see `assets/README.md`). Values (credits, cargo,
fuel, tick, location), camera tools, trade/refuel/hold actions, commodities and
planet traits (economy, specialization, biome, danger pips) appear as glyphs,
with their text kept as hover help and accessible names. Decisions with
consequences keep short labels: Plot course, Execute run, encounter choices,
Save/Load, Refuel and Depart. Explanations sit behind ⓘ and ? glyphs. Rocky
planets use biome-specific art (temperate, arid, frozen). Touch has no hover,
so icon meaning on mobile still needs a long-press or legend; that is open,
along with the other deferred mobile work.

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
