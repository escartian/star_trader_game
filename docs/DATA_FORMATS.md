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

## Session save (schema 1)

Each `data/slot-N.json` stores:

- `systems`: stable IDs, star/body kinds and galaxy/local positions.
- `player`: tick/revision, ship ID/owner, fuel/range, location or active journey.
- `economy`: credits, cargo keyed by commodity name, hold capacity/upgrade state,
  current markets and the exact rules used by that session.

The root schema is independent of the command protocol version. Money uses
unsigned integer hundredths of a credit; credits and cargo cannot be negative.
Loading checks size, schema, IDs, references, coordinates, ownership, balances,
capacity, stock/rates, route validity and journey timing before returning a session.
A failed load leaves the live session unchanged. Unknown versions currently produce
a clear error; no migrations are implemented. Do not hand-edit a live session's file
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

Client slot selection, pause, speed, guide visibility and event logs are runtime controls/history,
not part of this authoritative save schema. Fleets, multiple-player accounts,
discovery restrictions and remote identity are not implemented. Multiplayer will
need authenticated per-player state and explicit turn resolution around the same
rules; passing a numeric owner ID is not authentication.
