# First-session rules — prototype baseline

These are working design decisions, to be adjusted through playtesting.

- Premium, offline-first single-player; multiplayer is optional and later.
  The client owns an in-process authoritative session. No socket or server
  process is needed for offline play.
- The first goal is to earn a cargo-hold upgrade within roughly 20–30 minutes.
  Start at Sol with one ship, 1,000 credits, 20 cargo units, and a full fuel tank.
  The proposed upgrade adds 20 cargo units for 1,500 credits.
- Galaxy positions are signed integer light-years on independent X/Y/Z axes.
  Distances use Euclidean 3D distance, independent of the map projection.
  The fixture's routes are all within the proposed 20-light-year jump range.
  Local positions will be a distinct type measured in kilometers.
- Travel costs ceil(distance in light-years) fuel units, takes the same number
  of simulation ticks, and requires enough fuel before departure. Start with
  a 40-unit tank; visited markets sell fuel at a proposed 2 credits per unit.
  Centralize checked distance/rounding in the simulation before enabling travel.
- Time advances only through explicit travel/wait actions, never wall-clock
  time, camera movement, opening panels, or refreshing snapshots. One tick is
  one in-world hour. Offline suspension does not advance time.
- Money uses integer hundredths of a credit; cargo, fuel, stock, and ticks use
  whole nonnegative units. Reject invalid actions without partial mutation.
- Trade food, ore, medicine, machinery, textiles, and electronics between three
  reachable markets. Finite stock and a buy/sell spread prevent instant profit
  at a single port. Price quotes will carry revisions and expire clearly.
- The loop is inspect a route, buy cargo, travel, sell, refuel, then upgrade.
  No factions, combat, real-time piloting, or multiplayer are needed for this loop.

Only the galaxy snapshot and interface fixture are implemented in this batch.
The proposed starting balances, travel, prices, and upgrade require simulation
implementation and route/pacing tests before they become playable rules.
