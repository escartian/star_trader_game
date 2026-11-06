# Native Rust implementation roadmap

This is escartian's rewrite of their own [Star Trader game](https://github.com/escartian/star_trader_game#star-trader-game).
The original README was reviewed on 2026-11-05. Its fleet management, planetary
commerce, combat, factions and mission ambitions define the destination. The
current trading loop is an implementation foundation, not the entire game.
See [rewrite parity](docs/REWRITE_PARITY.md) for original claims versus current evidence.

## Decisions carried forward

- Pure Rust native client and authoritative Rust session/server; no web frontend.
  Single-player runs both in one process: `cargo run -p game-client`.
- Integer Cartesian coordinates and configurable cubic boundaries. Galaxy units
  remain **light-years**; local views use **AU**. Authoritative system positions
  remain integer kilometers, converted explicitly for display/input.
  Separate coordinate types and explicit checked conversions must prevent mixing
  spaces. The old generic ±100 local units are not interpreted as ±100 km.
- All campaign play uses explicit **navigation runs**. Plot courses for multiple
  fleets, review readiness, then **Execute navigation run** to advance all fleets
  and economies together. Unassigned fleets are selected/highlighted and require
  a course or explicit Hold. Long journeys span runs; contacts pause individual
  fleets. This replaces the earlier movement-triggered single-player clock.
- Working design direction: Heroes of Might and Magic meets Star Trader, with
  Civ-style fleet readiness and FTL-style encounters. Multiplayer will add shared
  player readiness around the same discrete, deterministic resolver.
- Backward save compatibility is not required during prototyping; breaking changes
  may require a fresh session. Keep current-format validation and atomic persistence.
- JSON for definitions and player/world saves. One in-memory authority owns live
  state; the original REST routes, per-model file reads and TTL caches are replaced.
- Modular readable code, at most 500 lines per source file. Rustfmt, strict Clippy,
  source-size checks and behavior tests run through `./tools/check.sh`.
- Mobile builds/device work are deferred. Desktop implementation proceeds while
  manual interaction, lifecycle and playtest gaps remain explicitly open.

## 1. Foundation and native presentation

- [x] Separate core, protocol, server library and native client crates.
- [x] In-process offline session with validated commands, revisions and events.
- [x] Seeded 3D galaxy with reachable starting markets; distinct ly/km types.
- [x] Orbit/pan/zoom, focus, presets, overlap chooser and double-click inspection.
- [x] Starfield/nebula background, star-type colors/glows/sizes, local system view.
- [x] Blender-authored celestial/station sprites and schematic moon separation.
- [x] Stock-sensitive commerce, per-ship holds, refuel and cargo upgrade.
- [x] Atomic JSON saves, backup recovery, slots, settings and trader onboarding.
- [x] Shared navigation-run planning/resolution, with no real-time scheduler.
- [ ] Manual desktop navigation/DPI/lifecycle acceptance and pacing study.

**Acceptance:** rejected actions preserve state; a navigation run advances the world once;
load restores existing state. Automated checks cover these rules. Window/device
acceptance is separate; see [platform status](docs/PLATFORM_STATUS.md).

## 2. Owned ships and fleets — implemented foundation

- [x] Stable ship/fleet IDs, per-ship cargo/fuel/capacity and data-driven ship classes.
- [x] Purchase freighter/scout/patrol hulls; select the ship used for commerce.
- [x] Split a fleet into nonempty groups; combine co-located owned fleets.
- [x] Transfer cargo between co-located owned ships without creating goods.
- [x] Travel the selected fleet atomically; check every member's fuel/range.
- [x] Save fleet membership and migrate prior single-ship saves without reset.
- [x] Original Blender ship sources/sprites and docked ship markers in system view.
- [x] Per-planet finite shipyard inventory, hull buying/selling and fleet renaming.
- [x] Body, local waypoint and deep-space locations, with ship markers tethered
  beside visited bodies/stations or placed at their deep-space coordinates.

