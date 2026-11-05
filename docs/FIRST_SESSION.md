# First-session rules — current prototype

- Premium, offline-first single-player; optional multiplayer comes later. The
  client hosts the authoritative session in-process, without a background server.
- Start at Sol with one ship, 1000.00 credits, 20 cargo spaces and 40 fuel. Its
  jump range is 20 light-years. All seeded starts retain two reachable markets.
  The initial hold upgrade adds 20 cargo spaces for 1500.00 credits.
- Galaxy coordinates are whole light-years; local coordinates are a distinct
  kilometer-based type. Travel uses true Euclidean 3D distance. The fuel cost is
  ceil(distance), calculated exactly with checked integer arithmetic. Duration is
  the same number of ticks, with a minimum of one tick for distinct co-located systems.
- Everything uses discrete simulation ticks. Only successful player movement
  advances single-player time; waiting, trading, fleet management, inspecting and
  rendering consume none. Travel resolves its tick cost immediately, without an
  End Turn action. Future multiplayer resolves explicit shared turns.
- Money is integer hundredths of a credit. Fuel, stock and cargo use whole units.
  The authority validates ownership, location, quantities, capacity, stock, funds
  and revisions before publishing each command. Rejections leave state unchanged.
- Trade food, ore, medicine, machinery, textiles and electronics. Production and
  consumption run on ticks. Prices respond to stock within bounds, with a spread.
  Bulk orders price each unit; immediate buy/sell reversal cannot make a profit.
- First delivery: buy food at Sol, travel to Meridian, sell and refuel. Compare
  known markets for return cargo. Fixed-price fuel service costs 2.00 credits/unit
  by default. Earn enough for the cargo upgrade while budgeting for future journeys.
- Continue restores the exact saved journey, cargo, prices, markets and tick.
  Save slots use validated JSON, atomic replacement and a previous-good backup.
  Autosave runs on close/suspend; failed close-saves keep the window open.

The economy values come from config/economy.json. The initial 20–30 minute upgrade
pacing remains a playtest hypothesis; regression checks deliberately complete the
mechanical loop quickly. Owned ships/fleets are implemented; factions, combat,
mining, local docking and shared multiplayer turns remain planned.

- Ships have separate cargo/fuel. Fleet purchases, splitting/merging and cargo
  transfers consume no ticks. Travel moves the whole fleet; split first for an
  independent journey. Galaxy distances remain light-years; local distances kilometers.
