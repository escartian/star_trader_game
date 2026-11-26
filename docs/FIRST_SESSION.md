# First-session rules — current prototype

- Premium, offline-first single-player; optional multiplayer comes later. The
  client hosts the authoritative session in-process, without a background server.
- Start at Sol with one ship, 1000.00 credits, 20 cargo spaces and 40 fuel. Its
  jump range is 20 light-years. All seeded starts retain two reachable markets.
  The initial hold upgrade adds 20 cargo spaces for 1500.00 credits.
- Galaxy anchors are whole light-years; local UI coordinates use AU while the simulation stores integer kilometers. Deep
  space stores absolute kilometers to preserve small system-boundary offsets.
  Cubic bounds default to ±100 ly and ±200,000,000 km. Local travel costs one fuel
  per 100,000,000 km and one tick per 50,000,000 km (ceil, minimum one). Deep-space
  travel/entry costs ceil(distance in ly) fuel/ticks with exact range checks.
- Plot a course for each fleet or order it to Wait, then press Advance.
  Missing orders highlight/select the fleet first. The shared run advances four
  ticks; fleets progress in parallel and long voyages span multiple runs. Fuel is
  charged once at departure. Plotting, browsing and trading never advance time.
- Travelling fleets have a saved per-run encounter roll. Salvage/distress contacts
  pause that fleet for a decision; other fleets still progress. Resolve contacts
  before the next run. See [navigation runs](NAVIGATION_RUNS.md).
- Money is integer hundredths of a credit. Fuel, stock and cargo use whole units.
  The authority validates ownership, location, quantities, capacity, stock, funds
  and revisions before publishing each command. Rejections leave state unchanged.
- Trade food, ore, medicine, machinery, textiles and electronics at independent
  planet/station markets. Planet specializations boost production. Market funds
  and hull stock are finite; sales need a buyer that can afford them. Production and
  consumption run on ticks. Prices respond to stock within bounds, with a spread.
  Bulk orders price each unit; immediate buy/sell reversal cannot make a profit.
- First delivery: at Sol press **Max** on food to fill the hold, select Meridian's
  trade port on the map (the tap plots the course) and Advance until arrival. Sell with **All**
  and refuel with **Refuel fleet**. The margin column names the port
  (and its system) that pays most for each good; the selector above it limits
  that search to ports in fuel range, this system or the whole galaxy, and the
  best-trade line picks the single best one. Fixed-price fuel service costs 2.00 credits/unit
  by default. Earn enough for the cargo upgrade while budgeting for future journeys.
- Continue restores the exact saved journey, cargo, prices, markets and tick.
  Save slots use validated JSON, atomic replacement and a previous-good backup.
  Autosave runs on close/suspend; failed close-saves keep the window open.

The economy values come from config/economy.json. The initial 20–30 minute upgrade
pacing remains a playtest hypothesis; regression checks deliberately complete the
mechanical loop quickly. Owned ships/fleets are implemented; factions, combat,
mining and shared multiplayer turns remain planned. Local/body movement and
station docking are implemented. See [movement rules](MOVEMENT.md).

- Ships have separate cargo/fuel. Fleet buying/selling, renaming, splitting/merging and cargo
  transfers consume no ticks. Travel moves the whole fleet; split first for an
  independent journey. Galaxy distances remain light-years; local distances kilometers.