**Acceptance:** split, move independently, rendezvous and merge; preserve each
ship's cargo, fuel and identity through save/load. These flows have behavior tests.
Planet/station shipyards have finite inventory and treasury-funded sales. Patrol
hulls have no combat yet; customization and used-hull loadouts remain later work.

## 3. Restore the original movement and planetary world model — in progress

- [x] Configurable galaxy/system cubic bounds in ly/km; extents contain generated
  bodies. Checked coordinates, edge/corner exits and exact range boundary tests.
- [x] Explicit fleet local/body/deep-space position, shared by its members.
- [x] Local movement to a waypoint/body, station docking and stellar exclusion.
- [ ] Planet/moon collision volumes and more detailed proximity/landing rules.
- [x] Exit at the first crossed cube face and place the fleet just outside it.
  Checked coordinate conversion preserves kilometer offsets in deep space.
- [x] Deep-space linear movement within galaxy bounds.
- [x] Explicit system entry by ID or exact coordinates, at the near cube face.
  Passing a system along a path never enters it.
- [x] Shared authoritative previews/commits, whole-fleet fuel/range checks,
  saved course durations/events, plotted orders and location-aware ship markers.
- [x] Save schema 3 and migration from schemas 1/2; resume older active journeys.
- [x] Planet economy type, production specialization, biome and danger rating
  (VerySafe through Insidious); resource market, ship market and local credits.
- [x] Add independent planet markets alongside stations, including buyer funds
  and a migration that preserves balances rather than silently generating wealth.

**Movement acceptance:** leave a body, exit its system, move through deep space,
enter another system and dock at its station. Behavior tests cover these stages
and save/load at each point; UI confirmation has an egui input regression test.
Inspection never moves the fleet. GPU previews are visually checked separately
from OS window/manual interaction. See [movement rules](docs/MOVEMENT.md).

Planet resource/ship markets, finite credit balances and metadata are implemented.
Schema 4 preserves old player assets/station stock, with explicit zero funding and
inventory for newly tracked accounts. Planetary commerce and hull transactions
have conservation/rejection/save tests. See [planet economies](docs/PLANET_ECONOMY.md).
Biome/danger are descriptive until encounters use them. Detailed collision and
landing volumes remain open, independently of the following gameplay work.

## 3a. Navigation runs and encounters — implemented foundation

- [x] Plot/replace/cancel one course per fleet without advancing time or charging fuel.
- [x] Fleet readiness review, selection/map focus and explicit Hold for unassigned fleets.
- [x] Shared atomic run resolution, parallel voyage progress and one fuel charge.
- [x] Per-fleet encounter chance, salvage/distress choices and persisted random state.
- [x] Schema 5: queued/active courses, holds, contacts, run/rescue counters and RNG.
- [x] Reject stale execution/reward retries and inconsistent saved orders.
- [x] Automatic destination itineraries: safe local approach, system departure,
  entry and body arrival, with saved remaining legs.
- [ ] Persistent standby, manually authored itineraries and movement/encounter balance.
- [ ] Richer authored contacts, risk factors, inspections, combat and faction outcomes.

See [navigation runs](docs/NAVIGATION_RUNS.md). The run itself is the visible
campaign advance action; the interface does not call it End Turn.

## 3b. Map fleet command — implemented foundation

- [x] Clickable fleet symbols and roster cards in inner/outer views.
- [x] Click a body/system and plot a complete route from an inline destination card.
- [x] Coordinate tools hidden under advanced controls; no numeric input required.
- [x] Departure marker, view-only breadcrumb and arrival/exit map following.
- [x] Stellar-core path rejection, safe local detours and symbol clearance around stars.
- [x] Sci-fi native command-console styling, route feedback and NPC traffic display.
- [x] Context-preserving arrival/transit framing without a system boundary overlay;
  free-space fleet anchors, neon-green gunmetal panels, action icons and image tiles.

See [map controls](docs/MAP_CONTROLS.md). Physical mouse/window/DPI acceptance
remains separate from GPU captures and egui input regression tests.

