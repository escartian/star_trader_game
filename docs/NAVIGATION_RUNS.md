# Navigation runs

The campaign now uses shared **navigation runs** in solo and planned multiplayer
play, inspired by fleet management in Star Trader, Civ-style readiness and
FTL-style voyage contacts. A navigation run is the deliberate world-advance step.
Plotting one fleet's course never advances other fleets or markets.

## One flagship, standing orders for the rest

You command one fleet directly: the **flagship** (by default your first fleet).
Only it takes direct orders (courses, landing, lift-off, cancelling). Every
other fleet follows a **standing order**:

- **Wait** (the default for a new or split-off fleet): stay put.
- **Trade route** between two markets (see below).
- **Patrol** a chosen system: tour its bodies one course at a time, topping up
  fuel at its markets when credits allow, so raider bands hunting the system
  meet it. A patrol that cannot set off (no fuel, say) reports it and retries.

Secondary fleets settle their own contacts: raider fights auto-resolve (each
bounty paid once), salvage is taken when a hold has room, distress calls are
answered when medicine is aboard, customs are complied with, convoys get the
best cargo, escorts are flown; a choice the fleet cannot make falls back to
passing the contact by. Their combat reports go to the log rather than opening.
Advance never stops for them. On the Fleets sheet, **Take command** makes a
fleet the flagship; the previous flagship finishes any course under way and
then waits. Saved as `flagship` and `patrols` in navigation state.

## Plan, review, execute

1. Select the flagship and tap a destination (a system, or a body or tile in a
   system). The tap plots the course at once and its card shows fuel and runs;
   deep-space waypoints use the Navigation window's **Plot course**.
2. Plotting leaves the fleet in place; no fuel, ticks or random rolls are
   consumed. You can replace or cancel a course before it departs.
3. The flagship needs a course or an explicit **Wait**. Pressing **Advance** (or
   the single-run clock button) while it has neither selects/highlights it and
   focuses its map location first; nothing advances. With a secondary fleet
   selected, a tap only selects the target (whose card can offer that fleet a
   trade route).
4. Once the flagship is ready, **Advance** executes navigation runs one after
   another (see below). Each run advances all assigned fleets in parallel and
   updates markets once. The clock button beside it executes exactly one run.

For the flagship, a Wait is always issued by the player, never assumed: once it
arrives or its course is cancelled it needs orders again. Once given,
a Wait lasts across runs until you plot a new course for that fleet, so waiting
fleets do not need re-confirming every run. Splits, merges and hull sales keep
the Wait of every fleet that still exists. The save field remains `holds`.

## Advance to next event

**Advance** repeats ordinary navigation runs one after another, 1.2 s apart by
default so each step shows on the map (Settings → **Run speed**: Relaxed 2 s,
Normal 1.2 s, Quick 0.6 s, Rapid 0.35 s), and stops after the run in which:

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
After each run, own ships and NPC traders glide for one run's time (Run speed) from their pre-run to
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

**Raider contact** (one roll in three): **Fight** or try to **Evade**; raiders
cannot be bypassed. See [combat](COMBAT.md) for the group, odds and outcomes.
Otherwise one of ten peaceful contacts, equally likely; each can be passed by:

| Contact | Act | Outcome |
| --- | --- | --- |
| Salvage (debris) | Salvage | 1 ore |
| Distress signal | Assist | −1 medicine, a rescue recorded |
| Derelict freighter | Salvage | 2–6 units of a random good, space allowing |
| Merchant convoy | Trade | up to 5 units of your most valuable cargo at 120% of list price |
| Drifting cargo pod | Open | 3 units of a random good; one time in four a trap damages a hull (never fatally) |
| Customs inspection | Comply | 20 cr fee; **Refuse** risks a 100 cr fine one time in two |
| Authority cutter | Escort | 150 cr; three times in ten a hull takes damage |
| Sensor anomaly | Scan | six times in ten, survey data worth 80 cr |
| Comet | Harvest | up to 10 fuel per ship |
| Lost survey probe | Recover | 120 cr reward |

