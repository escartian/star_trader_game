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
The session handles galaxy/session snapshots, travel previews and departures.
The host advances discrete ticks and emits arrival events. The dedicated server binary is
a thin remote adapter. Revisit crate extraction only when this creates a real need.

## Simulation

- Commands express intent: travel, buy, sell, accept contract. The session validates
  them and applies changes together, then emits events and an updated snapshot.
- Use integer minor units for money, checked arithmetic and explicit rounding rules.
- Use stable IDs; names and list positions are presentation, not identity.
- Keep galaxy and local coordinates as different types. Record units explicitly.
- Rules use discrete ticks, with no real-time physics. The authority owns state
  changes. Offline play schedules ticks from a monotonic host clock and feels
  continuous; future AI can act on ticks. Shared multiplayer will collect player
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

Use the original as a reference for resources, ships and world generation. Port
one feature at a time after defining its rules; do not transplant route handlers,
global state, file-backed model methods or UI geometry calculations into the core.
Keep one source of truth for travel distance/cost in simulation; clients can ask
for previews. Existing save import and source/assets reuse need a separate decision.

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

The offline host adapter is `game-client/src/session.rs`. It schedules ticks at one
second per in-world hour and pauses its monotonic clock at suspend/resume. While
idle it catches up on the next input without continuous redraws; travel schedules
redraws to report progress. A multiplayer adapter must never expose the host's
`advance` method as an untrusted command. Actual networking, multiple-player state,
authentication, turn ordering, reconnect/resync and idempotent responses remain open.
