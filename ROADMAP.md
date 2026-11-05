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
  remain **light-years**, system units **kilometers**, per the owner's preference.
  Separate coordinate types and explicit checked conversions must prevent mixing
  spaces. The old generic ±100 local units are not interpreted as ±100 km.
- Single-player advances discrete ticks **only on successful player movement**.
  Waiting, reading, trading, fleet management and rendering consume no time. No
  End Turn action or wall-clock catch-up. AI and markets act on movement ticks.
  Multiplayer will collect explicit shared turns around the same rules.
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
- [x] Action-driven single-player ticks, with no real-time scheduler.
- [ ] Manual desktop navigation/DPI/lifecycle acceptance and pacing study.

**Acceptance:** rejected actions preserve state; movement advances the world once;
load restores existing state. Automated checks cover these rules. Window/device
acceptance is separate; see [platform status](docs/PLATFORM_STATUS.md).

## 2. Owned ships and fleets — current implementation batch

- [x] Stable ship/fleet IDs, per-ship cargo/fuel/capacity and data-driven ship classes.
- [x] Purchase freighter/scout/patrol hulls; select the ship used for commerce.
- [x] Split a fleet into nonempty groups; combine co-located owned fleets.
- [x] Transfer cargo between co-located owned ships without creating goods.
- [x] Travel the selected fleet atomically; check every member's fuel/range.
- [x] Save fleet membership and migrate prior single-ship saves without reset.
- [x] Original Blender ship sources/sprites and docked ship markers in system view.
- [ ] Per-planet finite shipyard inventory, ship selling/trading and fleet renaming.
- [ ] Ship locations tied to a planet/station/deep-space position, rather than a
  system-level dock. Planet-orbit ship markers depend on that authoritative data.

**Acceptance:** split, move independently, rendezvous and merge; preserve each
ship's cargo, fuel and identity through save/load. These flows have behavior tests.
The current shipyard is a system-port catalog; patrol hulls have no combat yet.

## 3. Restore the original movement and planetary world model — next

- [ ] Add configurable galaxy/system cubic bounds in ly/km; choose kilometer
  extents that contain generated bodies. Validate edge/corner/overflow cases.
- [ ] Store fleet local or deep-space position explicitly, shared by its members.
- [ ] Local movement to a position or body within system bounds; validate docking
  and body proximity/collision rules without adding orbital physics.
- [ ] Exit a system by crossing its cube boundary along the requested direction.
  Compute a checked unit conversion; do not copy illustrative README formulas
  that combine galaxy and system units.
- [ ] Deep-space linear movement, bounded by galaxy dimensions.
- [ ] Enter only when explicitly targeting system coordinates. Passing a system
  along a route does not automatically intercept or enter it.
- [ ] Shared movement preview/commit rules, fuel/tick costs, path and arrival events.
- [ ] Planet economy type, production specialization, biome and danger rating
  (VerySafe through Insidious); resource market, ship market and local credits.
- [ ] Replace prototype system markets with planet markets, including buyer funds
  and a migration that preserves balances rather than silently generating wealth.

**Acceptance:** a fleet can leave a planet, exit a system, move through deep space,
explicitly enter another system and dock at a planet. All transitions survive
save/load and use movement ticks. UI inspection alone never moves a fleet.

## 4. NPC traders and fleet combat

Restore the original trader/personality direction before extending the economy.
Combat is the original README's first prioritized expansion.

- [ ] NPC traders with personality, route choice, conserved funds/cargo and tick AI.
- [ ] Ship type/size/engine, HP, shields, armor and weapon/loadout definitions.
- [ ] Carry forward Photon Singularity Beam, Quantum Entanglement Torpedo, Neutron
  Beam, Graviton Pulse and Magnetic Resonance Disruptor as balanceable JSON content.
- [ ] Authoritative fleet encounters/combat, target selection, damage/destruction,
  retreat, salvage/rewards and saved results; readable native combat report.
- [ ] Deterministic seeded resolution and explicit hostility/engagement rules.

**Acceptance:** two fleets resolve combat reproducibly; damaged/destroyed ships,
cargo and rewards remain consistent. NPC actions occur on simulation ticks.

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

- [ ] Authoritative saved contract lifecycle with tick deadlines and JSON content.
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
