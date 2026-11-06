# Planet economies and local shipyards

Rocky planets and gas giants each have a resource market, treasury and finite
ship inventory. Gas-giant commerce takes place at an orbital berth. The primary
station in each system retains a separate market. Moons, asteroids, arbitrary
waypoints and deep space currently have no markets.

Inspect a planet, choose **Preview movement to body**, then plot a course and execute navigation runs. At the body,
open **Market / refuel** or **Fleets / shipyard**. Commerce uses the selected ship's
actual location, not the body or system being inspected. Every market is identified
by the pair `(system ID, body ID)`; IDs scoped to different systems never share stock.

## Identity and production

`config/planets.json` defines rocky/gas templates with economy type, biome, danger,
production specialization, a market-template index, starting funds and ship stock.
Templates cycle by system order within each body kind. Each planet's markets are
independent even when they share a template. Its specialization adds two units of
production per tick, capped at the supported production limit. The current economy
types are agricultural, extraction and industrial.

Biome and danger (Very safe through Insidious) are saved and shown in the inspector
and market. They are descriptive data for upcoming encounters/factions; no combat
or danger-dependent encounter simulation is claimed yet.

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
  it; NPC manufacturing/trade will be separate work.
- Credits transfer between accounts rather than being created or discarded.
  All checks publish one candidate transaction or leave the entire session unchanged.

Stock arrays use the stable order **freighter, scout, patrol**, independent of
catalog display order. New sessions receive the explicit budgets and stocks in
their saved economy/planet definitions. Continuing uses saved definitions and
balances. A new session created in the GUI retains the active definitions.

## Older saves

Save schema 5 stores navigation runs and migrates rewrite schemas 1–4. Player credits, ships, cargo, fuel,
positions, journeys and fleet membership remain unchanged. Original system-market
stock and prices remain at their original primary stations and are not copied
into every planet.

Older saves never tracked market money or hull inventory. Their migrated treasuries
and shipyards therefore start at **zero**, and newly introduced planetary commodity
stocks start empty. The migration message states this explicitly. Executing navigation runs starts
planetary production; player purchases/services fund local treasuries, and ship
sales replenish inventory. Existing runs receive no free money or hulls. To try
the configured starting budgets and stocked yards, start a new game in another
save slot. Loading again does not rerun the migration or refill depleted markets.

Fleet renaming is available under **Fleets / shipyard**. Names are trimmed, contain
1–64 readable characters and at most 128 UTF-8 bytes. Names do not change IDs.
