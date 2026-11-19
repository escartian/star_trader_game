# JSON definitions and session saves

## Economy definition (schema 1)

`config/economy.json` controls starting credits, cargo capacity, fuel price, hold
upgrade price/capacity, and a list of market templates. `commodity_order` is
explicit and validated. Commodity prices are integer minor units; cargo, stock
and production/consumption rates are whole units. Each rate applies per simulation
tick. Templates are assigned in stable system order, cycling if needed.

Market stock is bounded to twice its target. The price midpoint varies between
50% and 150% of base price. Asks add 10% and round upward; bids subtract 10% and
round downward. Bids use post-deposit stock, while asks use pre-withdrawal stock.
Bulk orders price each unit along that curve. This prevents immediate reversible
orders from making money, including very small JSON-configured markets.

Fuel is a fixed-price port service with an unlimited supply in this prototype;
commodity stocks are finite. Every trade checks the actor, ship, location, revision,
funds, cargo capacity, stock and quantity. The session publishes a candidate
transaction only after all rules pass. The GUI displays exact order totals and
current marginal prices; stale commands reject without silently changing their price.

Definitions reject unknown fields, unsupported schemas, invalid commodity order,
unbounded prices/rates, and invalid stocks/capacities. New games load the project
file when available or an explicitly supplied `--economy PATH`; packaged builds
can use the embedded validated defaults. No per-trade filesystem reads occur.

## Session save (schema 6)

Each `data/slot-N.json` stores:

- `systems`: stable IDs, star/body kinds, planet profiles and galaxy/local positions.
- `player`: tick/revision, movement rules, navigation state (run, courses, Wait orders as `holds`,
  encounters with raider `hostiles`, RNG, rescues, `last_combat` report), ships (IDs, owner, class, individual cargo/hold/fuel/range,
  `hull_damage` and body/local/deep-space location or legacy journey), fleets (IDs, owner and ship membership), monotonic ID counters.
- `economy`: shared player credits, current markets and exact session rules,
  including ship/planet/combat catalogs. Each market records a composite system/body ID,
  station flag, treasury, hull inventory and commodity balances. Cargo is keyed by commodity name inside each ship.

The root schema is independent of the command protocol version. Money uses
unsigned integer hundredths of a credit; credits and cargo cannot be negative.
Loading checks size, schema, IDs, references, coordinates, ownership, balances,
capacity, stock/rates, route validity and journey timing before returning a session.
A failed load leaves the live session unchanged. Schema 1 migrates its existing
single ship and global cargo/hold into Fleet 1 without resetting time, money or
journeys. Schema 2 migrates to schema 3 by adding validated default movement
rules; existing fleets/balances/positions remain intact. Unsupported versions
produce a clear error. Schemas 1–3 migrate to schema 4 planetary markets: old
station goods/prices remain, new commodity stocks start empty, and newly tracked
market treasuries/hull stock start at zero. The migration never grants player
assets or copies goods. See [migration policy](PLANET_ECONOMY.md). Do not hand-edit a live session's file
and expect its in-memory state to change.

Saving validates the state, writes a sibling temporary file, flushes/syncs it, and
atomically replaces the destination. The previous valid save is preserved in
`slot-N.json.bak`. A damaged primary never overwrites a good backup. Loading first
tries the primary, then a validated backup, reporting recovery explicitly. Parent
directory metadata is synced on Unix. These are local desktop saves, not a database
or a cross-platform crash-proof storage guarantee.

Continue restores prices, cargo, credits, ticks and journeys exactly; it never
regenerates them. Offline elapsed time is not added on load. Autosave runs on close
and suspend; a failed close-save keeps the window open for recovery. Slots and a
custom `--save PATH` are supported. `--headless` exports the same validated JSON
without writing a save or advancing time.

## Client settings (schema 2)

`data/settings.json` separately remembers `save_path`, `slot`, `active_ship`,
`guide` and `autosave`. Loading validates these values and their schema; invalid
preferences fall back to defaults with a message. Changes and save/close hooks use
atomic replacement. Settings are not world data and do not regenerate prices,
advance ticks or change player balances. `--save PATH` overrides remembered selection.
Legacy schema-1 preferences migrate while discarding the obsolete tick speed.
Session logs remain transient. Headless/capture modes never write settings.

Fleets are implemented; multiple-player accounts, discovery restrictions and
remote identity remain planned. Multiplayer will need authenticated per-player state and explicit
turn resolution around the same rules; a numeric owner ID is not authentication.

