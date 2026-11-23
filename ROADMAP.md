# Native Rust implementation roadmap

This is escartian's rewrite of their own [Star Trader game](https://github.com/escartian/star_trader_game#star-trader-game).
The rewrite now exceeds the original's feature set. Fleet management, planetary
commerce, combat, factions and missions define the destination.

**Planned rework:** a phone-first interface, flat play, hex battles with altitude
bands and several roads to victory. See [the rework plan](docs/PHONE_FIRST_REWORK.md)
for decisions and phases; it takes precedence over conflicting older map/battle notes.

## Decisions carried forward

- Desktop-first Rust client and authoritative Rust session/server. The browser
  preview compiles the same Rust client to WebAssembly; no JavaScript game
  frontend is introduced. Single-player runs both in one process:
  `cargo run -p game-client`.
- Integer Cartesian coordinates and configurable cubic boundaries. Galaxy units
  remain **light-years**; local views use **AU**. Authoritative system positions
  remain integer kilometers, converted explicitly for display/input.
  Separate coordinate types and explicit checked conversions must prevent mixing
  spaces. The old generic ±100 local units are not interpreted as ±100 km.
- All campaign play uses explicit **navigation runs**. Plot courses for multiple
  fleets, review readiness, then **Execute navigation run** to advance all fleets
  and economies together. Unassigned fleets are selected/highlighted and require
  a course or explicit Wait, which lasts until replaced by a course. Long
  journeys span runs; contacts pause individual fleets. **Advance** repeats runs
  until an event needs the player. This replaces the earlier movement-triggered
  single-player clock.
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
- [x] Initial browser pipeline: Trunk bundles the game client as HTML and WASM.
- [x] In-process offline session with validated commands, revisions and events.
- [x] Seeded flat galaxy with reachable starting markets; distinct ly/km types.
  New games offer 4–150 systems, a 20–95 ly radius and 6–15 total objects per
  system (60/70/8 by default), with varied stars, planets, moons and asteroids;
  seed 0 retains the fixed test fixture.
- [x] Orbit/pan/zoom, focus, presets, overlap chooser and double-click inspection.
- [x] Starfield/nebula background, star-type colors/glows/sizes, local system view.
- [x] Blender-authored celestial/station sprites and schematic moon separation.
- [x] Stock-sensitive commerce, per-ship holds, refuel and cargo upgrade.
- [x] Market quick actions: Max/All orders, Full refuel and best-margin hints.
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
- [x] Save fleet membership in the current schema and validate it on load.
- [x] Original Blender ship sources/sprites and docked ship markers in system view.
- [x] Per-planet finite shipyard inventory, hull buying/selling and fleet renaming.
- [x] Body, local waypoint and deep-space locations, with ship markers tethered
  beside visited bodies/stations or placed at their deep-space coordinates.

**Acceptance:** split, move independently, rendezvous and merge; preserve each
ship's cargo, fuel and identity through save/load. These flows have behavior tests.
Planet/station shipyards have finite inventory and treasury-funded sales. Patrol
hulls fight raiders (milestone 4); customization and used-hull loadouts remain later work.

## 3. Restore the original movement and planetary world model — implemented foundation

- [x] Configurable galaxy/system cubic bounds in ly/km; extents contain generated
  bodies. Checked coordinates, edge/corner exits and exact range boundary tests.
- [x] Explicit fleet local/body/deep-space position, shared by its members.
- [x] Local movement to a waypoint/body, station docking and stellar exclusion.
- [x] Map-scale collision volumes for every body; local legs and named routes
  avoid them. Landing on rocky planets, moons and asteroids (fuel by gravity,
  automatic lift-off on a new course) and ore mining while waiting landed.
- [x] Orbits: bodies circle the star (moons their planet) as a deterministic
  function of the tick; courses re-aim each leg and re-route around bodies that
  moved into the way; saves stay valid at every tick.
- [x] Exit at the first crossed cube face and place the fleet just outside it.
  Checked coordinate conversion preserves kilometer offsets in deep space.
- [x] Deep-space linear movement within galaxy bounds.
- [x] Explicit system entry by ID or exact coordinates, at the near cube face.
  Passing a system along a path never enters it.
- [x] Shared authoritative previews/commits, whole-fleet fuel/range checks,
  saved course durations/events, plotted orders and location-aware ship markers.
- [x] Current-format save validation and atomic recovery; schema 7 rejects older
  saves after the flat-map change.
- [x] Planet economy type, production specialization, biome and danger rating
  (VerySafe through Insidious); resource market, ship market and local credits.
