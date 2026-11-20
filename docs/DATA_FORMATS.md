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

## Session save (schema 8)

Each `data/slot-N.json` stores:

- `systems`: stable IDs, star/body kinds, planet profiles and galaxy/local positions.
- `player`: tick/revision, movement rules, navigation state (run, courses, Wait orders as `holds`,
  encounters with raider `hostiles`, RNG, rescues, `last_combat` report (outcome `victory`, `defeat`, `disengaged` or `surrendered`), the open `battle` (omitted when none), trade
  `routes` with their fleet and two market stops, omitted when empty), ships (IDs, owner, class, individual cargo/hold/fuel/range,
  `hull_damage` and body/local/deep-space location or legacy journey), fleets (IDs, owner and ship membership), monotonic ID counters,
  the Raider `haven` in navigation (`system`, `body`, `garrison`, `sunk`, `destroyed_run`; contacts against it carry `haven: true`)
  and `campaign` (`run_limit`, `ended` as `won` with its `road`, `lost` or `time_up`, each with its `run`).
- `economy`: shared player credits, current markets and exact session rules,
  including ship/planet/combat catalogs and `campaign` goals
  (`wealth_share_percent`, `charter_price`, `charters_to_win`,
  `port_share_percent`; an older `net_worth_goal` is read and ignored) and `dynamics`
  (`treasury_drift_percent`, `idle_size_percent`, `inflation_ppm_per_tick`), the
  galaxy `price_level_ppm`, and port `charters`
  (`market`, `owner`; omitted when none). Each market records a composite system/body ID,
  station flag, treasury, `founding_credits`, `size_percent`, recent `activity`,
  hull inventory and commodity balances (each good with its `price_level_ppm`).
  Saves from before living markets load: a market without `founding_credits`
  adopts its current treasury on the next tick. Cargo is keyed by commodity name inside each ship.

The root schema is independent of the command protocol version. Money uses
unsigned integer hundredths of a credit; credits and cargo cannot be negative.
Loading checks size, schema, IDs, references, coordinates, ownership, balances,
capacity, stock/rates, route validity and journey timing before returning a session.
A failed load leaves the live session unchanged. Since schema 7 every galaxy
system, local body, orbit and fleet position lies on the map plane (`y = 0`);
schema 8 moved battles to the hex board, and a schema-7 save is upgraded on
load. Earlier schemas are rejected with a clear message; start a new session. Do not
hand-edit a live session's file and expect its in-memory state to change.
Normal desktop launch shows a game chooser for three slots and the last-used
custom path. An incompatible save stays on disk until the player explicitly
confirms deletion of both the primary and backup files. No autosave runs while
the chooser is open. Other valid games can be loaded from the same screen.

Saving validates the state, writes a sibling temporary file, flushes/syncs it, and
atomically replaces the destination. The previous valid save is preserved in
`slot-N.json.bak`. A damaged primary never overwrites a good backup. Loading first
tries the primary, then a validated backup, reporting recovery explicitly. Parent
directory metadata is synced on Unix. These are local desktop saves, not a database
or a cross-platform crash-proof storage guarantee.
Campaign validation also checks that recorded ends agree with the run and
campaign length, that a Soldier win follows the Haven's destruction, and that
the Haven's remaining garrison uses its original hulls.

Continue restores prices, cargo, credits, ticks and journeys exactly; it never
regenerates them or the map. A new game chooses a fresh map seed unless an explicit
seed is given; the generated map is then saved as part of `systems`. Offline
elapsed time is not added on load. Autosave runs on close
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
`shield_efficiency_percent` (0–1000), `armor_penetration`, `range` (battle-board
steps: hexes plus altitude bands crossed, 1–12; 4 when absent, so older saved
catalogs load) and `description`.
It ships the original five: Photon Singularity Beam, Quantum Entanglement
Torpedo, Neutron Beam, Graviton Pulse and Magnetic Resonance Disruptor. New games
load the project file when present; saves preserve the active catalog. See
[combat](COMBAT.md) for resolution rules. Economy `repair_point_price`
(hundredths per hull point, default 500) prices market repairs. `raider_bounties`
(hundredths per destroyed raider, by hull: `freighter`, `scout`, `patrol`) and
`npc_raid_chance_percent` (0–100, default 4) load with defaults when absent. A
combat report adds `bounty` (hundredths, default 0).

