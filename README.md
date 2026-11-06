# Star Trader — native Rust

A trading and fleet-management game in a three-dimensional universe, presented
through a readable native map. No game engine, browser, webview or JavaScript.

## Current prototype

- Galaxy navigation: orbit/pan/zoom, presets, focus, overlap chooser,
  double-click system inspection, and separate local views with planets/stations.
- Blender-authored star, planet, moon, asteroid station and ship models, baked to
  transparent sprites. Shared textures preload once; halos/selection stay procedural.
- Authoritative local/body movement, cubic system exits, deep-space travel and
  explicit system entry, with whole-fleet fuel/range checks and shared navigation runs.
- Per-planet and station markets with finite funds/hull stock, planet economy/biome/
  danger/specialization, stock-sensitive prices and refuelling,
  known-market comparisons and a purchasable cargo-hold upgrade.
- Fleet course planning, readiness highlighting, Hold orders, parallel navigation
  runs and individual salvage/distress encounters.
- Seeded starts, a first-delivery guide, session logs and
  three JSON save slots with validation, atomic replacement and backup recovery.

Owned ships support buying/selling, individual holds, fleet split/merge, renaming
and cargo transfers.
Multiplayer, discovery, combat and contracts are not implemented.
Mobile build work is deferred at the user's request. Desktop interaction, pacing
and device lifecycle still need manual/playtest evidence; see [platform status](docs/PLATFORM_STATUS.md).

## Run

Use a current stable Rust toolchain with edition 2024 support.

```sh
cargo run -p game-client
# Start fresh in another slot with a reproducible seed:
cargo run -p game-client -- --new --seed 42 --save data/slot-2.json
# Explicitly resume slot 1:
cargo run -p game-client -- --continue
# Export a complete initial world/player save as JSON without a window:
cargo run -p game-client -- --new --headless > /tmp/star-trader-save.json
# Run all engineering checks:
./tools/check.sh
```

The initial path is `data/slot-1.json`. Later launches remember the selected save
via `data/settings.json`; `--save PATH` explicitly selects another file. Launch
resumes that file if it exists, including backup recovery; otherwise it starts a
new session. To resume another UI slot, select it and choose Continue. **Session / saves** provides New,
Save, Continue and slot selection. Autosave runs on
close/suspend and writes to the selected slot. The previous valid file becomes
its `.bak`; a failed close-save keeps the window open with an error message.
Save slots live under ignored `data/`, keeping personal progress out of Git.

Changing the slot changes the autosave destination; it does not load that slot.
Use Continue to load it. Closing or starting fresh in a selected slot can replace
that slot, with its previous valid snapshot retained as backup.

Open **Fleets / shipyard** to buy ships, split checked members, merge co-located
fleets or transfer cargo. Travel moves every ship in the selected fleet and checks
each member’s fuel/range. Ships appear beside visited bodies/trade ports and at their deep-space positions.
Rocky/gas planets have markets at orbital berths; detailed landing/collision
volumes remain planned.

Local-system distances, boundaries and waypoint inputs use **AU**. Galaxy distances
remain light-years; saved simulation coordinates stay in integer kilometers.
Backward save compatibility is not a requirement during prototyping, so future
breaking versions may require a fresh session.

## First delivery

At Sol, open **Market / refuel**, set quantity to 20, and buy food. Open
**Navigation**, choose **+X** under Exit toward, and **Plot course**. In deep space, choose
**Enter Meridian** and plot its course. Double-click Meridian to inspect it, select its
trade port, choose **Preview docking at trade port**, and plot its course. Sell your food,
refuel, and compare ore or machinery for the return trip. Work toward the hold
upgrade. Use **Execute navigation run** to progress courses between these stages. Longer
journeys continue across runs. Double-clicking a star only inspects it.

Local/body destinations and coordinate waypoints use the same preview/confirm
flow. Routes through the star reject; use a waypoint around it. See the complete
[movement controls and costs](docs/MOVEMENT.md). Mining and orbital physics remain
unimplemented. Markets belong to individual planets and primary trade stations.

**Plot course** never advances time or spends fuel. Give each fleet a course or
**Hold position**, then **Execute navigation run** to advance all fleets and markets
together. Missing orders select/highlight the fleet first. Each travelling fleet
can encounter salvage or a distress signal; resolve contacts in Navigation.
Random state and choices persist. See [navigation runs](docs/NAVIGATION_RUNS.md).

The game is turn based internally, with navigation runs as the campaign language.
Single-player no longer advances the whole world when one fleet moves. Multiplayer
will add shared player readiness; networking/authentication remain unimplemented.