- [x] Add independent planet markets alongside stations, including buyer funds
  and conservation checks.

**Movement acceptance:** leave a body, exit its system, move through deep space,
enter another system and dock at its station. Behavior tests cover these stages
and save/load at each point; UI confirmation has an egui input regression test.
Inspection never moves the fleet. GPU previews are visually checked separately
from OS window/manual interaction. See [movement rules](docs/MOVEMENT.md).

Planet resource/ship markets, finite credit balances and metadata are implemented.
Current-format saves preserve player assets and market stock. Planetary commerce and hull transactions
have conservation/rejection/save tests. See [planet economies](docs/PLANET_ECONOMY.md).
Biome/danger are descriptive until encounters use them. Bodies have map-scale
collision volumes and landing. New galaxies give bodies deterministic circular
orbits (moons around planets). Navigation plans use their positions at the current world tick.

## 3a. Navigation runs and encounters — implemented foundation

- [x] Plot/replace/cancel one course per fleet without advancing time or charging fuel.
- [x] Fleet readiness review, selection/map focus and explicit Wait for unassigned fleets.
- [x] Shared atomic run resolution, parallel voyage progress and one fuel charge.
- [x] Per-fleet encounter chance, salvage/distress choices and persisted random state.
- [x] Schema 5: queued/active courses, holds, contacts, run/rescue counters and RNG.
- [x] Reject stale execution/reward retries and inconsistent saved orders.
- [x] Automatic destination itineraries: safe local approach, system departure,
  entry and body arrival, with saved remaining legs.
- [x] Persistent standby: an explicit Wait lasts across runs until a new course.
- [x] Advance to next event: paced repeated runs that stop on contacts, arrivals
  needing orders, idle fleets or Stop; a clock button keeps single-run stepping.
- [x] Repeating trade routes between two markets: greedy most-profitable cargo,
  sell/refuel/buy at each stop, contacts as usual; an empty hold pauses the route
  until Resume, and failures end it.
- [ ] Manually authored itineraries and movement/encounter balance.
- [x] One flagship under direct command; secondary fleets on standing orders
  (Wait, trade route, patrol a system) that settle their own contacts.
- [x] Ten peaceful contact kinds (salvage, convoys, cargo pods, customs,
  authority escorts, anomalies, comets, probes, debris, distress) beside raiders.
- [x] New-game setup: galaxy size, starting credits, inflation, danger,
  campaign length and Trader goal; generated star and body names.
- [x] Progressive unlocks (phone-first phase 6): shipyard, trade routes,
  battles, landing and the Victory panel appear as play reaches them, each with
  a guide card.
- [ ] Faction outcomes for law contacts, branching authored story chains.

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
- [x] Run motion: ships and NPC traffic glide between pre- and post-run positions
  (display only), continuous across the runs of an Advance.

See [map controls](docs/MAP_CONTROLS.md). Physical mouse/window/DPI acceptance
remains separate from GPU captures and egui input regression tests.

**Campaign (phone-first phase 5):** roads to victory, score, campaign length,
game over, the Raider Haven and port charters are implemented; see
[campaign](docs/CAMPAIGN.md).

**Next:** shifting territory and faction war fleets, NPC trader faction
affiliations, mid-fight retreat and combat balance from playtests
([combat](docs/COMBAT.md)).

## 4. NPC traders and fleet combat

Restore the original trader/personality direction before extending the economy.
Combat is the original README's first prioritized expansion.

- [x] Initial cautious/opportunist NPC traders with route choice, fuel-aware
  profitability, real cargo/wallets, finite-market trades and navigation-run AI.
- [ ] Expanded NPC personalities, interactions, encounters and faction affiliations.
- [x] Ship type/size/engine, HP, shields, armor and weapon/loadout definitions
  (`config/ships.json` schema 2, current save schema 8; see [combat](docs/COMBAT.md)).
- [x] Carry forward Photon Singularity Beam, Quantum Entanglement Torpedo, Neutron
  Beam, Graviton Pulse and Magnetic Resonance Disruptor as balanceable JSON content
  (`config/combat.json`).
- [x] Authoritative raider encounters: Fight/Evade, focus-fire targeting,
  damage/destruction, salvage, saved results and a readable native combat report.
- [x] Deterministic seeded resolution (seed saved in the report), withdrawal and
  ceasefire disengagement; last-ship rule; paid hull repairs at markets.
- [x] Turn-based tactical rules: HoMM-style engine initiative, move, ranged
  fire, withdrawal from your own edge, raider AI and Auto-resolve.
