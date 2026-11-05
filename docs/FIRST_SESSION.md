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
  Both other systems are within 20 light-years of Sol; the direct Meridian–Far
  Reach route exceeds the initial range.
  Local positions use a distinct type measured in kilometers.
- Travel costs ceil(distance in light-years) fuel units, takes the same number
  of simulation ticks, and requires enough fuel before departure. Start with
  a 40-unit tank; visited markets sell fuel at a proposed 2 credits per unit.
  Travel now uses checked integer distance/rounding in the simulation. Distinct
  systems at coincident coordinates still take at least one tick to travel between.
- Simulation rules use discrete ticks. Single-player advances continuously via a
  local host clock at one tick per real second in the prototype; one tick represents
  one in-world hour. Idle ticks catch up on the next UI input, while active travel
  requests scheduled redraws. Rendering, refreshes and inspection do not themselves
  advance time. Suspension pauses the clock; no catch-up is applied on resume.
  Multiplayer is planned around explicit turns, with tick-based AI. The shared-turn
  collection/resolution policy is not implemented yet.
- Money uses integer hundredths of a credit; cargo, fuel, stock, and ticks use
  whole nonnegative units. Reject invalid actions without partial mutation.
- Trade food, ore, medicine, machinery, textiles, and electronics between three
  reachable markets. Finite stock and a buy/sell spread prevent instant profit
  at a single port. Price quotes will carry revisions and expire clearly.
- The loop is inspect a route, buy cargo, travel, sell, refuel, then upgrade.
  No factions, combat, real-time piloting, or multiplayer are needed for this loop.

Implemented: fixture navigation and authoritative travel/arrival with fuel,
range, ownership, stale-quote and overflow checks. Proposed credits, cargo,
refuelling, prices and upgrade still require economic rules and pacing tests.