## Movement definition (schema 1)

`config/movement.json` controls cubic extents, local fuel/tick distance units,
stellar exclusion, map-scale `body_radii` (km per body kind) and `landing` (fuel
per ship to land on or lift off from rocky planets, moons and asteroids, and ore
mined per ship per run on moons and asteroids). Older definitions without the
last two load with defaults. Ship locations add `Landed { system, body }`, which
must name a rocky planet, moon or asteroid.

Navigation adds roaming raider `bands` (id, system, local `position` and
`heading`, `ships`, optional `hunting` fleet, `cooldown` runs), `band_respawns`
(system and due run), `next_band_id` and the bands' own `band_rng`; all are
omitted or defaulted when absent. A raider contact may name the `band` that
struck; such a contact may target a waiting fleet, and its hostiles must match
the band's ships.

Bodies may carry an `orbit`: `period_ticks`, `epoch` (km offset at tick 0 from
what they circle) and an optional `parent` body for moons (one level only).
Saved `position` is recomputed from the orbit for the saved tick on load. Bodies
without an orbit stay fixed. Movement plans record their planning `tick`.
Configuration must contain every system/body within its bounds
and keep arithmetic in supported ranges. New sessions use these definitions;
continuing restores saved rules. See [movement rules](MOVEMENT.md) for units,
rounding, location variants and supported transitions. The primary station is
resolved from the saved system's first Station body; Docked retains its system ID.

## Planet catalog and finite-market allocations

`config/planets.json` schema 1 defines rocky/gas profiles: `economy`, `biome`,
`danger`, `specialization`, market-template index, `starting_credits` (hundredths)
and `ship_stock` in freighter/scout/patrol/prospector order. An optional `price_percent`
(six values in commodity order, 20–300; default 100) scales the template's base
prices when the market is created. Values below 100 mark a local surplus (the
specialty is cheap); values above 100 mark local demand (imports pay more).
Existing saves keep their markets' recorded prices. Economy template indices cycle
modulo available market templates. A specialization adds two production units per
tick, capped at 1000. Danger/biome remain descriptive pending encounter simulation.

Economy schema 1 has additive defaults for `market_starting_credits`,
`shipyard_starting_stock` and `ship_resale_percent` (1–90), plus embedded catalogs.
The defaults support existing definition files. Runtime profile definitions, treasuries and hull
stocks are preserved in saves, rather than rebuilt when opening a panel.

Schema 7 stores the current combat and navigation state. Loading never executes
a run. Course geometry/durations, paid fuel, fleet references,
Holds, encounter IDs and counter bounds are checked before replacing a session.
Raider contacts store `hostiles` (required exactly on raider contacts, at
most 3) and an optional `last_combat` report whose damage, indices and encounter
reference must be consistent. An open `battle` records its `board` (`radius`),
each ship's position `at` (`hex` with axial `q`/`r`, and `band`: `low`, `level`
or `high`), the acting ship's `moves_left` and the ceasefire state (`acted`,
`quiet_rounds`). It must use the board its ship count gives, belong to a pending
raider contact, field exactly that fleet's ships and the contact's raiders with
their catalog profiles and saved damage, keep active ships on distinct hexes on
the board, give the acting ship no more movement than its engine allows, and
wait on a player ship; anything else rejects the save. Schema 8 introduced the
hex board; a schema-7 save loads with any open 3D-grid battle auto-resolved
(see [combat](COMBAT.md)).
Movement settings add `ticks_per_navigation_run` (default 4, 1–100) and
`encounter_chance_percent` (default 12, 0–100), with defaults for older definitions.

Backward compatibility is not required during prototyping. Only the current
session schema loads.
Local UI distances/inputs use AU while JSON local coordinates remain integer km;
AU input conversion is checked and rounds to the nearest kilometer.

Navigation orders now include `remaining_waypoints`, an optional list of subsequent
MoveTargets. All future legs are checked against saved geometry and fuel. Economy
includes `traders`: identity, personality, wallet, independent hull/navigation state
and last activity, plus `refit_runs` (0–8, omitted when 0) while a trader destroyed
by raiders is out of service. A trader's hull damage must stay below its class
hull. New sessions seed trader budgets from `npc_starting_credits`.
Upper-half ship/fleet IDs are reserved for NPCs. Existing sessions without traders
remain empty; backward compatibility is not a prototype requirement.
