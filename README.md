# Star Trader — Rust

A trading and fleet-management game on a flat space map, presented
through a readable native map. The desktop client is native Rust; an early
browser build compiles the same client to WebAssembly.

## Current prototype

- Click fleets on either map, select a planet/station/point of interest and plot
  the route from a destination card. Numeric controls are optional advanced tools.
- A phone-first layout: status strip, action bar (Fleets, Advance, Menu) and one
  context sheet; panels go full screen on narrow windows. Route feedback and live
  NPC traffic shown with ship models on the map.
- Galaxy navigation: drag to pan, pinch/wheel zoom, focus, overlap chooser,
  double-click system inspection, and separate local views with planets/stations.
- Blender-authored celestial and ship art. Map bodies and owned fleets use
  transparent sprites; moving NPCs and battles draw embedded ship GLBs. Shared
  textures preload once; halos and selection stay procedural.
- Authoritative local/body movement, cubic system exits, deep-space travel and
  explicit system entry, with whole-fleet fuel/range checks and shared navigation runs.
- Per-planet and station markets with finite funds/hull stock, planet economy/biome/
  danger/specialization, stock-sensitive prices and refuelling,
  known-market comparisons and a purchasable cargo-hold upgrade.
- Fleet course planning, readiness highlighting, persistent Wait orders, parallel navigation
  runs and individual salvage, distress and raider encounters.
- Planets and moons on deterministic orbits, with collision volumes that courses
  route around; landing on planets,
  moons and asteroids, and mining ore while a landed fleet waits.
- Repeating trade routes: a fleet loops between two markets, buying the most
  profitable cargo at each end, and still stops for contacts.
- Turn-based battles against raiders on a single-layer hex board with asteroids
  (impassable, block fire) and debris (cover)
  and 3D ship models, fought to a finish (engine initiative, movement, high
  ground, weapon ranges, withdrawal) that you command or auto-resolve, with a
  saved combat report, destroyed ships, salvage, raider bounties and paid hull
  repairs.
- Roaming raider bands in every system but home that hunt fleets in the open,
  chase fleets that flee into the next system and wander between systems;
  dock or land to hide, outrun them, flee home or fight them for the bounty.
- Four factions (Federation, Empire, Republic, Alliance) chosen at setup, with
  territory, alliances and rivalries; your standing with each sets prices at
  its ports, and piracy, raider hunting and trade move it (Menu → Factions).
  Piracy makes you wanted: faction police patrols stop your fleets (comply and
  pay the fine, fight or run) and their ports refuse you until you pay.
- Roads to victory (Menu → Victory): win as a Trader (10% of all galaxy wealth
  or three port charters) or a Soldier (destroy the Raider Haven), with a published score,
  an optional campaign length and game over when your last ship is lost. Port
  charters pay a share of every trade there and make shipping lanes raiders
  never strike. See [campaign](docs/CAMPAIGN.md).
- Seeded starts, a first-delivery guide, session logs and
  three JSON save slots with validation, atomic replacement and backup recovery.

Owned ships support buying/selling, individual holds, fleet split/merge, renaming
and cargo transfers.
Multiplayer, discovery, faction wars and contracts are not implemented. Combat is
limited to raiders, hailed NPC traders and faction police.
Mobile build work is deferred at the user's request. Desktop interaction, pacing
and device lifecycle still need manual/playtest evidence; see [platform status](docs/PLATFORM_STATUS.md).

## Run

Use a current stable Rust toolchain with edition 2024 support.

```sh
cargo run -p game-client
# Start a new game with a fresh 60-system map (the same starting ship and stats):
cargo run -p game-client -- --new --save data/slot-2.json
# Choose map size, span and objects per system (up to 150 systems, 15 objects):
cargo run -p game-client -- --new --systems 150 --radius 95 --objects 15 --save data/slot-2.json
# Start fresh in another slot with a reproducible seed:
cargo run -p game-client -- --new --seed 42 --save data/slot-2.json
# Explicitly resume slot 1:
cargo run -p game-client -- --continue
# Multi-fleet test save: four fleets (docked, planet orbit, moon orbit, underway).
# Built with ordinary commands plus a 25,000 cr test budget; reload with --continue:
cargo run -p game-client --features dev-tools -- --scenario multi-fleet --save data/multi-fleet.json
cargo run -p game-client -- --continue --save data/multi-fleet.json
# Seven ships against seven raiders, the battle open on your first turn
# (or pick sizes, 1 to 8 a side: fleet-battle:3v5):
cargo run -p game-client --features dev-tools -- --scenario fleet-battle --save data/fleet-battle.json
cargo run -p game-client --features dev-tools -- --scenario fleet-battle:3v5 --save data/fleet-battle.json
# macOS: build target/StarTrader.app with the game icon:
./tools/package_macos.sh
# Export a complete initial world/player save as JSON without a window:
cargo run -p game-client -- --new --headless > /tmp/star-trader-save.json
# Developer tooling (captures, test scenes, scenario saves) is behind the
# `dev-tools` feature and never ships in a default/release build.
# Run all engineering checks:
./tools/check.sh
```

