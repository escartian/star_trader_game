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

## Session save (schema 10)

Each `data/slot-N.json` stores:

- `systems`: stable IDs, star/body kinds, planet profiles and galaxy/local positions.
- `player`: tick/revision, movement rules, navigation state (run, courses, Wait orders as `holds`,
  encounters with raider `hostiles`, RNG, rescues, `last_combat` report (outcome `victory`, `defeat`, `disengaged` or `surrendered`), the open `battle` (omitted when none), trade
  `routes` with their fleet and two market stops, omitted when empty), ships (IDs, owner, class, individual cargo/hold/fuel/range,
  `hull_damage` and body/local/deep-space location or legacy journey), fleets (IDs, owner and ship membership), monotonic ID counters,
  the Raider `haven` in navigation (`system`, `body`, `garrison`, `sunk`, `destroyed_run`; contacts against it carry `haven: true`)
  the `flagship` fleet and secondary fleets' `patrols` (`fleet`, `system`, `stops`) in navigation,
  the flagship's `pursuit` (`fleet`, `quarry`: `{"Trader": id}` or `{"Band": id}`; omitted when none),
  `campaign` (`run_limit`, `ended` as `won` with its `road`, `lost` or `time_up`, each with its `run`),
  and `jobs` (board offers, accepted jobs and their record; see [JOBS.md](JOBS.md#saves); omitted when empty).
- `economy`: shared player credits, current markets and exact session rules,
  including ship/planet/combat catalogs, the `job_catalog` (`config/jobs.json`), the `quest_catalog` (`config/quests.json`) and `campaign` goals
  (`wealth_share_percent`, `charter_price`, `charters_to_win`,
  `port_share_percent`; an older `net_worth_goal` is read and ignored) and `dynamics`
  (`local_income_percent`, `trade_income_ppm`, `upkeep_ppm_per_tick`,
  `inflation_ppm_per_tick`, `inflation_basis_points_per_100_runs`,
  `inflation_ppb_per_tick`,
  `inflation_ticks_per_100_runs`), the galaxy `price_level_ppm` and saved
  `inflation_remainder`, port `charters`
  (`market`, `owner`; omitted when none) and `factions` (`in_play`,
  `allegiance`, `territory`, `relations`, `standing`, `sales_cents`, `wanted`; see
  [factions](FACTIONS.md); absent means no factions). Each market records a composite system/body ID,
  station flag, treasury, recent trade `activity`, hull inventory and commodity
  balances (each good with its `price_level_ppm`). Cargo is keyed by commodity name inside each ship.

The root schema is independent of the command protocol version. Money uses
unsigned integer hundredths of a credit; credits and cargo cannot be negative.
Loading checks size, schema, IDs, references, coordinates, ownership, balances,
capacity, stock/rates, route validity and journey timing before returning a session.
A failed load leaves the live session unchanged. Since schema 7 every galaxy
system, local body, orbit and fleet position lies on the map plane (`y = 0`);
schema 8 moved battles to the hex board, schema 9 flattened that board, and
schema 10 added two alien factions with six-slot standing and quest state.
Schema-7 through -9 saves are upgraded on load; older games keep their original
four factions in play. Earlier schemas are rejected with a clear message; start a new session. Do not
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
`shield_efficiency_percent` (0–1000), `armor_penetration`, `range` (hexes,
1–12; 4 when absent, so older saved catalogs load), `description` and an
optional `effect` (`plain` when absent, `piercing`, `guided`, `steady`,
`ripple` or `disrupt`; see [combat](COMBAT.md)). A battle unit may record
`jammed` (true while a disruptor hit costs it a move).
It ships the original five: Photon Singularity Beam, Quantum Entanglement
Torpedo, Neutron Beam, Graviton Pulse and Magnetic Resonance Disruptor. New games
load the project file when present; saves preserve the active catalog. See
[combat](COMBAT.md) for resolution rules. Economy `repair_point_price`
(hundredths per hull point, default 500) prices market repairs. `raider_bounties`
(hundredths per destroyed raider, by hull: `freighter`, `scout`, `patrol`) and
`npc_raid_chance_percent` (0–100, default 4) load with defaults when absent. A
combat report adds `bounty` (hundredths, default 0).

## Faction catalog (schema 2)

`config/factions.json` lists six factions in order (`Federation`,
`Empire`, `Republic`, `Alliance`, `Miri`, `Auralith`), each with `id`, `name` (1–64 bytes),
`species` (1–64 bytes),
`description` (up to 256), `color` (sRGB triple), `piracy_penalty` (0–100),
`raider_kill_standing` (0–100), `sale_credits_per_point` (hundredths,
100–100,000,000), `wanted_runs` (1–200, default 30) and `fine_per_run`
(hundredths, at most 10,000,000, default 1,000), plus `classic_relations`: pairs `between` two different
factions with `relation` `Allied` or `Rival` (unlisted pairs are neutral). It is
embedded as a fallback, loaded from the project file when present, and saved in
the economy rules as `faction_catalog`.

## NPC personality catalog (schema 1)

`config/npc.json` lists the six NPC trader personalities in order
(`Cautious`, `Opportunist`, `BulkHauler`, `Loyalist`, `Smuggler`, `Corsair`),
each with `id`, `label` (1–64 bytes), `description` (up to 256),
`avoids_danger` (skip ports on dangerous, very dangerous or insidious planets),
`danger_profit_percent` (50–400: how a haul to such a port is valued),
`friendly_only` (a faction trader trades only in its own or allied space),
`margin_percent` (0–100: least profit as a share of the haul's cost),
`haul_units` (1–1,000, capped by the hold), `yield_percent` (100–1,000: your
fleet strength, as a share of the trader's, that makes it yield to a demand)
`raid_escape_percent` (0–100: chance to slip past raiders) and
`approach_percent` (0–100, default 0: chance per run to approach your
flagship), plus `approaches` (`offer_price_percent` 10–200,
`shakedown_strength_percent` 100–1,000, `cooldown_runs` 0–1,000,
`distress_percent` 0–100, `rescue_reward_percent` 0–100, `rescue_standing`
0–100; defaults 85, 150, 10, 60, 10 and 3) and
`spawn_cycle` (1–64 personalities dealt to new traders in turn; independents
dealt a loyalist become smugglers). It is embedded as a fallback, loaded from
the project file when present, and saved in the economy rules as
`personality_catalog`; saves without it use the embedded catalog.

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

`config/planets.json` schema 1 defines rocky/gas profiles (and, optionally,
`surfaces` for moons and asteroids: `kind` `Moon` or `Asteroid` and a `profile`
of `biome`, `danger` and `ore_percent` 10–1000; bodies save it as `surface`, never
on other kinds; biomes add `metallic`). Rocky/gas profiles hold `economy`, `biome`,
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
its `terrain` (a list of `hex` with axial `q`/`r` and `kind`: `asteroid` or
`debris`), each ship's hex `at` (axial `q`/`r`), the acting ship's `moves_left`
and the ceasefire state (`acted`, `quiet_rounds`). It must use the board its
ship count gives, carry exactly the terrain its seed generates, keep ships off
asteroids, belong to a pending raider (or hailed trader) contact, field exactly that fleet's ships and the contact's raiders with
their catalog profiles and saved damage, keep active ships on distinct hexes on
the board, give the acting ship no more movement than its engine allows, and
wait on a player ship; anything else rejects the save. Schema 8 introduced the
hex board and schema 9 removed its altitude bands; a schema-7 or -8 save loads
with any open battle auto-resolved (see [combat](COMBAT.md)).
A hailed trader's contact (`kind` `Trader`) stores `trader` (`id`, `name`,
`hull_damage`) and the trader's hull as its one `hostiles` entry; both must match
the saved NPC trader, which cannot be refitting. A combat report from an attack
on a trader names it in `trader` and records `plunder` (cargo units taken).
A trader that approaches your flagship opens a `TraderOffer` or `Shakedown`
contact stored the same way (`trader` and its hull as `hostiles`); a report
from resisting a shakedown also sets `shakedown`. A `TraderDistress` contact
stores the stalking band in `band` (its ships as `hostiles`) and the trader in
`rescue`; its battle report keeps `rescue` and records `rescue_reward` (cents
paid by the trader on victory). Both must name an existing trader. NPC traders record
`last_approach` (the run of their last approach, at most the current run).
`navigation.piracies` counts demands and attacks. Ships (player and NPC) may
list fitted `gear` (universal afterburners and point defense; model-specific
`scout_drive`, `freighter_racks`, `patrol_targeting`, `prospector_drill`; and
size-specific `evasion_jets`, `shield_array`, `armored_bulkheads`, each at most
once), and so
may battle units; `navigation.escorts` holds hired escorts (`fleet`, `runs_left`
1–10, one per existing fleet), NPC traders `escort_runs` (0–10), and a hailed
trader's contact its `gear` and `escort` (then `hostiles` adds the escort's
patrol hull). An open battle may include one unowned player-side escort only
while that fleet's contract runs. Economy rules add `gear_prices`
(`afterburners`, `point_defense`, the seven new upgrades, `escort`), with defaults
for older files. The model and size restrictions are checked on purchase and
save load; Scout range and Freighter capacity reflect their fitted upgrades. A raider band may record
`stalking` (an NPC trader's ID, which must exist; never together with
`hunting`), and its `cooldown` may reach 12 after striking a trader. These fields are additive and
omitted when empty.
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
