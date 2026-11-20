# Desktop acceptance and first-loop playtest

These checks require observed use. Automated tests/captures do not mark them done.
Mobile builds and device checks are deferred by user direction.

## Native interaction

- [ ] Launch, resize and change display DPI; confirm text/sprites and input align.
- [ ] Single-click/tap selects; double-click enters an unambiguous system only.
- [ ] Open the overlap fixture, inspect all candidates, orbit, focus and reset.
- [ ] Use camera presets/orbit and candidate selection to inspect the dense fixture.
- [ ] Enter a local system and return; galaxy camera/selection are preserved.
- [ ] Verify transparent celestial artwork, scale hierarchy, moon separation and readable labels.
- [ ] Minimize/restore and suspend/resume without losing state or advancing background time.
- [ ] Observe idle redraw/power behavior rather than infer it from the event loop.

## First delivery / upgrade

- [ ] Start a new seed, follow the guide, buy food, travel to Meridian and sell.
- [ ] Identify fuel/cargo/stock/funds constraints before a failed order.
- [ ] Compare another commodity/route and include journey fuel costs in the choice.
- [ ] Refuel and complete a profitable return trip.
- [ ] Use Max, All and Full; check hover totals and whether the margin column
  makes a profitable next cargo obvious without opening Price comparison.
- [ ] Discover and purchase the cargo-hold upgrade without outside explanation.
- [ ] Record first-delivery and upgrade times, credits and any confusing decisions.
- [ ] Judge whether the 20–30 minute upgrade hypothesis should change.

## Returning safely

- [ ] Save to another slot while travelling; Continue restores the same journey/cargo/markets.
- [ ] Close normally and relaunch; autosave resumes without re-rolling prices or adding offline time.
- [ ] Verify a failed save leaves the window open with actionable recovery text.
- [ ] Recover a deliberately damaged copied primary from its backup; keep real progress untouched.
- [ ] Confirm ticks advance only through navigation runs (Advance or the clock button); plotting, waiting and trading leave time unchanged.
- [ ] Select slot 2, save, quit and relaunch; verify remembered slot, active ship, guide and autosave choices.

## Before expanding scope

Observe 5–10 trading/management players once the desktop acceptance checks pass.
Record navigation, profitable delivery, upgrade discovery and willingness to return.
Capture dominant routes/confusion before adding factions, encounters or fleets.
No participants have been contacted and no playtest results are claimed here.

- [ ] Exit a system in Navigation, enter a deep-space waypoint, explicitly enter
  the destination system, and dock at its trade port before trading.
- [ ] Inspect another system without moving; verify the ship remains where it was.
- [ ] Try a local route through the star, then route around it with a waypoint.
- [ ] Visit a planet/moon, focus the selected ship and inspect its tethered marker.
- [ ] Show the system boundary and save/continue from an entry face/deep space.

- [ ] Trade at two planets in the same system and verify separate stock/funds.
- [ ] Buy a hull, split its fleet, sell it empty and verify local stock replenishes.
- [ ] Rename a fleet, save and continue with the same name and ship membership.
- [ ] Compare a migrated save's empty new treasuries/yards against the explicit
  configured budgets in a fresh slot. Verify no player assets or station goods reset.

- [ ] Split two fleets, plot only one course and click Advance.
  Verify the unassigned fleet is selected/highlighted before anything advances.
- [ ] Order the other fleet to Wait, execute, and confirm both share one world clock step.
- [ ] Execute further runs and confirm the waiting fleet is not asked for orders again.
- [ ] Advance a multi-leg route to another system: ships visibly step each run, the
  camera follows exit/entry, and the advance stops on arrival with the fleet focused.
- [ ] Watch an advance on both maps: ships and NPC traffic glide continuously
  between runs, and a ship arriving at a body flies in before taking its berth.
- [ ] Press Stop mid-advance; confirm no further runs execute. Advance into a contact
  and confirm its decision card is shown under the run controls.
- [ ] Start a trade route Sol port to Meridian port and Advance: the fleet trades at
  both ends without asking for orders, the log explains each stop, and credits
  trend upward over several loops. End route and confirm it stops after the leg.
  When a stop finds nothing profitable, confirm Advance stops with the reason and
  Resume continues once prices recover.
- [ ] Follow a multi-run voyage and verify fuel is charged once at departure.
- [ ] Resolve a salvage/distress contact, save/continue, and check that the decision
  and cargo/rescue effects occur once. Verify other fleets progressed independently.

- [ ] Click fleet symbols in both map layers and confirm selection matches the roster.
- [ ] Select a remote planet and plot without typing coordinates. Follow the full
  departure/entry/arrival itinerary and check the camera follows real transitions.
- [ ] Change views with breadcrumbs without moving ships; use the departure marker
  only when intending a real exit. Verify star silhouettes never contain fleet symbols.
- [ ] Observe NPC traffic after several runs and compare local stock/funds changes.
