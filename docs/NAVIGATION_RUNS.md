# Navigation runs

The campaign now uses shared **navigation runs** in solo and planned multiplayer
play, inspired by fleet management in Star Trader, Civ-style readiness and
FTL-style voyage contacts. A navigation run is the deliberate world-advance step.
Plotting one fleet's course never advances other fleets or markets.

## Plan, review, execute

1. Select a fleet and tap a destination (a system, or a body or tile in a
   system). The tap plots the course at once and its card shows fuel and runs;
   deep-space waypoints use the Navigation window's **Plot course**.
2. Plotting leaves the fleet in place; no fuel, ticks or random rolls are
   consumed. You can replace or cancel a course before it departs.
3. Give every other fleet a course or an explicit **Wait** order. Navigation shows
   each fleet's status and destination; Navigation and the map's fleet cards both
   offer Wait for fleets that need orders. Pressing **Advance** (or the single-run
   clock button) with an unassigned fleet selects/highlights it and focuses its
   map location first; nothing advances.
4. Once all fleets are ready, **Advance** executes navigation runs one after
   another (see below). Each run advances all assigned fleets in parallel and
   updates markets once. The clock button beside it executes exactly one run.

A Wait is always issued by the player, never assumed: a fleet that has just
arrived, was split off or had its course cancelled needs orders again. Once given,
a Wait lasts across runs until you plot a new course for that fleet, so waiting
fleets do not need re-confirming every run. Splits, merges and hull sales keep
the Wait of every fleet that still exists. The save field remains `holds`.

## Advance to next event

**Advance** repeats ordinary navigation runs about three times per second
(0.35 s apart, so each step shows on the map) and stops after the run in which:

- a contact needs a decision: the fleet is selected and its decision appears at
  the top of the context sheet;
- a fleet completes its route and needs orders: the fleet is selected and the
  map focuses it (intermediate itinerary legs do not stop an advance);
- no fleet is underway, e.g. every fleet is Waiting, so Advance acts as one run;
- a run is rejected, or a 64-run safety limit is reached.

**Stop** ends an advance after its latest completed run. Loading or starting a
session cancels it. Advance never changes the rules: every step is the same
command as a single run, so the results and saves are identical to pressing the
clock button repeatedly.

Each run currently advances four internal ticks (`ticks_per_navigation_run` in
config/movement.json). Course durations retain their distance-based calculations;
a 15-tick voyage takes at least four runs. Short courses can arrive in one run.
Long voyages automatically continue on following runs without reissuing orders.
The shared world advances four ticks regardless of fleet count or course length.
Bodies orbit as the world tick advances. A queued leg is recalculated at departure
if the clock changed since plotting. After a leg, the next leg uses the bodies'
current positions and re-routes an obstructed detour. If a clear route cannot be
found, the fleet stops and needs a new course.

Fuel is charged once per leg when that leg departs. A selected destination can
include safe waypoints, departure, entry and local arrival legs; all are previewed
for fuel/range before plotting. Underway fleets cannot trade,
refuel, split/merge, buy/sell hulls or cancel/replot. Queued courses must be cancelled
before changing fleet membership. Trading/loading/naming/encounter choices do not
advance the clock. Ordering every fleet to Wait is a valid deliberate waiting run.

Local-system displays and waypoint inputs use AU; authoritative positions remain
integer kilometers. Galaxy coordinates are displayed in light-years. Rendering
interpolates active course positions from saved progress; it never advances time.
After each run, own ships and NPC traders glide for 0.35 s from their pre-run to
their post-run positions (the Advance interval, so an advance reads as one
continuous voyage). The glide is display-only; camera-follow, saves and rules use
the authoritative positions, and captures (animations off) show final positions.
Existing paid voyages from older saves progress on navigation runs too.

## Trade routes

Select a destination with a market while a fleet is at a different market and
choose **TRADE ROUTE**. The fleet then loops between the two markets on its own:

