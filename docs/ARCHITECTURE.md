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
  changes. Offline play advances ticks only on successful player movement;
  future AI acts on these ticks. Shared multiplayer will collect player
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

`game-server/src/offline.rs` dispatches movement to host resolution in movement.rs.
It publishes a complete fleet location/fuel/tick/market change atomically. Nonmovement commands never advance time. `ContinueTravel` is an
offline-only action for resuming journeys saved by older builds. The client has
no wall-clock simulation scheduler. A multiplayer adapter must keep host `advance`
private and resolve authenticated players' collected orders at turn boundaries.
Networking, per-player accounts, ordering, reconnect and retry replay remain open.

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
The player does not press End Turn in single-player; movement consumes ticks.

`game-server/src/save.rs` owns schema-3 authoritative JSON, validation, atomic
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
its system ID in Docked; planetary markets are next work.
Ship catalog definitions come from config/ships.json, embedded as a fallback.
Schema-1 migration moves the old global cargo/upgrade into its existing ship.

## Movement spaces and host resolution

`game-core::movement` separates configuration, integer geometry and planning.
Galaxy anchors are integer ly; local offsets and absolute SpacePosition use km.
The local cube exit clips a ray to its first face; entry projects the approach to
its near face. Crossing an intermediate system never intercepts a deep-space move.
Body arrivals attach to an orbit/berth; local stellar exclusion blocks invalid
segments. Body collision volumes and planetary landing physics remain unmodeled.

Movement previews validate all fleet members. Inputs contain a target and revision,
never an executable plan. Offline host resolution rebuilds the plan on current
state, resolves movement ticks, updates markets and publishes one candidate session.
Ordinary handle_as rejects immediate Move; a future multiplayer turn resolver
must collect orders before invoking host resolution. It cannot expose that host
entry point directly as a remote command handler. See MOVEMENT.md for current costs.
