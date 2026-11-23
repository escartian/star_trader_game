# Architecture and implementation constraints

## Dependency direction

```text
game-client  ──→ game-server (in-process offline session)
     │                │
     └────────→ game-protocol ──→ game-core
game-server  ──→ game-protocol ──→ game-core
```

Both applications may use core data types. Neither core nor protocol imports a
windowing, UI, rendering, runtime, networking, or storage framework. Protocol uses
explicit visible snapshots; never serialize the complete authoritative world by default.
The fixture currently shares `StarSystem` directly because it has no hidden state.

The server package exposes a session library that the client hosts in-process.
The session handles snapshots, travel, trading, refuelling and upgrades.
The host advances discrete ticks and emits events. The dedicated executable is
currently a print-and-exit fixture; a remote adapter is still planned. Revisit crate extraction only when this creates a real need.

## Simulation

- Commands express intent: travel, buy, sell, accept contract. The session validates
  them and applies changes together, then emits events and an updated snapshot.
- Use integer minor units for money, checked arithmetic and explicit rounding rules.
- Use stable IDs; names and list positions are presentation, not identity.
- Keep galaxy and local coordinates as different types. Record units explicitly.
- Rules use discrete ticks, with no real-time physics. The authority owns state
  changes. Campaign time advances only through shared navigation runs;
  fleet courses progress in parallel and future AI acts at these boundaries. Shared multiplayer will collect player
  actions and resolve explicit turns. Its order collection/resolution is pending.
  Clock advancement is a host method, deliberately absent from player commands.
- Seed random generation and persist its necessary state. Rendering never consumes
  simulation randomness. Do not assume floating-point bitwise determinism across devices.
- State mutation belongs to one session authority. No per-model file reads, global
  mutable current-game state, or TTL caches as the source of economic truth.

## Client and map

The desktop host is in `game-client/src/desktop.rs`; the initial browser host is
in `game-client/src/web.rs` and uses the same `Map` UI. Platform entry points
stay separate so browser storage and event-loop differences do not leak into
simulation/session rules.

The client owns the camera, projections, picking, label placement, panels and
transient animation. GPU calculations use coordinates relative to the camera or
focused system; simulation coordinates remain independent of GPU precision.
Draw star markers and route lines, with regular UI panels for markets/fleets.
No ECS, physics world or general scene framework is required. The client reads
only the embedded ship GLBs used by battle and NPC traffic.

Music is client I/O in `game-client/src/audio.rs`. The map decides which track
should play (the main menu theme, or in game when the player enables it) and
the gain from the Settings sliders; `audio` owns the device through rodio
(desktop only; the browser build is silent). `main` opens the device and hands
it to the map, so tests and captures never touch audio hardware. Long Haul is
an original Strudel composition kept in `assets/source/music/`, played with
Strudel's synthesizers and FluidR3_GM soundfont instruments whose sample data is
kept alongside it. Its exported Ogg blocks (eight bars unless a block sets its
own length) and transition manifest are embedded in the desktop client.
The audio source chooses a weighted successor at each exact sample-frame boundary
and mixes the previous block's reverb tail under the new block. The exporter
balances the blocks' active RMS and records their final relative levels; a
louder successor eases in over one bar, while a quieter successor keeps its
exported level beneath the fading tail. It publishes the current block through
an atomic index; the main-menu Songbook reads that
index to show the playing part and its place in the track. Selecting a part
in the Songbook sends an atomic request to the audio source; it switches on a
stereo frame edge and crossfades for two seconds, then resumes weighted valid
transitions. Browser audio is still pending.
Kenney CC0 effects are embedded from `assets/effects/`; the same rodio output
mixes short UI and battle cues with music. Battle view state schedules each cue
at the matching beam start or hit time. Kenney particle sprites are loaded once
per egui context and tinted for shield, hull and destruction flashes. Effects
and their timing never change combat rules.

Library selection is provisional until the desktop/mobile spike passes. Keep
versions mutually compatible, pin resolved dependencies in Cargo.lock, and avoid
adding a second graphics backend or async runtime without a concrete need.

## Storage and transport

Start with versioned JSON snapshots through a save adapter. Validate before
replacing a session, write via a temporary file and atomic replacement, and keep
a recoverable previous snapshot. Use platform-provided writable app storage on
mobile rather than relying on the working directory.

