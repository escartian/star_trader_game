# Navigation runs

The campaign now uses shared **navigation runs** in solo and planned multiplayer
play, inspired by fleet management in Star Trader, Civ-style readiness and
FTL-style voyage contacts. A navigation run is the deliberate world-advance step.
Plotting one fleet's course never advances other fleets or markets.

## Plan, review, execute

1. Select a fleet and preview a local/body, exit, deep-space or entry course.
2. Choose **Plot course**. The fleet stays put; no fuel, ticks or random rolls are
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

- a contact needs a decision: the fleet is selected and its decision card opens
  under the run controls;
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

Fuel is charged once per leg when that leg departs. A selected destination can
include safe waypoints, departure, entry and local arrival legs; all are previewed
for fuel/range before plotting. Underway fleets cannot trade,
refuel, split/merge, buy/sell hulls or cancel/replot. Queued courses must be cancelled
before changing fleet membership. Trading/loading/naming/encounter choices do not
advance the clock. Ordering every fleet to Wait is a valid deliberate waiting run.

Local-system displays and waypoint inputs use AU; authoritative positions remain
integer kilometers. Galaxy coordinates are displayed in light-years. Rendering
interpolates active course positions from saved progress; it never advances time.
Existing paid voyages from older saves progress on navigation runs too.

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

Save schema 5 stores run number, queued/active courses, Wait orders, encounters, rescue
count and random-generator state. Schemas 1–4 migrate without moving ships or
resetting balances. Schema-4 market money/inventory is preserved; older planetary
migrations retain their documented empty new accounts. Loading never executes a
run. Course geometry, membership, counters and encounter references are validated.

The authoritative session owns the entire resolution transaction. A stale run
request rejects without advancing twice. Solo play can execute after its fleets
are ready. Advance is client-side repetition of that single-run command; in
multiplayer it maps to standing readiness that lapses on the same stop conditions.
Multiplayer still needs authenticated accounts, readiness collection,
order visibility and reconnect rules before shared execution can be exposed to
remote clients; this change does not implement network multiplayer.

Backward save compatibility is not a prototype requirement. Existing migration
coverage describes this batch only; future breaking changes may require a new
session rather than additional migration work.

The map-first interface now selects fleets and destinations directly. Automatic
multi-leg itineraries continue over runs without re-entering coordinates. Initial
NPC traders make decisions and advance their own safe routes once per global run;
market production still executes once. See MAP_CONTROLS.md for current scope.