- [x] Play battles turn by turn: saved battles in progress, move/fire/withdraw/
  auto-resolve commands. Battles run to a finish (no round limit).
- [x] Hex board (phone-first phase 4): boards of radius 4–6 by fleet size,
  movement points spent on steps, a fixed-tilt 3D board view, tap-to-aim then
  Fire, and a stacked phone layout. Schema 9 flattened it to one layer (the
  Low/Level/High bands are gone) and added seeded, mirrored terrain: asteroids
  that block movement and fire, debris that gives cover. Schema-7 and -8 saves
  auto-resolve an open battle on load.
- [x] Every hull drawn from a Blender glTF export (the scout decimated to 1,600
  triangles); animated moves and weapon beams; side strength, portraits, hover
  cards and move-range previews.
- [x] Morale: badly damaged AI ships and broken sides retreat and withdraw;
  fleeing raiders count as a victory. Battle boards grow with the fleets.
- [ ] A GPU mesh renderer with a depth buffer (the egui painter sorts faces).
- [x] Raider bounties by hull, paid once per battle for destroyed raiders.
- [x] NPC traders meet raiders off-screen (auto-resolved tactical battles):
  fought off, escaped by dumping cargo, or destroyed and refitted at home.
- [x] Roaming raider bands: one per system but home, hunting exposed fleets
  (docked or landed fleets are safe), patrolling dangerous planets, striking
  with their real ships, backing off after fights and returning when wiped out.
- [x] Hail NPC traders within reach of the flagship: let them go, demand cargo
  from an outgunned trader, or attack and plunder it (no bounty; acts of piracy
  are counted for the future wanted status).
- [x] Raider bands stalk laden NPC traders in their system: outgunned traders
  hand over half their cargo, others fight (auto-resolved); bands then lie low.
- [x] Anti-piracy gear and escorts: afterburners and point defense per ship, a
  hired patrol escort per fleet for 10 runs; NPC traders buy them from profits.
- [x] Raider bands cross systems: they chase a fleet that flees into the next
  system on its course (never home) and roam between systems, two at most per system.
- [x] Hostility rules for faction fleets: police patrols pursue fleets of a
  captain their faction wants (or holds Hostile) and otherwise engage raider
  bands hunting in their system ([factions](docs/FACTIONS.md#the-law-wanted-status-and-police)).

**Acceptance:** two fleets resolve combat reproducibly; damaged/destroyed ships,
cargo and rewards remain consistent. Raider fights meet this with behavior and
save tests. NPC actions will resolve at navigation-run boundaries.

## 5. Factions and ship management

- [x] Federation, Empire, Republic and Alliance with territory, influence (share
  of systems held) and alliances/rivalries, set up at new-game setup: factions
  in play, allegiance, territory share and classic or random relations
  ([factions](docs/FACTIONS.md)).
- [x] Standing per faction (−100 to 100, spreading to allies and rivals): piracy
  costs it, destroying raiders and selling cargo earn it; tiers set prices and
  hostile ports refuse trade.
- [x] Faction police: a patrol in every claimed system that visits its bodies,
  fights raider bands and is replaced when wiped out.
- [ ] Influence affects NPC fleets and territory over time; faction war fleets.
- [x] Wanted status from piracy and resisting arrest, fines (paid at a port or to
  a patrol), wanted ports refusing trade and police contacts (Comply, Fight, Evade).
- [ ] Faction inspections for contraband, bribes and bounty hunters.
- [ ] Repairs (full hull repair at markets exists), maintenance, detailed ship
  statistics, equipment and customization.
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
  Done so far: main menu, Settings (volume, fullscreen, interface scale,
  animations) and the desktop Long Haul music sequencer (also available in game
  when enabled), with a main-menu Songbook that lists parts, maps their transitions and selects them. Kenney CC0
  effects add UI clicks, battle shots and hit sounds,
  plus battle hit particles. Still open: broader sound coverage, other in-game tracks, browser audio,
  key rebinding and other accessibility options.
- [ ] Complete phone/tablet gestures, safe areas, lifecycle and real-device profiling.
- [ ] Mobile packaging/signing and full-session offline/connected acceptance.

Potential later scope from the original: enhanced faction AI/economy/generation,
alien reputation, colonization, planetary development/events/diplomacy. Binary
stars, multiple galaxies, galactic motion and orbital mechanics remain unlikely.
Original web-save import is separate from this Rust rewrite's JSON;
no compatibility claim is made without inspecting and testing actual old saves.