## Ship catalog (schema 2)

`config/ships.json` defines unique freighter/scout/patrol entries with `kind`,
`price` (credit hundredths), `cargo_capacity`, `fuel_capacity`, `range_ly` and a
`combat` profile. The profile has `size` (small/medium/large), `hull`, `shields`,
`armor` (points), `engine` (1–10), `weapon_slots` (0–8) and a default `loadout`
of weapon IDs (at most `weapon_slots`). Every loadout weapon must exist in the
combat catalog.
New games load this project file if available, with embedded defaults as fallback.
Saves preserve the active catalog. Purchased ships start empty and fully fuelled;
shipyard inventory is finite and saved by class. Empty-hold hulls can be sold
while retaining another owned ship, subject to market funds. Starter hold
capacity still comes from the economy definition. Ships record `hull_damage`
(0 = intact; it must stay below the class hull). Customization comes later.

## Combat catalog (schema 1)

`config/combat.json` lists weapons by stable snake_case `id`, each with `name`,
`damage` (1–10,000), `shots` (1–20), `accuracy_percent` (1–100),
`shield_efficiency_percent` (0–1000), `armor_penetration` and `description`.
It ships the original five: Photon Singularity Beam, Quantum Entanglement
Torpedo, Neutron Beam, Graviton Pulse and Magnetic Resonance Disruptor. New games
load the project file when present; saves preserve the active catalog. See
[combat](COMBAT.md) for resolution rules. Economy `repair_point_price`
(hundredths per hull point, default 500) prices market repairs.

## Movement definition (schema 1)

`config/movement.json` controls cubic extents, local fuel/tick distance units and
stellar exclusion. Configuration must contain every system/body within its bounds
and keep arithmetic in supported ranges. New sessions use these definitions;
continuing restores saved rules. See [movement rules](MOVEMENT.md) for units,
rounding, location variants and supported transitions. The primary station is
resolved from the saved system's first Station body; Docked retains its system ID.

## Planet catalog and finite-market allocations

`config/planets.json` schema 1 defines rocky/gas profiles: `economy`, `biome`,
`danger`, `specialization`, market-template index, `starting_credits` (hundredths)
and `ship_stock` in freighter/scout/patrol order. An optional `price_percent`
(six values in commodity order, 20–300; default 100) scales the template's base
prices when the market is created. Values below 100 mark a local surplus (the
specialty is cheap); values above 100 mark local demand (imports pay more).
Existing saves keep their markets' recorded prices. Economy template indices cycle
modulo available market templates. A specialization adds two production units per
tick, capped at 1000. Danger/biome remain descriptive pending encounter simulation.

Economy schema 1 has additive defaults for `market_starting_credits`,
`shipyard_starting_stock` and `ship_resale_percent` (1–90), plus embedded catalogs.
The defaults support existing definition files; migration explicitly starts newly
tracked live balances at zero. Runtime profile definitions, treasuries and hull
stocks are preserved in saves, rather than rebuilt when opening a panel.

Schema 6 adds combat data. A v5 save keeps its saved prices, holds, fuel and
range, gains each class's embedded combat profile, catalog schema 2 and the
default weapon catalog, and its ships start intact. Schema 5 adds explicit
navigation planning. Schema 4 migrates with existing market
balances/inventory and positions intact, with empty navigation orders and a saved
initial random state. Older versions chain through their previous migrations.
Loading never executes a run. Course geometry/durations, paid fuel, fleet references,
Holds, encounter IDs and counter bounds are checked before replacing a session.
Schema 6 also accepts additive `hostiles` (required exactly on raider contacts, at
most 3) and an optional `last_combat` report whose damage, indices and encounter
reference must be consistent.
Movement settings add `ticks_per_navigation_run` (default 4, 1–100) and
`encounter_chance_percent` (default 12, 0–100), with defaults for older definitions.

Backward compatibility is not required during prototyping. The migration paths
above describe current code, not a promise to support old schemas in future work.
Local UI distances/inputs use AU while JSON local coordinates remain integer km;
AU input conversion is checked and rounds to the nearest kilometer.

Navigation orders now include `remaining_waypoints`, an optional list of subsequent
MoveTargets. All future legs are checked against saved geometry and fuel. Economy
includes `traders`: identity, personality, wallet, independent hull/navigation state
and last activity. New sessions seed trader budgets from `npc_starting_credits`.
Upper-half ship/fleet IDs are reserved for NPCs. Existing sessions without traders
remain empty; backward compatibility is not a prototype requirement.
