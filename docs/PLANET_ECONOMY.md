# Planet economies and local shipyards

Rocky planets and gas giants each have a resource market, treasury and finite
ship inventory. Gas-giant commerce takes place at an orbital berth. The primary
station in each system retains a separate market. Moons, asteroids, arbitrary
waypoints and deep space currently have no markets.

Select a planet on the map, plot a course and Advance. At the body, open **Trade**
or **Fleets** (shipyard). Commerce uses the selected ship's
actual location, not the body or system being inspected. Every market is identified
by the pair `(system ID, body ID)`; IDs scoped to different systems never share stock.

The Trade window lists first-unit buy/sell prices per good and the best known
margin for buying here and selling elsewhere (first-unit prices before fuel; a
market in the same system is named by its body). Buy/Sell press the chosen order
quantity and show the whole order's total on hover. **Max** buys as much as cargo
space, stock and credits allow; **All** sells as much as is held and the buyer can
fund and store; **Full** refuels as far as credits allow. These sizes come from the
economy's own quote rules (`max_trade_quantity`, `max_refuel_units`), and the
authority still validates every order.

## Living markets and inflation

Markets are not static (`game-core/src/economy/dynamics.rs`). A market's future
depends only on its current state and recent past; nothing remembers how it was
founded. Each tick:

- **Activity:** every trade (yours or an NPC's) adds its value to its market's
  recent activity, which fades by 1/40 per tick (about 10% per run).
- **Income:** 4% of the value of the port's own production and consumption at
  today's prices (its local industry), plus 0.15% of its recent activity.
- **Upkeep:** 0.5% of its current treasury.

So a treasury settles where income meets upkeep: trade makes a port richer, a
quiet port falls back to what its industry supports, and a change in traffic
moves it from wherever it is now. Income and upkeep stand for off-screen trade,
taxes and spending; with bounties they are the only way money enters or leaves
the galaxy. Trades still only move money. The market header shows whether the
port's economy is growing or shrinking, with its income and upkeep per tick.

**Inflation:** the galaxy's price level compounds from its current value by 125
parts per million per tick (about 5% per 100 runs). Goods, fuel, repairs, hold
upgrades, hulls, charters and bounties all scale with it.

Tuning lives in the session rules (`dynamics`); `MarketDynamics::frozen()`
gives a static economy. In the three-system default world with two NPC traders
and the player idle, galaxy wealth went from 65,000 cr to about 56,000 cr by
run 50 as treasuries settled to their ports' industry and trade (2,000–8,000
cr), then rose with inflation to about 91,000 cr by run 400 (price level 1.22).

## Identity and production

`config/planets.json` defines rocky/gas templates with economy type, biome, danger,
production specialization, a market-template index, starting funds and ship stock.
Templates cycle by system order within each body kind. Each planet's markets are
independent even when they share a template. Its specialization adds two units of
production per tick, capped at the supported production limit. Each template's
`price_percent` gives the planet its own price identity. The specialty sells at
roughly half the template base price; goods the economy needs (machinery and
medicine for farms, food for mines, ore for industry) cost 20–40% more. So planets
in the same system as a trade port are worth a trip; a test checks that each
planet's specialty is at least a third cheaper than its system's port. The current
economy types are agricultural, extraction and industrial.

Biome and danger (Very safe through Insidious) are saved and shown in the inspector
and market. Cautious NPC traders avoid dangerous markets; biome remains descriptive data for
upcoming encounters/factions. No combat or danger-dependent encounter probability
is claimed yet.

Only Execute navigation run advances production/consumption ticks for the world. Browsing, quotes,
trading, ship transactions, fleet naming and saving never advance time. Market
production changes goods, not money. Fuel service currently has unlimited supply.

## Money and inventory

- Resource purchases, fuel and hold upgrades move credits from the player into
  the local market's treasury. Resource sales move credits back. A market that
  cannot afford an order rejects it without taking cargo.
- Hull purchases consume one local inventory unit and transfer the price into
  that market. They join the selected fleet at its actual location.
- Hull sales require an empty hold, another owned ship and sufficient local funds.
  The default offer is 60% of catalog price, configured by `ship_resale_percent`.
  Sales replenish one hull of that class, remove empty fleets and never reuse IDs.
- Returned hulls are refurbished catalog stock. Fuel and hold upgrades are not
  separately refunded. There are no used-hull identities/loadouts in shipyards yet.
- Ship stock is finite and does not replenish automatically. Selling hulls restocks
  it; NPC hull manufacturing/trading remains future work.
- Credits transfer between accounts rather than being created or discarded.
  All checks publish one candidate transaction or leave the entire session unchanged.

Stock arrays use the stable order **freighter, scout, patrol, prospector**, independent of
catalog display order. New sessions receive the explicit budgets and stocks in
their saved economy/planet definitions. Continuing uses saved definitions and
balances. A new session created in the GUI retains the active definitions.

## Saves

Schema 7 saves preserve market stock, prices, treasuries, hull inventory and
player assets exactly. Older schemas are not loaded; start a new game in another
slot to use the current world rules. Executing navigation runs advances planetary
production; player purchases and services fund local treasuries, and ship sales
replenish inventory.

Fleet renaming is available under **Fleets / shipyard**. Names are trimmed, contain
1–64 readable characters and at most 128 UTF-8 bytes. Names do not change IDs.

Initial NPC traders now compete in these same finite markets, with separate
wallets, cargo and fuel. `npc_starting_credits` explicitly allocates each trader's
new-session budget. They only act on committed navigation runs, and their trading
never debits the player wallet. See MAP_CONTROLS.md for personalities and scope.