## Browser preview

Install the WebAssembly target and Trunk once:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --locked --version 0.22.2
```

Run the game in a browser with a local development server, or create a static
HTML/WASM bundle under the ignored root `dist/` directory:

```sh
cd crates/game-client
trunk serve
# Or: trunk build --release
```

This is an early browser port: it starts a fresh game and does not yet persist
saves or preferences. See [platform status](docs/PLATFORM_STATUS.md).

The initial path is `data/slot-1.json`. Normal desktop launches open the
[main menu](docs/previews/main-menu.png): **Continue** (the last used game),
**New game** (setup, then an empty slot or a confirmed start-over in a used one),
**Load game** (three slots and any separately named last-used save), **Settings**
and **Quit**, over a slowly orbiting view of the stars where the game's 3D
ships cross on generated routes with live engine trails, slide into and out of a
turning trade station or sink into the planet (picked anew each launch) while a
moon and a prospector orbit it, while the game's original music plays. Load game shows which games can load, which slots are empty, and which
saves need attention. An incompatible save remains untouched until you confirm
deletion of it and its backup; the menu never autosaves over it. In game,
**Menu → Main menu** saves (or, with autosave off, asks first) and returns there.

[Settings](docs/previews/settings.png) (main menu, or **Menu → Settings** in game)
hold master and music volume, mute, music during play, fullscreen, interface scale
(75–200%), animations, the guide and autosave. They are saved immediately with
the other client preferences. The browser build has no audio yet. `data/settings.json` remembers the
selected save, and `--save PATH` selects another file. `--continue` loads the
selected file directly, while `--new` starts a session directly. **Session / saves**
provides New, Save, Load and slot selection after entering a game. Autosave runs on
close/suspend and writes to the selected slot. The previous valid file becomes
its `.bak`; a failed close-save keeps the window open with an error message.
Save slots live under ignored `data/`, keeping personal progress out of Git.
New games choose a fresh map seed by default. The main menu's New game page and
**Session / saves** let you choose 4–150 systems, a 20–95 ly maximum radius and
6–15 total objects per system (the central star counts as one). Defaults are
60 systems, a 70 ly radius and up to 8 objects. System count and radius set the
target density. Maps vary star classes, positions, planets,
moons and asteroids; each system has a trade port and a route back to Sol through
hops of 20 ly or less. Sol keeps its starter content and the starting ship and
stats stay the same. A saved game keeps its exact map; loading never generates
a new one. Use the same `--seed NUMBER`, `--systems`, `--radius` and `--objects` values to
repeat a map. Seed 0 remains the original baseline map for test fixtures.

Generated-world [galaxy preview](docs/previews/generated-galaxy-150.png) and
[15-object system preview](docs/previews/generated-system-15.png) show seed 42.
They are data schematics, generated by `python3 tools/preview_generated_maps.py`
after building the client (requires Pillow); they are not game UI captures.
The [celestial variant sheet](docs/previews/celestial-variants.png) shows the
nine star classes, seven rocky biomes and alternate gas giants, moons and
asteroids. Rebuild it with `python3 tools/art/preview_celestials.py` after exporting
sprites with `tools/art/build_celestials.py` in Blender.

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

See [map controls](docs/MAP_CONTROLS.md) for fleet selection, automatic routes,
stellar clearance and inner/outer view transitions. Fresh sessions include two
NPC traders with cautious/opportunistic route choices and real funds/cargo; they
meet raiders too, fight them off-screen and can lose cargo or be put out of service.

## First delivery

At Sol, tap **TRADE** (or **Menu**, then **Trade**) and press **Max** on food. Select your fleet on the map or
on the **Fleets** page. Double-click Meridian, select its trade port, then choose
**PLOT COURSE** in the destination card on the Selection page. Departure, entry and docking
are planned automatically. Press **Advance** to run until the fleet arrives;
resolve any contacts along the way, then sell (**All**) and refuel (**Full**).

Within a system, click planets, moons, asteroids or stations to choose them.
**Back to galaxy** only changes the view; **Depart system → galaxy** selects a
real departure course. Executed exits/arrivals follow the selected fleet between
outer/inner views. Safe local approaches route around the star; raw unsafe routes
reject. Mining and landing physics remain unimplemented.

**Plot course** never advances time or spends fuel. You command one **flagship**;
other fleets follow standing orders (Wait, trade route or patrol a system) and
settle their own contacts. Give the flagship a course or a **Wait**, then press
**Advance**: it executes navigation runs, moving all fleets and markets
together, until the flagship arrives, meets a contact or needs orders. The clock
button runs exactly one run. Travelling fleets meet raiders or one of ten other
contacts (salvage, convoys, customs, anomalies, comets and more); see
[navigation runs](docs/NAVIGATION_RUNS.md). New games are set up with galaxy
size, starting credits, inflation, danger, campaign length and the Trader goal.
Stars, planets and stations carry generated names. A new game opens its controls
gradually: the shipyard after your first 1,000 cr of profit, trade routes, battles,
landing and the Victory panel as you reach them, each with a short guide card.
**Fight** opens the full-screen battle (**Auto-resolve** skips it; × surrenders your cargo); each battle ends
with a combat report. Repair damaged hulls in the Market window.
Random state and choices persist. To automate a loop, select a destination market
while the fleet is at another market and choose **TRADE ROUTE**; **End route** on
the fleet's card stops it. See [navigation runs](docs/NAVIGATION_RUNS.md).

The game is turn based internally, with navigation runs as the campaign language.
Single-player no longer advances the whole world when one fleet moves. Multiplayer
will add shared player readiness; networking/authentication remain unimplemented.

Visit a planet, then open **Trade** to trade there. **Fleets** (the shipyard)
shows local hull stock, treasury-funded sale offers and fleet naming controls.
Markets can run out of money; shipyards can run out of hulls. See
[planet economies](docs/PLANET_ECONOMY.md).

Older saves preserve player assets and station goods; new treasuries and shipyards
start empty, and new planetary stock grows through navigation runs. A new game in
another save slot uses the configured starting funds and hull inventory.

## JSON economy and player data

[config/planets.json](config/planets.json) defines planet profiles, market templates,
starting budgets and hull stock. [config/ships.json](config/ships.json) defines purchasable hull prices, holds, fuel,
range and combat profiles. [config/combat.json](config/combat.json) defines the
weapons ([combat](docs/COMBAT.md)). [config/npc.json](config/npc.json) tunes the
NPC trader personalities ([traffic](docs/MAP_CONTROLS.md)). [config/economy.json](config/economy.json) defines starting credits/cargo capacity,
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
handle data; PNG decoding handles baked art. The browser build uses Trunk to
bundle the same Rust client. Source files are limited to 500
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
cargo run -p game-client --features dev-tools -- --new --capture /tmp/system.png --capture-view system
cargo run -p game-client --features dev-tools -- --new --capture /tmp/market.png --capture-view market
```