**Next:** ship combat statistics/loadouts and fleet combat, building on the first
NPC traders and current navigation/encounter foundations. Combat is not implemented.

## 4. NPC traders and fleet combat

Restore the original trader/personality direction before extending the economy.
Combat is the original README's first prioritized expansion.

- [x] Initial cautious/opportunist NPC traders with route choice, fuel-aware
  profitability, real cargo/wallets, finite-market trades and navigation-run AI.
- [ ] Expanded NPC personalities, interactions, encounters and faction affiliations.
- [ ] Ship type/size/engine, HP, shields, armor and weapon/loadout definitions.
- [ ] Carry forward Photon Singularity Beam, Quantum Entanglement Torpedo, Neutron
  Beam, Graviton Pulse and Magnetic Resonance Disruptor as balanceable JSON content.
- [ ] Authoritative fleet encounters/combat, target selection, damage/destruction,
  retreat, salvage/rewards and saved results; readable native combat report.
- [ ] Deterministic seeded resolution and explicit hostility/engagement rules.

**Acceptance:** two fleets resolve combat reproducibly; damaged/destroyed ships,
cargo and rewards remain consistent. NPC actions will resolve at navigation-run boundaries.

## 5. Factions and ship management

- [ ] Federation, Empire, Republic and Alliance with influence and relationships.
- [ ] Influence affects NPC fleets, territory and trade access/opportunities.
- [ ] Reputation, law/inspections, wanted status and police response.
- [ ] Repairs, maintenance, detailed ship statistics, equipment and customization.
- [ ] Fleet summaries and batch management that remain readable as ownership grows.

**Acceptance:** allegiance, equipment and damage affect actionable decisions;
repairs, access and reputation changes persist and cannot duplicate credits/items.

## 6. Missions and player specializations

The current first-delivery guide is trader onboarding, not a mission engine.
Build common objective/deadline/reward rules after movement/combat foundations.

- [ ] Authoritative saved contract lifecycle with run/tick deadlines and JSON content.
- [ ] Transport/logistics, combat/escort, exploration/anomalies, bounty hunting and
  piracy of goods/people, matching the original README's mission categories.
- [ ] Trader, spy/espionage, soldier and pirate routes with appropriate onboarding.
- [ ] Mixable specializations; shared control/save introduction, route-specific goals.
- [ ] Candidate additions: explorer/scout, prospector/salvager, smuggler and
  diplomat/fixer. Explorer and prospector are the preferred first additions.

**Acceptance:** success/failure/rewards resolve once, survive saves, and use the
same fleet/economy/combat/legal state. Trading is not mandatory for every route.
See [specializations](docs/PLAYER_SPECIALIZATIONS.md).

## 7. Shared multiplayer turns

- [ ] Versioned transport, serialization, authenticated per-player accounts.
- [ ] Dedicated server loop with ordered actions and explicit turn collection.
- [ ] Request IDs, duplicate response replay, stale revisions and snapshot resync.
- [ ] Reconnect/restart/disconnection rules and hidden-state filtering.
- [ ] Test competing purchases, ownership, fleet actions and turn determinism.

**Acceptance:** two clients share one world without duplicate purchases/actions;
clients cannot advance host time or impersonate another player. The current
protocol boundary anticipates this; multiplayer itself is not implemented.

## 8. Desktop release, then mobile

- [ ] Observe 5–10 players completing their preferred route; improve navigation,
  tradeoffs and repetitive fleet management using evidence.
- [ ] Package Windows/macOS/Linux; add accessibility, inputs, audio and feedback.
- [ ] Complete phone/tablet gestures, safe areas, lifecycle and real-device profiling.
- [ ] Mobile packaging/signing and full-session offline/connected acceptance.

Potential later scope from the original: enhanced faction AI/economy/generation,
alien reputation, colonization, planetary development/events/diplomacy. Binary
stars, multiple galaxies, galactic motion and orbital mechanics remain unlikely.
Original web-save import is separate from migrations of this Rust rewrite's JSON;
no compatibility claim is made without inspecting and testing actual old saves.
