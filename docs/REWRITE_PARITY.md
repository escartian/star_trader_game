# Original-game parity review

Source: escartian's [Star Trader README](https://github.com/escartian/star_trader_game#star-trader-game),
reviewed 2026-11-05. The project owner confirms both repositories are theirs.
Existing code/assets may be reused after checking fit; this is a native rewrite.
This review analyzes README descriptions, not a runtime audit of the old game.
Its “implemented” and “future” sections are tracked separately below.

| Area | Original README | Native Rust status / next work |
| --- | --- | --- |
| Presentation | React/TypeScript + Rocket | Native Rust winit/wgpu/egui; in-process server for solo play |
| Galaxy | Procedural integer 3D world, cubic boundaries | Seeded world, ly coordinates, native camera and configurable cubic bounds |
| System movement | Local, exit, deep space and explicit entry | All four movement types, km local/absolute positions, explicit entry and station docking |
| Fleets | Owned ships, fleet details and movement under development | Purchases, per-ship cargo/fuel, split/merge, transfers, fleet travel and saves |
| Planet economy | Per-planet resources/ships, production, biome, danger, credits | Independent planet/station markets, profiles, production specializations, finite funds and hull stocks |
| NPCs | Traders and personality under development | No NPC trader simulation yet |
| Combat | Backend concepts; first future priority | Pending ship HP/shield/armor/loadout and fleet resolution |
| Factions | Federation, Empire, Republic, Alliance; second priority | Pending influence, relationships, territories and access |
| Ship management | Type/size/engine/stats; third priority | Three classes, hull buying/selling and fleet renaming; repair/equipment pending |
| Missions | Fourth priority: transport, combat, anomalies, bounty, piracy | Trader guide only; shared mission lifecycle pending |
| Persistence | JSON per-game files and aggregate saves | Validated atomic JSON snapshots, backup recovery; rewrite schemas 1–4→5 migration |
| Multiplayer | Potential future feature | Authoritative command boundary present; transport/accounts/turn resolver pending |

The owner initially specified movement-triggered ticks, then replaced that with
shared navigation runs for all campaign play, Civ-style fleet readiness and
FTL-style per-fleet encounters. Other requirements include modular source files under 500 lines, Blender ship/celestial
art, and spy/soldier/pirate progression. Galaxy light-years and local kilometers
are retained instead of the README's generic units; cubic bounds still apply.

The original TTL caches, REST response wrappers and frontend geometry are not
domain requirements. The rewrite uses an in-memory session, typed commands and
snapshots, with filesystem access isolated in persistence/startup adapters.
Movement now uses explicit checked unit conversions and tested cube intersections
instead of copying the original illustrative formulas. No automatic entry merely because a path crosses a system.

Next work restores NPC traders/personality, then combat, factions/ship management
and missions in the original priority order.
No mobile milestone blocks this desktop implementation work.
