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

The client owns the camera, projections, picking, label placement, panels and
transient animation. GPU calculations use coordinates relative to the camera or
focused system; simulation coordinates remain independent of GPU precision.
Draw star markers and route lines, with regular UI panels for markets/fleets.
No ECS, physics world, model importer or general scene framework is required.

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

## Migration from the original

The project owner is escartian, author of the original repository. Preserve its
fleet, planetary economy, combat, faction and mission direction; reuse source or
assets where appropriate. The README parity review is in REWRITE_PARITY.md.
Port rules into the core rather than transplanting web handlers or TTL caches.
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
Players plot courses and Execute navigation run; plotting alone consumes no ticks.

`game-server/src/save.rs` owns schema-5 authoritative JSON, validation, atomic
replacement and a previous-good backup. Saves contain the world/player/economy
and their active definitions; continuing does not regenerate markets or apply
changed startup definitions. The client replaces its session only after load
succeeds and refreshes map data. Close/suspend hooks save at safe
lifecycle points. A failed close-save keeps the window open. See DATA_FORMATS.md.

Blender is a build-time authoring tool. Static transparent sprites are embedded
and preloaded once per egui context, with shared texture handles reused across
system views. A new graphics context receives new handles. Stars retain procedural
halos; tiny galaxy markers remain geometry. No runtime model importer, animation
asset pipeline or additional engine framework is introduced. Offscreen wgpu captures
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
Ship catalog definitions come from config/ships.json and weapons from
config/combat.json, both embedded as fallbacks. `game-core::combat` validates them.
Schema-1 migration moves the old global cargo/upgrade into its existing ship.

## Movement spaces and host resolution

`game-core::movement` separates configuration, integer geometry and planning.
Galaxy anchors are integer ly; local offsets and absolute SpacePosition use km.
The local cube exit clips a ray to its first face; entry projects the approach to
its near face. Crossing an intermediate system never intercepts a deep-space move.
Body arrivals attach to an orbit/berth; local stellar exclusion blocks invalid
segments. Body collision volumes and planetary landing physics remain unmodeled.

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

Schema-4 migration keeps existing station goods/prices and player state. Newly
introduced planet stock and previously untracked market cash/hulls start empty;
loading never seeds free funds or duplicates legacy stock. New sessions alone
allocate configured budgets/inventory. See PLANET_ECONOMY.md for this distinction.

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
in stable ID order. Existing commerce rules transfer payments to/from the same
market treasuries; the temporary trader-account adapter restores the player wallet
on both success and rejection before publishing the candidate session.