1. At once (no time passes) it keeps any cargo aboard, fills the remaining space
   with the most profitable cargo for the far end and plots that leg.
2. Each time it arrives at either end it sells everything it carries, refuels as
   far as credits allow, buys the most profitable cargo for the other end and
   sets off on the next run. Advance does not stop for these stops; each one is
   logged ("Fleet 1 route · At Sol Trade port: sold …; bought …; bound for …").

"Most profitable" is chosen greedily one cargo unit at a time: each unit goes to
the good with the largest margin between the far end's sell price at its rising
stock and the local buy price at its falling stock, the same per-unit curves trade
quotes use. Margins only shrink as units are added, so this maximizes estimated
profit for the space. Buying stops when the hold, credits, the far market's
storage or buyer funds run out, or nothing pays. Prices at arrival may differ.
Route trades use the player's credits.

If a stop would send the fleet off with an empty hold (nothing aboard and nothing
profitable to buy), the route **pauses** instead: the fleet stays at that stop,
its card shows ROUTE PAUSED with **Resume** and **End route**, and it needs
orders, so Advance stops and says why. Resume re-checks prices and sets off if
any cargo pays (otherwise it is rejected and the route stays paused); any other
order ends the route. Starting a route with an empty hold and nothing profitable
is rejected the same way.

Route legs are ordinary courses: they roll contacts like any fleet, and a contact
pauses the route fleet and stops Advance until you decide. Any new order for the
fleet (Plot course, Wait, Cancel) ends its route. **End route** on the fleet's card
lets the current leg finish; the fleet then needs orders. A stop that cannot
continue (for example, no money for fuel to reach the far end) ends the route
after selling and refuelling, and the fleet needs orders, which stops Advance.
Destroyed, sold or merged-away fleets lose their routes. Routes are saved in the
navigation state (`routes`, omitted when empty) and validated on load.

## Encounters

Each travelling fleet gets its own encounter roll per run. Waiting fleets do not
roll. The default probability is 12%, configurable through
`encounter_chance_percent`. A contact pauses that fleet's progress for the run;
other fleets continue. Resolve outstanding contacts before executing the next run.

- **Salvage contact:** recover one ore if any member has room, or bypass the field.
- **Distress signal:** spend one medicine from this fleet to assist and record a
  rescue, or bypass the contact. The rescue counter persists across saves.
- **Raider contact** (one roll in three): **Fight** or try to **Evade**; raiders
  cannot be bypassed. See [combat](COMBAT.md) for the group, odds and outcomes.

NPC trade, reputation rewards, inspections and branching authored story chains
remain roadmap work built on this lifecycle.

Random state is seeded and saved. Rolls use stable fleet-ID order, not the order
in which you plotted courses. Reloading the same planning state and executing the
same run produces the same contacts. Resolving a contact cannot duplicate rewards
through retries or reloads. Salvage explicitly creates the recovered ore; rescue
assistance consumes medicine; raider wrecks yield ore. No contact grants credits yet.

## Saves and multiplayer

Save schema 7 stores run number, queued/active courses, Wait orders, encounters,
rescue count and random-generator state. Older schemas are rejected. Loading never
executes a run. Course geometry, membership, counters and encounter references
are validated.

The authoritative session owns the entire resolution transaction. A stale run
request rejects without advancing twice. Solo play can execute after its fleets
are ready. Advance is client-side repetition of that single-run command; in
multiplayer it maps to standing readiness that lapses on the same stop conditions.
Multiplayer still needs authenticated accounts, readiness collection,
order visibility and reconnect rules before shared execution can be exposed to
remote clients; this change does not implement network multiplayer.

Backward save compatibility is not a prototype requirement. Future breaking
changes may require a new session.

The map-first interface now selects fleets and destinations directly. Automatic
multi-leg itineraries continue over runs without re-entering coordinates. Initial
NPC traders make decisions and advance their own safe routes once per global run;
market production still executes once. See MAP_CONTROLS.md for current scope.
