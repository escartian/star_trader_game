# Map-first fleet command

Select a fleet by clicking its map chip or its card on the **Fleets** page. A
symbol represents a fleet, with a ship illustration and member count. Selection
alone never moves ships or advances time. The bottom ship selector still chooses
which member's hold is used for trading/services.

Click a planet, station, moon or asteroid to select a destination. The left-hand
course card shows the approach, total fuel per ship and minimum navigation runs.
Choose **PLOT COURSE**, then **ADVANCE** once all fleets have
courses or explicit Wait orders (fleet cards that need orders offer **Wait**).
For a destination with a market, **TRADE ROUTE** instead loops the fleet between
its current market and that one; its card then shows ON ROUTE, both stops and
**End route** (ROUTE PAUSED with **Resume** when nothing was profitable to carry). See [trade routes](NAVIGATION_RUNS.md#trade-routes).
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

Each run's movement plays out as a short glide on both maps, including NPC
traffic. A ship arriving at a body or port flies in, then takes its berth marker.

Local labels and input units are AU; galaxy distances are light-years. Rendering
retains symbolic body/fleet sizes and schematic moon spacing.

**Fleet chips.** Fleets at the same place merge into one chip showing a
ship, with a count when there are several. Resting fleets sit below-left of
their body (or, in the galaxy view, their star), clear of labels and other chips,
joined to it by a short tether line so a fleet at a star never reads as the star.
Fleets in flight sit on their course position with a short tick toward where
they are heading; on the galaxy map a faint dashed green line runs to their
destination. Your chips (20 pt) are larger than
NPC markers (12 pt) and ringed green: dim, or bright and named when selected;
a fleet with an unresolved contact is ringed amber. Tapping a
chip selects the next fleet in it, cycling through the stack. The roster lists
every fleet.

**Labels and traffic.** Body names show when zoomed in, or for the body selected
or pointed at. NPC traders are small GLB ship models with no labels in the
galaxy view. Map markers and the lines drawn from them use friend-or-foe
colours: **red** for raiders, corsairs, police chasing you and factions that
count you Hostile or want you; **green** for your fleets and factions that
count you Friendly or better; **blue-grey** for everyone else (independents
included). A trader's faction shows as a small pip in its colour (grey if
independent). Raider bands and police patrols are ship markers too (their
heaviest hull, with a ship count): bands ringed red, patrols ringed
friend-or-foe with a pip in their faction's colour. NPC traders
appear in both views: resting ones beside their body or star on a tether
(below-right, then around it), flying ones on their position, and on the galaxy
map a faint dashed line in their colour to the system they are bound for. The
red lines between systems are raider bands crossing. A raider band's heading line is drawn only while it hunts one of your
fleets. A band crossing to another system is drawn on a faint line between
the two systems in the galaxy view, and is absent from the system view. The orbit ring appears only around the selected fleet's body.
**System overview** refits the scene; **Focus selected fleet** provides a close
view. The simulation's system limits have no boundary cube or display toggle.

## Stellar clearance

Named local destinations use a deterministic visibility-graph approach around
the stellar exclusion zone. Each route leg is checked by the same authoritative
movement planner. Raw advanced local routes through the star reject. Deep-space
segments crossing any known stellar core also reject, using a conservative core
cube. Passing through the rest of a system does not automatically enter it.