For remote play, start with one framed, versioned WebSocket command/response
channel; Rust on both ends does not require a custom binary protocol. The session
resolves identity from the connection and checks all ownership, location, funds
and capacity rules. Request IDs support safe retry; state revisions expose stale
quotes. Persistence and transport errors must not leave partial transactions.

## Relationship to the original

The project owner is escartian, author of the original repository. Source or
assets may be reused where they fit, but rules belong in the core rather than
transplanted web handlers or TTL caches.
Keep galaxy light-years and local kilometers distinct, with cubic boundaries.
Original web-save import is separate, still unimplemented work.

## Verification

Use fmt/check/clippy for the scaffold. As behavior is implemented, add focused
tests for trade conservation and rejection, travel boundaries, seeded generation,
save recovery, authority checks, concurrent commands and retry deduplication.
Use actual device/manual checks for picking, readability and lifecycle behavior.
Do not treat a successful Rust cross-compilation as mobile usability validation.

## Initial travel authority

`game-core::travel` owns typed ship/player IDs, fuel, range, journey state and
integer distance/rounding. `game-server::Session` resolves known system positions,
checks the trusted caller's identity, builds quotes and applies commands atomically.
Departures include an expected revision. A stale or repeated departure is rejected
without charging fuel twice; this is not full network retry deduplication.
`handle_as` requires an authenticated identity from a future adapter; its numeric
argument is not authentication. The one-player fixture rejects other identities.

`game-server/src/offline.rs` is an in-process identity convenience adapter over the
same session commands. `game-core::navigation` owns plotted courses, fleet readiness,
run progress, encounters and deterministic random state. `game-server::navigation`
resolves a candidate world and advances economy once per run before publication.
There is no instant-movement or finish-old-journey command. Legacy voyages advance
on runs. Multiplayer readiness/authentication/transport remain separate pending work.

## Economy, JSON and shared art

`game-core::economy` owns fixed-unit credits/cargo, finite market stock, bounded
per-unit stock pricing and service/upgrade rules. JSON definitions live in
config/economy.json and are validated before startup. Core has data serialization
but no runtime filesystem access. Commerce commands include a state revision;
the authority recomputes prices and publishes a candidate state only after all
ownership/location/funds/capacity/stock checks pass. The current account is the
single-player fixture; multiple authenticated accounts are later multiplayer work.

Production/consumption have closed-form, bounded net changes per tick. Current
bulk advancement matches individual ticks. Future AI/encounters must preserve
intermediate event ordering rather than assume all tick systems can be batched.
Players plot courses and execute navigation runs; plotting alone consumes no ticks.
The client's Advance control only repeats the single-run command until an event
needs the player; it adds no authoritative rules.

`game-server/src/save.rs` owns schema-8 authoritative JSON, validation, atomic
replacement and a previous-good backup. Saves contain the world/player/economy
and their active definitions; continuing does not regenerate markets or apply
changed startup definitions. The client replaces its session only after load
succeeds and refreshes map data. Close/suspend hooks save at safe
lifecycle points. A failed close-save keeps the window open. See DATA_FORMATS.md.

Blender is a build-time authoring tool. Static transparent sprites are embedded
and preloaded once per egui context, with shared texture handles reused across
system views. A new graphics context receives new handles. Stars retain procedural
halos; tiny galaxy star markers remain geometry. Embedded ship GLBs are decoded
once and drawn as client-side triangles for battles and moving NPC traffic. No
general scene importer, animation asset pipeline or additional engine framework
is introduced. Offscreen wgpu captures
exercise the same UI code for visual review, separately from window/device checks.

Mobile build work is deferred by explicit user direction. Desktop implementation
continues while those platform/lifecycle gaps remain recorded in PLATFORM_STATUS.md.

## Fleet authority

TravelState owns ships, disjoint fleet membership and monotonic IDs. Each ship
owns cargo, hold upgrades, fuel and range. The current single player shares one
credit account. Split/merge/transfer/purchase operations validate on a candidate
session before publication; every member must pass travel checks before fuel is
charged. Fleet members share a location. Local-body and deep-space positions
are authoritative. Ships share fleet locations and are illustrated beside their
body anchors or at deep-space positions. The current primary trade station retains
its system ID in Docked; planetary Orbiting locations resolve a market by body.
Ship catalog definitions come from config/ships.json, weapons from
config/combat.json and factions from config/factions.json, all embedded as
fallbacks. Faction territory and your standing live in `Economy::factions`;
the host adjusts standing and wanted status after each command
(`game-server/src/standing.rs`) and core prices apply its terms
(`factions::terms`). Police patrols live in `NavigationState::police`; the host
runs `game-core::police::patrol_run` once per navigation run, after NPC traders,
routes and patrols, because pursuit reads `Economy::factions`. `game-core::combat` validates them.
Older save schemas are rejected during prototyping. Schema 8 is current; a
schema-7 save differs only in its battle format and is upgraded on load, its
open 3D-grid battle (if any) auto-resolved.

