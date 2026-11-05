# Star Trader — native Rust rewrite

A space trading and fleet management game in a genuinely three-dimensional universe,
presented through a readable map and native management screens.

**No game engine, browser, webview, or JavaScript frontend.** Use focused libraries;
do not build a general-purpose engine as part of the game.

## Current status

The first native desktop prototype implements an interactive orthographic star map
and a scrollable market-layout fixture. The client starts an authoritative local
session in the same process. Trading, travel, saves, and networking are not yet
implemented; displayed market prices are sample data.

## Run

Install a current stable Rust toolchain with edition 2024 support.

```sh
# One command starts the native client AND its in-process offline session:
cargo run -p game-client
# Check offline session startup without a window:
cargo run -p game-client -- --headless
# Dedicated-session fixture (prints and exits; no network listener yet):
cargo run -p game-server
cargo test --workspace --offline
cargo fmt --all --check
cargo check --workspace --all-targets --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
```

## Workspace

| Crate | Responsibility |
| --- | --- |
| `game-core` | World data, economy, travel, combat, validated simulation rules |
| `game-protocol` | Commands, responses, versioning, client-visible snapshots |
| `game-server` | Authoritative sessions, transport, saves, dedicated server executable |
| `game-client` | Window, map projection, UI, input, audio, connection state |

Single-player runs the reusable `game-server::Session` library inside the client.
No second terminal or background server is required. A remote transport is planned
for multiplayer; the dedicated executable currently only prints a snapshot.

Start with [ROADMAP.md](ROADMAP.md), then [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
and [docs/MAP_DESIGN.md](docs/MAP_DESIGN.md).

## Library stack

The prototype uses `winit` for window/input/lifecycle, `wgpu` for GPU rendering, `egui` with
`egui-winit` / `egui-wgpu` for panels and text, and `glam` for camera math. This is a
collection of libraries, not an engine; we own the small event loop and map renderer.
The initial renderer draws projected star markers and route lines, not 3D ships or planets.

The initial compatible set is pinned in the client manifest and Cargo.lock.
See [platform status](docs/PLATFORM_STATUS.md) for validation limits. This choice minimizes engine
machinery, not necessarily compiled binary size or build time.

Add `serde` / `serde_json` for versioned data, a seeded RNG for generation, and
`tracing` when those milestones require them. Use a single async runtime and a
WebSocket transport only when remote sessions are implemented. Do not pull in an
ECS, physics, scripting, or asset pipeline framework by default.

Desktop comes first, with an early Android/iOS build and lifecycle check. Mobile
support is a milestone to verify on devices, not an assumed automatic port.

Official references reviewed when preparing this plan:

- [winit](https://github.com/rust-windowing/winit)
- [wgpu](https://github.com/gfx-rs/wgpu)
- [egui](https://github.com/emilk/egui)
- [glam](https://github.com/bitshifter/glam-rs)

## Relationship to the original

Design reference: <https://github.com/escartian/star_trader_game>.
This scaffold does not copy original source or assets. Reuse domain ideas after
review, while keeping I/O and framework dependencies out of simulation rules.
Save compatibility with the original is not an initial requirement.

Working rules: [first session](docs/FIRST_SESSION.md). Map controls are shown in
the client; click/tap selects, drag orbits, middle drag pans, and the slider or
wheel zooms. Reset recovers the initial view. Mobile feasibility remains open.

The map now has a nebula/starfield background and glowing, type-colored stars
with different symbolic sizes. Use **Focus selection** (or F) to center a target,
**Pan tool** to pan by dragging, and **Systems & star legend** to browse types.
**Overlap test** and **Dense test** exercise navigation without changing the local
session. Overlapping targets appear in a dedicated chooser; ordinary labels are
reduced where they collide. Select a star and choose **Open selected system** to inspect its planets and
trade station. The local map uses kilometers and has its own camera. **← Galaxy**
or Escape returns to the same galaxy view and selection. Local destinations are
fixed fixtures; playable travel and docking are still planned.
