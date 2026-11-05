# Implementation roadmap

Work in this order. Each phase has a visible completion criterion; complete it
before expanding scope. These are dependency milestones, not calendar estimates.
The initial desktop interface and local snapshot session are implemented; mobile
feasibility and the playable simulation remain open. See docs/PLATFORM_STATUS.md.

## 0. Establish the boundaries

- [x] Create core, protocol, server, and client crates with a compiling fixture.
- [x] Record the no-engine architecture and 3D map interaction plan.
- [x] Record initial design decisions: premium/offline-first single-player with
  optional multiplayer is the working assumption. Rules use discrete ticks;
  single-player schedules them continuously, multiplayer will resolve explicit turns.
- [x] Record working coordinate units, travel rules, and a concrete first-session goal
  in [first-session rules](docs/FIRST_SESSION.md).
  Proposed goal: earn enough to buy a larger cargo hold in roughly 20–30 minutes.

**Done when:** the first-session rules fit on one page and do not require factions,
real-time piloting, or multiplayer to be entertaining.

## 1. Prove the native interface and platform path

- [x] Add a compatible pinned library set: winit, wgpu, egui integrations, glam.
- [ ] Open a window, draw text and a selectable star, and handle resize/DPI changes.
- [ ] Prove a scrollable market table and readable layout on a phone-size viewport.
- [ ] Build a minimal Android and iOS shell; check touch, safe areas, suspend/resume,
  GPU surface recreation, and redraw behavior on real devices where available.
- [x] Document any unavailable SDK/device checks and unresolved platform blockers.
- [ ] Render on demand while idle; animate only while movement/effects require it.

**Done when:** desktop works and mobile feasibility is established with explicit
evidence. Resolve platform blockers before investing in the full UI. If no device
is available, treat feasibility as unresolved rather than marking this phase complete.

## 2. Make the 3D universe understandable

- [x] Use a fixed small galaxy plus fixtures with overlapping and dense stars.
  Session / Overlap test / Dense test switcher; fixture state stays outside the session.
- [x] Implement orthographic projection, orbit, pan, zoom, camera reset and focus.
  Focus button/F and a visible pan tool are implemented; focus has automated coverage.
- [x] Add top/front/side presets, axis indicator, labels, hover and selection.
  Labels avoid other labels and reserve space for the selected system.
- [ ] Resolve projected overlap with a candidate list; support touch selection.
  Explicit candidate list and 24-point pick radius implemented and tested; real touch
  and gesture-conflict verification remain open.
- [x] Show actual 3D distance and a selected route, independent of camera angle.
  Distance uses world positions; the overlap fixture covers distinct depths.
- [x] Add an adjustable depth slice for dense/overlapping fixtures.
  Choose X/Y/Z, center and thickness in light-years. Bounds/count are visible;
  outside stars are hidden and excluded from picking. Focus reveals a hidden
  selection without changing thickness; returning from local view preserves the slice.
- [x] Switch between galaxy and local system views while preserving selection.
  Open selected system shows fixed planets/stations in local kilometers. The Galaxy
  breadcrumb/Escape returns without changing galaxy camera, selection or candidates.

**Done when:** a new player can find a system, distinguish depth, select an
overlapping target, and explain a route's distance without entering coordinates.
See [map design](docs/MAP_DESIGN.md).

Map presentation now includes a static procedural nebula/starfield, soft star
halos and symbolic sizes/colors driven by explicit star kinds. This introduces
no simulation randomness or continuous animation. The trade port now uses an
original Blender-authored transparent sprite; editable source/export script are kept
in assets/ and tools/art/. PNG decoding is isolated in the local artwork module.
Phase 2 remains incomplete: device interaction and manual acceptance scenarios
still require work. Local-system navigation is implemented with fixed destinations;
galaxy travel has its first authoritative implementation; local docking is pending. Phase 1 platform blockers remain
open; these map changes extend the existing prototype without adding a full UI.

## 3. Build one authoritative local session

- [ ] Define stable entity IDs, separate galaxy/local positions, fixed-unit money,
  cargo quantities, ships, fleets, markets, and simulation time.
- [x] Add reusable session handling to the server package; invoke it in-process
  for offline play. Galaxy/session snapshots, travel previews and departures are handled.
- [x] Implement initial travel commands, validation, typed errors, events and visible snapshots.
  Commands carry ship/destination IDs and expected revisions, never a trusted owner ID.
- [x] Add movement range, fuel costs, travel time and arrival handling.
  Exact integer distance/ceil rounding, checked time arithmetic and host-tick arrivals;
  the client displays fuel, location, quotes, errors and automatic travel progress.
- [ ] Validate fuel/capacity/ownership and apply each command atomically.
- [ ] Add seeded generation with a guaranteed usable start and reachable markets.
- [ ] Keep hidden world state out of client snapshots when discovery is introduced.

**Done when:** the client can start a session and move a ship between markets;
invalid actions leave state unchanged. Test travel boundaries, command atomicity,
and reproducible generation rather than UI implementation details.

## 4. Complete the first trading loop