Credits are at a new game's prices and scale with inflation; a fine never takes
the wallet below zero. These random law contacts do not touch faction standing;
faction police contacts do (see [the law](FACTIONS.md#the-law-wanted-status-and-police)).

**Traders that come to you.** After a run, an NPC trader within hailing
range of your flagship (in open space or orbit, not docked or landed) may
approach it, at most one per run and each trader at most once per 10 runs
(chance by personality, see [traffic](MAP_CONTROLS.md)):

| Contact | Who | Act | Outcome |
| --- | --- | --- | --- |
| Trader alongside | any trader with cargo except a corsair (pirates never trade; a corsair that cannot rob you keeps its distance) | Buy its cargo | its hold at 85% of list price, most valuable first, as far as your holds and wallet allow; the trader keeps the money |
| Trader in distress | a trader a raider band is stalking (six times in ten, whatever its personality, checked first) | Fight the band, Auto-resolve or Leave it | a battle against the band; win and the trader pays you 10% of its wallet, and its faction grants +3 standing |
| Corsair shakedown | a corsair with 150% of your fleet's strength, when you carry cargo | Hand it over, Fight, Auto-resolve or Evade | half your cargo, rounded up; or a battle as with raiders |

The band's bounty is paid as for any band, and it stops stalking the trader
whatever the outcome; leaving the trader costs nothing. Resisting a shakedown is self-defence: no standing lost, no wanted status,
no bounty, and a destroyed corsair's hold is yours. Only the flagship is
approached; secondary fleets never are.

Random state is seeded and saved. Rolls use stable fleet-ID order, not the order
in which you plotted courses. Reloading the same planning state and executing the
same run produces the same contacts. Resolving a contact cannot duplicate rewards
through retries or reloads. A contact's credits (`payout`) are settled into the
wallet by the host in the same command, so saves always hold zero.

## Pursuit

Tap a trader or raider band on either map to select
it (a white ring; its card heads the Selection page with where it is and is
bound, and **Hail** or **Pursue**), or use **Pursue** in the Fleets page's
traffic list (traders in your system out of hailing range, and the raider bands
there). The flagship then chases its quarry, drawn with an amber crosshair and a
dashed amber line from your flagship:

- At once, and after every run (once everything has moved), a flagship within
  reach opens the contact and Advance stops: a trader the hail card (demand
  cargo, attack, let go; within 20 million km; at a port, docked beside it,
  only its trade offer), a band
  the raider fight (within 20 million km). A quarry already in reach is
  contacted immediately.
- Otherwise the flagship steers to intercept. Your fleets and NPC traders move
  at the same speed (50 million km a tick in a system, 4 ticks a run, whatever
  the hull), so trailing a trader rarely closes the gap: a trader with a
  course is met where it is bound (its body or point, in this system or the
  next), and a resting one at its berth. Raider bands and police patrols move
  at half that speed (100 million km a run), so a band is steered for where
  it is. A band crossing to another system is followed there. While a leg is
  under way it is finished first, and at any market on the way the flagship
  refuels as far as credits allow (a jump costs about half a freighter's
  tank). A trader bound for a station is met 8 million km off it, on its way
  in; one that docks between two looks while the flagship is within reach is
  still caught there with the full hail. A chase never settles for a docked
  trader's trade-only offer: it waits off the port for it to come out. While
  you hunt, other traders do not come alongside to sell or shake you down
  (a distress call still can), and the hunted trader never approaches you. If the quarry sits inside a body's collision zone or at the
  flagship's own spot, the flagship waits a run.
- The pursuit ends, with a log note, when the quarry is gone, when no course
  can reach it (out of jump range or fuel; a pursuit that could not even start
  is refused with that reason), when a contact (any kind) stops the
  flagship, or when you give the flagship any other order (**Stop** issues a
  Wait). Only the flagship pursues.

A docked flagship can pursue too: the chase plots its way out of the port.
Saved as `pursuit` in navigation state and validated on load.

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