Example captures: [system](docs/previews/system.png), [market](docs/previews/market.png),
[fleets](docs/previews/fleets.png), [movement](docs/previews/navigation.png),
[entry preview](docs/previews/entry.png), [system arrival](docs/previews/arrival.png),
[local approach](docs/previews/approach.png), [ship near planet](docs/previews/orbit.png),
[planet market](docs/previews/planet-market.png), [planet shipyard](docs/previews/planet-shipyard.png),
[run review](docs/previews/run-review.png), [advancing](docs/previews/advance.png),
[trade route](docs/previews/trade-route.png),
[map command](docs/previews/destination.png),
[NPC traffic](docs/previews/traffic.png), [encounter](docs/previews/encounter.png), [battle](docs/previews/battle.png) (`battle-close`: stepped forward and aimed), [7 v 7 battle](docs/previews/fleet-battle.png) (`--scenario fleet-battle`), [phone battle](docs/previews/phone-battle.png), [Raider Haven](docs/previews/haven.png) (`haven`), [phone Victory panel](docs/previews/phone-victory.png) (`victory`), [landed and mining](docs/previews/landed.png), [raider band strikes](docs/previews/raiders.png), [fleet stacks](docs/previews/fleet-stacks.png) (`--scenario multi-fleet --capture-view galaxy`), [phone system view](docs/previews/phone-system.png) and [phone market](docs/previews/phone-market.png) (`--capture-size 390x844`), [save controls](docs/previews/saves.png). Capture views: `galaxy`, `system`, `advance`, `trade-route`, `market`, `fleets`, `saves`, `navigation`, `entry`, `arrival`, `approach`, `deep-space`, `orbit`, `planet-market`, `planet-shipyard`, `run-review`, `destination`, `traffic`, `encounter` (use `--seed 2`), `hail` (a hailed NPC trader), `trader-offer` and `distress` (a trader that came to you), `combat`, `battle`, `battle-close`, `landed`, `raiders`, `victory`, `factions` (the Factions panel), `factions-map` (territory rings), `police` (a
patrol stopping a wanted fleet), `haven`, `roster`, `unlock`, `chooser` (the main menu; `chooser@SECONDS` shows its
animation at that moment, for frame-by-frame checks), `menu-new`, `menu-load`,
`menu-settings`, `menu-songbook` (the Songbook's list), `menu-songbook@PART` (its map,
with a music part such as `cruise_a` shown as playing), `settings` (the in-game
Settings window). Captures do not test window
resize/DPI, touch or device lifecycle. An available GPU adapter is required.

This is escartian’s native rewrite of their own [original game](https://github.com/escartian/star_trader_game).
The [roadmap](ROADMAP.md) tracks fleet, planetary commerce, combat, faction and
mission work. Original web-save import remains unimplemented.
