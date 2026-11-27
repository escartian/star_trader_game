# Planet economies and local shipyards

Rocky planets and gas giants each have a resource market, treasury and finite
ship inventory. Gas-giant commerce takes place at an orbital berth. The primary
station in each system retains a separate market. Moons, asteroids, arbitrary
waypoints and deep space currently have no markets.

Moons and asteroids carry a **surface** instead (`surfaces` in
`config/planets.json`): a biome, a danger rating and an ore richness. It sets
their look, how strongly raider bands favour patrolling them (like planets'
danger), where the Raider Haven may stand, and how much ore a landed fleet
mines each run (the landing rules' yield × `ore_percent`):

| Body | Surface | Danger | Ore per ship per run |
| --- | --- | --- | --- |
| Moon | barren (rocky moon) | safe | 2 |
| Moon | frozen (ice moon) | very safe | 1 |
| Asteroid | barren | moderate | 2 |
| Asteroid | metallic | dangerous | 4 |

Richer surfaces are riskier places to sit and mine. The info card shows a
selected moon's or asteroid's biome, danger and yield. Saves from before
surfaces get the ones a new game would give them when they load.

Select a planet on the map, plot a course and Advance. At the body, open the
Market window's **Trade** or **Shipyard** tab. Commerce uses the selected ship's
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

### Fleet orders

Buying and selling are **fleet orders** (`game_core::fleet_trade`,
`ClientCommand::FleetTrade`): every ship of the selected ship's fleet at the
same port shares the order. The Trade window shows their pooled hold (cargo
column and used/capacity), and the order is priced and checked once, as one
bulk trade against that pooled hold, then split among the ships. Cargo stays
per ship, so a ship destroyed in battle loses its own hold. The **Load**
picker beside the order quantity (shown when two or more of the fleet's ships
are there; also under Settings → Gameplay, saved with the client settings)
chooses the split:

| Load order | Buying fills | Selling empties |
| --- | --- | --- |
| Spread evenly (default) | each hold to the same share of its capacity | the fullest share first |
| Toughest ship first | the ship with the most hull and shields left | the weakest first |
| Largest hold first | the largest hold | the smallest first |
| Selected ship only | only the selected ship | only the selected ship |

Upgrades and repairs stay per ship: the hold upgrade and repairs apply to the
selected ship. Fit gear from each ship's row under *Ship upgrades* on the
Shipyard tab (see [combat](COMBAT.md#ship-upgrades-and-escorts)). The event log
names how many units each ship took or gave.

## Living markets and inflation

Markets are not static (`crates/game-core/src/economy/dynamics.rs`). A market's future
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

**Inflation:** new games default to 0.00%. Setup accepts a custom percentage per
100 navigation runs to two decimal places (0.00–100.00%). The rate is spread
across navigation ticks with a saved fractional remainder, so even 0.01% takes
effect. Prices compound from the current level. Goods, fuel, repairs, upgrades,
hulls, charters and bounties all scale with it.

Tuning lives in the session rules (`dynamics`); `MarketDynamics::frozen()`
gives a static economy. In the three-system default world with two NPC traders
(measured before the population scaled with the galaxy; that world now fields
four) and the player idle, galaxy wealth went from 65,000 cr to about 56,000 cr by
run 50 as treasuries settled to their ports' industry and trade (2,000–8,000
cr). An earlier 5% inflation setup rose to about 91,000 cr by run 400
(price level 1.22); the new default remains at a price level of 1.00.

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
- Hull sales require another owned ship and sufficient local funds. Cargo moves
  to other owned ships at the same berth, in ship ID and commodity order. If
  they lack space, the sale is rejected until the player explicitly chooses
  **Sell and discard**; then only the overflow is lost.
  The default offer is 60% of catalog price, configured by `ship_resale_percent`,
  plus the same share of today's price for every upgrade on the ship (fitted
  gear and a bought hold upgrade); the Sell button's hover shows that part.
  Sales replenish one hull of that class, remove empty fleets and never reuse IDs.
- Returned hulls are refurbished catalog stock: upgrades are sold back with the
  ship and stripped, fuel is not refunded. There are no used-hull
  identities/loadouts in shipyards yet.
- A shipyard can remove a fitted upgrade, including the original cargo hold
  upgrade, for its current resale value. A one-step swap refunds the removed
  item and charges for the new one atomically. Capacity can be swapped without
  unloading if cargo fits the final hold; removal that leaves too little room
  is rejected. The shipyard needs funds for the refund.
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
