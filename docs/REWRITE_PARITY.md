# Original-game parity review

Source: escartian's [Star Trader README](https://github.com/escartian/star_trader_game#star-trader-game),
reviewed 2026-11-05. The project owner confirms both repositories are theirs.
Existing code/assets may be reused after checking fit; this is a native rewrite.
This review analyzes README descriptions, not a runtime audit of the old game.
Its “implemented” and “future” sections are tracked separately below.

| Area | Original README | Native Rust status / next work |
| --- | --- | --- |
| Presentation | React/TypeScript + Rocket | Native Rust winit/wgpu/egui; in-process server for solo play |
| Galaxy | Procedural integer 3D world, cubic boundaries | Seeded world, ly coordinates and native camera; configurable bounds pending |
| System movement | Local, exit, deep space and explicit entry | km body coordinates and inspection; travel currently abstracts system-to-system movement |
| Fleets | Owned ships, fleet details and movement under development | Purchases, per-ship cargo/fuel, split/merge, transfers, fleet travel and saves |
| Planet economy | Per-planet resources/ships, production, biome, danger, credits | System-port stock-sensitive markets; planetary model and market funds pending |
| NPCs | Traders and personality under development | No NPC trader simulation yet |
| Combat | Backend concepts; first future priority | Pending ship HP/shield/armor/loadout and fleet resolution |
| Factions | Federation, Empire, Republic, Alliance; second priority | Pending influence, relationships, territories and access |
| Ship management | Type/size/engine/stats; third priority | Three cargo/fuel/range classes; repair, selling and equipment pending |
| Missions | Fourth priority: transport, combat, anomalies, bounty, piracy | Trader guide only; shared mission lifecycle pending |
| Persistence | JSON per-game files and aggregate saves | Validated atomic JSON snapshots, backup recovery; rewrite schema 1→2 migration |
| Multiplayer | Potential future feature | Authoritative command boundary present; transport/accounts/turn resolver pending |

The owner further specified movement-triggered ticks for single-player, explicit
multiplayer turns, modular source files under 500 lines, Blender ship/celestial
art, and spy/soldier/pirate progression. Galaxy light-years and local kilometers
are retained instead of the README's generic units; cubic bounds still apply.

The original TTL caches, REST response wrappers and frontend geometry are not
domain requirements. The rewrite uses an in-memory session, typed commands and
snapshots, with filesystem access isolated in persistence/startup adapters.
The original movement examples need explicit unit conversions and boundary tests
before implementation. No automatic entry merely because a path crosses a system.

Next work restores local/deep-space movement and planetary economies, followed by
combat, factions/ship management and missions in the original priority order.
No mobile milestone blocks this desktop implementation work.