## Movement spaces and host resolution

`game-core::movement` separates configuration, integer geometry and planning.
Galaxy anchors are integer ly; local offsets and absolute SpacePosition use km.
The local cube exit clips a ray to its first face; entry projects the approach to
its near face. Crossing an intermediate system never intercepts a deep-space move.
Body arrivals attach to an orbit/berth; local stellar exclusion blocks invalid
segments, and every body has a map-scale collision sphere that local legs route
around (`movement::obstacles`). Bodies orbit (`orbits`): positions are a deterministic integer function of the
tick, placed by the server before every command and after every run, and by each
movement planner for its own state's tick. Landing (`movement::LandingRules`,
`navigation::landing`) is a location, not physics: immediate, fuel-priced, and
mining runs once per navigation run for waiting landed fleets.
`game-core::orbits` places circular, integer-defined star and moon orbits from
the world tick. The server stores their current positions after each time advance;
movement planning independently places a copy at its own tick. Saved courses
remember their planning tick for validation. A queued course is recalculated on
departure if time passed, and itinerary detours are checked again at each leg.
The client interpolates body positions only for drawing during a run glide.

Movement previews validate all fleet members. Inputs contain a target and revision,
never an executable plan. Plotting stores authoritative geometry without spending
fuel/time. On a shared run, departure charges fuel once and sets InTransit; every
order progresses by the fixed run quantum. Arrivals restore body/local/deep-space
locations. A future multiplayer coordinator must collect readiness before calling
shared resolution. See NAVIGATION_RUNS.md for encounter lifecycle and save rules.

## Planet economies and finite shipyards

Planet profiles live on local bodies; `game-core::planet` owns their JSON templates
and deterministic assignment. Economy owns markets keyed by `(SystemId, BodyId)`
with finite credit balances and hull inventory. Station flags are validated against
the saved world's primary station. A body ID alone never identifies a market.
A planet specialization affects tick production; biome/danger are saved descriptions
for future encounter logic.

Resource/service/hull transactions transfer credits between the player and local
market. Hull purchases consume stock, sales restock it and remove empty fleets;
all publish through candidate-session transactions. Fleet/ship IDs remain monotonic.
Core rules resolve ship location; the client cannot nominate a remote marketplace.

Saving and loading keep exact market treasuries, stock and player state. New
sessions allocate configured budgets and inventory.

Local-view distances and coordinate inputs now use AU through the client's units
adapter (149,597,870.7 km/AU). Core movement and saves retain integer kilometers;
galaxy distances remain light-years. Non-finite/out-of-range AU inputs reject.
Backward save compatibility is explicitly not a prototyping requirement. Keep
current-format validation and atomic saves; allow fresh-session resets for future
breaking schemas rather than making migration support a blocker.

## Map command, safe itineraries and initial traders

The client shares fleet marker geometry between paint and pick, reserves stellar
silhouettes when placing symbols, and groups fleet members into one selectable
symbol. View-only breadcrumbs and committed route transitions are separate.
Destination cards submit body/system intent; coordinate fields remain advanced UI.
A shared theme styles native panels in interactive and offscreen contexts.

Core destination planning builds a checked itinerary with visibility-graph local
detours around the star. Entry/exit legs remain explicit in simulation, but are
planned automatically from map destinations. Navigation stores remaining targets
and rebuilds each next leg at arrival. Save validation rechecks future legs and
remaining fuel. Deep-space segment clipping rejects known stellar-core crossings.

Economy owns separate NPC trader wallets and hulls. NPCs use isolated TravelState
instances and the same destination/navigation rules; their IDs occupy the reserved
upper half of the ship/fleet ID space. Player commands cannot authorize NPC hulls.
The run coordinator advances production once, then resolves NPC decisions/routes
in stable ID order, then services player trade routes (`game-core::routes`) in
fleet ID order inside the same candidate session. Existing commerce rules transfer payments to/from the same
market treasuries; the temporary trader-account adapter restores the player wallet
on both success and rejection before publishing the candidate session.