- [ ] Implement buy/sell commands, credits, cargo capacity and market stock.
- [ ] Add production, consumption, bounded stock-sensitive pricing and a spread.
- [ ] Advance markets on simulation time, never on opening or refreshing a panel.
- [ ] Prevent profitable immediate buy/sell loops at the same market.
- [ ] Show price per unit, total cost, capacity, stock and estimated journey costs.
- [ ] Show comparisons for known markets; give quotes a timestamp/revision so
  stale purchases can be rejected clearly, without silently changing the price.
- [ ] Add one cargo upgrade and the first-session earnings goal.

**Done when:** travel → buy → travel → sell → upgrade works; multiple routes have
credible tradeoffs, and credits/cargo/stock stay consistent on rejected trades.
Test economic invariants and representative routes.

## 5. Make it safe to return and easy to learn

- [ ] Add a versioned save format, validated loading, atomic replacement and backup.
- [ ] Save/load world, markets, simulation time, player, ships and current journey.
- [ ] Add new/continue/save slots, settings, a short guided first delivery and logs.
- [ ] Add recovery messages for damaged or incompatible saves.
- [ ] Ensure loading does not regenerate prices or duplicate money/items.
- [ ] Validate offline suspend/resume; persist at safe lifecycle points on mobile.

**Done when:** players can quit mid-session and resume the same state. Verify save
round trips and failed-write recovery. This phase is required before external testing.

## 6. Test whether the game deserves expansion

- [ ] Give the small build to 5–10 trading/management players.
- [ ] Observe navigation, first profitable delivery, upgrade discovery and return play.
- [ ] Ask which decisions were interesting and where the game became repetitive.
- [ ] Fix confusing navigation, pacing and obvious dominant routes first.
- [ ] Record a continue/change-direction decision from observed behavior.

**Done when:** there is evidence people understand and enjoy the core loop.
Do not expand the feature list to compensate for an uninteresting economy.

## 7. Add risk, identity and progression

- [ ] Delivery contracts with deadlines, deposits and clear failure consequences.
- [ ] A small encounter set: trader, inspection, distress call, pirate demand.
- [ ] Choices to negotiate, flee, surrender cargo or fight, with understandable odds.
- [ ] Simple combat resolution driven by relevant equipment, damage and repairs.
- [ ] Distinct ship roles, cargo/range/defense upgrades and maintenance costs.
- [ ] Small faction reputation model, legal restrictions and meaningful access perks.
- [ ] Support a few fleets with useful summaries and low repetitive interaction cost.

**Done when:** safe and risky strategies are both viable, and upgrades alter choices.
Avoid a separate tactical combat game unless playtesting establishes a need.

## 8. Connect remote clients

- [ ] Extract transport adapters around the existing session command interface.
- [ ] Add serialization, protocol negotiation, request IDs and state revisions.
- [ ] Implement an authoritative dedicated server with ordered command processing.
- [ ] Establish authenticated player identity; do not trust an owner ID in a command.
- [ ] Use TLS for public connections and bounds on message size/request frequency.
- [ ] Reject stale trades, prevent duplicate actions after retry, and resync snapshots.
- [ ] Handle reconnect, server restart, disconnect during travel and concurrent trades.
- [ ] Implement shared-turn collection/resolution and session timing configuration.
  Direction chosen: continuous single-player over discrete ticks; explicit turns for
  multiplayer. Host-only clock advancement is separate from player commands.
- [ ] Test two clients competing for the same stock and retrying the same purchase.

**Done when:** two clients share one consistent world, reconnect safely, and cannot
spend the same credits or purchase the same stock twice. No MMO infrastructure yet.

## 9. Ship desktop, then finish mobile

- [ ] Package and smoke-test Windows, macOS and Linux builds.
- [ ] Add audio, visual feedback, accessibility settings and configurable inputs.
- [ ] Finish phone/tablet layouts, gestures, safe areas and text sizing.
- [ ] Profile map density, battery use, memory and network interruptions on devices.
- [ ] Add mobile packaging, signing, store assets and platform-specific integration.
- [ ] Run full-session checks on each supported platform, both offline and connected.

**Done when:** a fresh install can complete, save and resume the core session on
every claimed platform. Desktop may ship before the mobile rollout.

## Deferred until there is a clear player need

Planet colonization/development, elaborate diplomacy, research trees, orbital
mechanics, fully rendered planets, real-time ship piloting, tactical 3D battles,
multiple galaxies, MMO-scale hosting, modding infrastructure and a custom engine.

## First implementation batch

1. Complete the one-page rules and coordinate/time decisions from phase 0.
2. Open the native window and render one egui panel with the proposed library stack.
3. Draw the fixture's 3D stars through an orthographic camera; select by mouse/touch.
4. Build the minimal Android/iOS shells before adding a large dependency graph.
5. Add camera controls and route inspection; then implement the local session.

## Engineering checks

Follow [coding standard](docs/CODING_STANDARD.md) and run `./tools/check.sh` before
commits. Maintained source files are checked against the 500-line limit; map
state, rendering, panels, assets, slice filtering and tests have separate modules.
Clippy denies warnings. The same command is configured for GitHub CI.