The map draws approved approach segments rather than inventing a direct line
through the star. Fleet chips sit beside their true course or body anchors; the
screen layout never changes simulated positions. Bodies have collision volumes
that courses also route around; landed fleets show an amber pad on the body
(see [movement](MOVEMENT.md#landing)).

## Command-console style and NPC traffic

The layout is phone-first (see the [rework plan](PHONE_FIRST_REWORK.md)): the map
takes the screen, framed by two panels.

| Part | Position | Contents |
| --- | --- | --- |
| Status strip | top | Back (system view), place name, **Fleets**, **ADVANCE** / **STOP** with the single-run clock, **Menu**, the card toggle (ⓘ), then credits, run number and alert badge; below 720 points wide the actions take a second row |
| Context card | bottom, or docked on the right when the window is wider than tall; can be closed | **Close**, the guide and a contact reminder first, then one page |
| Event card | centred over the map | a contact's decision: an illustrated scene of both sides, the story and the choices as full-width rows; **×** hides it until **Decide** on the card's contact line |
| Message pop-up | bottom of the map | each new message for about 3 s, then fades over 1 s |
| Search | galaxy map buttons (magnifier, top of the column) | opens a name box beside the buttons (also Ctrl/Cmd+F); Escape, a pick or the button closes it |
| Map buttons | map's top-right corner | overview or reset, focus selection and fleet |

The sheet's pages:

- **Selection** (default): what was tapped: in a system, Depart, trade
  shortcuts, destination tiles and the selected body; in the galaxy, the
  selected system; then the destination card with **PLOT COURSE** or **TRADE
  ROUTE**.
- **Fleets:** the ship used for trading with its cargo and fuel, then every fleet
  (status, Wait, Land / Lift off, route controls) and NPC traffic.
- **Menu:** **Songbook · Music** (play or queue any part, as from the main
  menu), **Trade**, **Fleets**, **Navigation**, **Victory**,
  **Factions**, **Save & load** (slots, Save, Load), **New session** (the new
  game's setup, one setting per line, and an optional fixed seed) and
  **Settings**, then **Latest events**
  (the last 12 messages, newest first, highlighted; formerly the action bar's
  line and the Session window's log).

The card's **Close** button hides it entirely, giving the map the whole
screen below the strip. It comes back with the strip's card toggle, a Fleets or
Menu tap, a tap on the map (a new selection), another selected ship or a new
contact. Tapping an open page's strip button returns to Selection.

Stars, planets and ports have no hover pop-ups: the selection card shows a
star's type, holder and your standing, and a body's kind and traits.

Tapping a body or system shows Selection; tapping a fleet chip shows Fleets;
tapping a trader or raider band selects it (see
[pursuit](NAVIGATION_RUNS.md#pursuit)). On the galaxy map a pursuit shows as an
amber dashed line from your flagship to the quarry and on, fainter, to the
system the quarry is bound for. The galaxy map's course line runs only
to where the selected fleet's course ends; cancelling the course (or a Wait)
removes it and the system view's route line.
Tapping the place the flagship rests at cancels a course it has not set off on
(the destination card and the Shipyard tab also show **Cancel course**): hull
purchases, fleet changes and landing need no plotted course. Double-clicking a
body (or chip) where one of your fleets can trade opens that market.
The amber alert badge counts fleets needing a decision or orders: tap it to
select the next one, which also switches to the page that fleet needs. When
Advance stops, the sheet does the same. Market (with its **Shipyard** and **Jobs** tabs), Fleets, Navigation, Jobs,
Session and the combat report open as centred windows on a wide screen, and
full screen with a **Back** button on a narrow one (below 700 points), scrolling
sideways where a table is wider than the screen. The system overview frames
the bodies into the whole map, clear of the corner buttons.

**Resting states.** A fleet at a body is **docked** at the system's trade port,
**in orbit** around a body, or **landed** on its surface.

| State | Map marker | Location chip |
| --- | --- | --- |
| Docked | chip beside the port | `Docked · Sol port` (green) |
| In orbit | chip beside the body; dashed orbit ring when selected | `Orbit · Inner world` (cyan) |
| Landed | chip beside the body, amber pad on it | `Landed · Moon` |
| Flying | chip on its course with a heading tick | course / position |
| Paused by a contact | amber ring | `Underway`; the decision appears under **Advance** |

A **TRADE** shortcut on the Selection page appears only when the authoritative
economy allows that ship to trade where it is. Moons and asteroids have no
market.

**Zoom:** the mouse wheel, two-finger trackpad scroll and trackpad pinch zoom
the hovered map. The map is never inside a scrolling page, so a scroll gesture
cannot both scroll the page and zoom. Cards and windows take input before the map.

Text uses one face throughout (Rajdhani), with Orbitron for display headings.
Icons are Blender-authored SVGs (see `assets/README.md`). Values (credits, cargo,
fuel, tick, location), camera tools, trade/refuel/hold actions, commodities and
planet traits (economy, specialization, biome, danger pips) appear as glyphs,
with their text kept as hover help and accessible names. Decisions with
consequences keep short labels: Plot course, Advance/Stop, encounter choices,
Save/Load, Refuel/Repair (per ship and fleet), Max/All and Depart. Explanations sit behind ⓘ and ? glyphs. Rocky
planets use biome-specific art (temperate, arid, frozen, ocean, jungle, volcanic,
barren). Gas giant art follows its hazard profile. Touch has no hover,
so icon meaning on mobile still needs a long-press or legend; that is open,
along with the other deferred mobile work.

Fresh sessions spawn one NPC trader per three market systems: at least four
(fewer only when a tiny galaxy has one planet market) and at most 32. The first
two are always **Aster Haul** (cautious) and **Helix Courier** (opportunistic);
later traders are dealt personalities in turn from `config/npc.json`. Home ports are spread evenly over the planet markets,
and a destroyed trader is refitted at its own. Each trader flies for the faction
holding its home port when the game starts, or is independent; the traffic
roster shows its faction in that faction's colour. Their traffic entries and
galaxy markers show genuine persisted traders, not decorative ships.

Personalities (tuned in [`config/npc.json`](DATA_FORMATS.md#npc-personality-catalog-schema-1);
hover a trader's name in the roster for its own):

| Personality | Hauls | Danger | Margin | Yields to | Slips raiders |
| --- | --- | --- | --- | --- | --- |
| Cautious | 4 units | avoids | 10% | 150% strength | never |
| Opportunist | 4 | goes | any | 300% | never |
| Bulk hauler | 12 | avoids | 3% | 200% | never |
| Loyalist | 6 | goes; own and allied space only | 5% | 250% | never |
| Smuggler | 6 | prefers (hauls valued 150%) | 5% | 250% | one time in two |
| Corsair | 4 | prefers (120%) | any | 400% | one time in five |

A loyalist dealt to an independent trader becomes a smuggler. Traders also
come to your flagship to trade or to call for help against raiders, and a
corsair that outguns a laden flagship shakes it down (a weaker corsair keeps
its distance: pirates never trade with you)
([traders that come to you](NAVIGATION_RUNS.md#encounters)); per run, when in
reach: cautious and loyalist 5%, bulk hauler 10%, opportunist and smuggler
15%, corsair 30%. Traders compare commodity
routes including fuel costs among the six market systems nearest them (their own
included), buy/haul/sell real cargo, refuel with their own credits and use the
same safe multi-leg route planner. A faction trader never sails for a port held
by a rival of its faction, and values hauls into its own or allied space at 125%
of their profit; independents go anywhere.

NPCs act once per shared navigation run. They have separate wallets/hulls, finite
market stock/funds and saved journey state. Trades conserve money and goods;
production runs once for the world. Earlier saved sessions
without NPCs retain that population; start a fresh session for the initial traders.
Backward save compatibility is not a prototype requirement.
