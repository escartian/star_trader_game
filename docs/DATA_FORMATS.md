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

## Session save (schema 2)

Each `data/slot-N.json` stores:

- `systems`: stable IDs, star/body kinds and galaxy/local positions.
- `player`: tick/revision, ships (IDs, owner, class, individual cargo/hold/fuel/range
  and location/journey), fleets (IDs, owner and ship membership), monotonic ID counters.
- `economy`: shared player credits, current markets and exact session rules,
  including the ship catalog. Cargo is keyed by commodity name inside each ship.

The root schema is independent of the command protocol version. Money uses
unsigned integer hundredths of a credit; credits and cargo cannot be negative.
Loading checks size, schema, IDs, references, coordinates, ownership, balances,
capacity, stock/rates, route validity and journey timing before returning a session.
A failed load leaves the live session unchanged. Schema 1 migrates its existing
single ship and global cargo/hold into Fleet 1 without resetting time, money or
journeys; other unsupported versions produce a clear error. Do not hand-edit a live session's file
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

## Ship catalog (schema 1)

`config/ships.json` defines unique freighter/scout/patrol entries with `kind`,
`price` (credit hundredths), `cargo_capacity`, `fuel_capacity` and `range_ly`.
New games load this project file if available, with embedded defaults as fallback.
Saves preserve the active catalog. Purchased ships start empty and fully fuelled;
shipyard stock is currently unlimited and ships cannot yet be sold. Starter hold
capacity still comes from the economy definition. HP/equipment are future data.
