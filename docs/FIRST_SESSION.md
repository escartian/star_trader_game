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
- Everything is calculated in discrete simulation ticks. Single-player schedules
  ticks automatically with no End Turn action: at 1×, one real second is one
  in-world hour. Pause and 2×/4× change scheduling, not rules. Idle ticks catch up
  on the next input; active journeys schedule redraws. Suspend/load adds no offline
  time. Future multiplayer will resolve explicit turns; future AI can act on ticks.
- Money is integer hundredths of a credit. Fuel, stock and cargo use whole units.
  The authority validates ownership, location, quantities, capacity, stock, funds
  and revisions before publishing each command. Rejections leave state unchanged.
- Trade food, ore, medicine, machinery, textiles and electronics. Production and
  consumption run on ticks. Prices respond to stock within bounds, with a spread.
  Bulk orders price each unit; immediate buy/sell reversal cannot make a profit.
- First delivery: buy food at Sol, travel to Meridian, sell and refuel. Compare
  known markets for return cargo. Fixed-price fuel service costs 2.00 credits/unit
  by default. Earn enough for the cargo upgrade while budgeting for future journeys.
- Continue restores the exact saved journey, cargo, prices, markets and clock.
  Save slots use validated JSON, atomic replacement and a previous-good backup.
  Autosave runs on close/suspend; failed close-saves keep the window open.

The economy values come from config/economy.json. The initial 20–30 minute upgrade
pacing remains a playtest hypothesis; regression checks deliberately complete the
mechanical loop quickly. Factions, combat, mining, fleets, local docking and shared
multiplayer turns are not needed or implemented for this first trading loop.