Visit a planet, then open **Market / refuel** to trade there. **Fleets / shipyard**
shows local hull stock, treasury-funded sale offers and fleet naming controls.
Markets can run out of money; shipyards can run out of hulls. See
[planet economies](docs/PLANET_ECONOMY.md).

Older saves preserve player assets and station goods; new treasuries and shipyards
start empty, and new planetary stock grows through navigation runs. A new game in
another save slot uses the configured starting funds and hull inventory.

## JSON economy and player data

[config/planets.json](config/planets.json) defines planet profiles, market templates,
starting budgets and hull stock. [config/ships.json](config/ships.json) defines purchasable hull prices, holds, fuel
and range. [config/economy.json](config/economy.json) defines starting credits/cargo capacity,
service/upgrade costs, commodity prices and market production/consumption. Money
uses integer hundredths of a credit: `100000` means `1000.00` credits.
The explicit commodity order identifies array entries. Custom definitions are
validated before a new session starts:

```sh
cargo run -p game-client -- --new --economy config/economy.json
```

New games load the project JSON when present; packaged builds have an embedded
fallback. Saved sessions include their definitions and all current player, ship,
journey, tick, cargo and market state. Continuing a save does not regenerate it
from the seed or apply newly edited economy definitions. The live authority stays
in memory; trades do not read or rewrite JSON files.

Save slot/path, active ship, guide visibility and autosave choices are remembered
in separate client-settings JSON. Session logs apply to the current run.

The current delivery guide is the **trader introduction**, not a generic questline.
Trader, spy (espionage), soldier (warfare) and pirate (robbery/police evasion)
progression are now explicit planned routes; see
[player specializations](docs/PLAYER_SPECIALIZATIONS.md). They are not playable yet.

See [JSON data and saves](docs/DATA_FORMATS.md) and [working rules](docs/FIRST_SESSION.md).

## Workspace and checks

| Crate | Responsibility |
| --- | --- |
| `game-core` | Data, deterministic travel/economic rules, seeded starts |
| `game-protocol` | Commands, typed errors, revisioned quotes, visible snapshots/events |
| `game-server` | Session authority, JSON save adapter, future dedicated transport |
| `game-client` | Native window, map/UI, shared art, offline action adapter and session controls |

Single-player hosts `game-server::Session` inside the client process. No separate
server is required. `cargo run -p game-server` is still a print-and-exit fixture;
it does not accept network connections.

The client uses pinned winit, wgpu, egui integrations and glam. Serde/serde_json
handle data; PNG decoding handles baked art. Source files are limited to 500
lines by `./tools/check.sh`, alongside rustfmt, strict Clippy and workspace tests.
See [coding standard](docs/CODING_STANDARD.md), [architecture](docs/ARCHITECTURE.md),
[map design](docs/MAP_DESIGN.md) and [roadmap](ROADMAP.md).

## Art and GPU captures

Editable sources and export commands are documented in [assets/README.md](assets/README.md).
Runtime sprites are static, screen-facing illustrations with symbolic sizes;
world positions/distances retain their own units. Baking improves the available
detail, but no rendering-performance advantage has been benchmarked. No animation
frame sets are preloaded; effects remain lightweight procedural overlays. Planet
rotation is deferred until surface rotation can keep lighting/shape consistent;
rotating a flat baked image would rotate its lighting too. Moon spacing is
exaggerated in the diagram for readability, while kilometer readings and saves
retain actual positions. Drawing, picking and Focus share that schematic layout.

Render the real UI to a GPU texture for visual review without a desktop window:

```sh
cargo run -p game-client -- --new --capture /tmp/system.png --capture-view system
cargo run -p game-client -- --new --capture /tmp/market.png --capture-view market
```

Example captures: [system](docs/previews/system.png), [market](docs/previews/market.png),
[fleets](docs/previews/fleets.png), [movement](docs/previews/navigation.png),
[system entry](docs/previews/entry.png), [ship near planet](docs/previews/orbit.png),
[planet market](docs/previews/planet-market.png), [planet shipyard](docs/previews/planet-shipyard.png),
[run review](docs/previews/run-review.png), [encounter](docs/previews/encounter.png), [save controls](docs/previews/saves.png). Capture views: `galaxy`, `system`, `market`, `fleets`, `saves`, `navigation`, `entry`, `deep-space`, `orbit`, `planet-market`, `planet-shipyard`, `run-review`, `encounter` (use `--seed 2`). Captures do not test window
resize/DPI, touch or device lifecycle. An available GPU adapter is required.

This is escartian’s native rewrite of their own [original game](https://github.com/escartian/star_trader_game).
The [parity review](docs/REWRITE_PARITY.md) and [roadmap](ROADMAP.md) carry its
fleet, planetary commerce, combat, faction and mission scope forward. Original
web-save import remains unimplemented.
