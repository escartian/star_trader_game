# Jobs

Status: **job board implemented** (roadmap milestone 6, first part). Story
quests and the Diplomat road build on this lifecycle next.

Code: `game-core/src/jobs.rs` (types, accept, catalog), `jobs/board.rs`
(offers), `jobs/settle.rs` (completion, failure, payout); commands and the
after-command pass in `game-server/src/jobs.rs`; the port's Jobs tab, the Jobs
window and the outcome card in `game-client/src/session/jobs.rs`. Tuning and
text: `config/jobs.json`.

## Finding and taking jobs

- A port posts a few offers (`offers_per_port`, 3) while one of your ships is
  there and its last board has expired. A board stands for `refresh_runs` (8)
  runs from posting, then the next visit gets a new one. Boards where you have
  not been are not kept, so saves stay small.
- The market window's **Jobs** tab lists the port's offers: what to do, the
  reward, standing with the port's faction (and what failing costs), and the
  deadline run. **Accept** takes it; the selected ship must be at that port.
- At most `max_active` (5) jobs at once, and never two with the same task.
- **Menu → Jobs** lists accepted jobs with where to go, runs left and
  **Abandon**, plus the completed/failed record and recent results.

## Kinds

Every target lies in the posting port's `reach_systems` (6) nearest systems.
Rewards are base pay plus `reward_step` per unit, raider or light-year.

| Kind | Task | Done when | Fails when |
| --- | --- | --- | --- |
| Delivery | bring N units of a good to another port | a ship of yours at that port carries them: they are taken into its stock | deadline |
| Bounty | wipe out a raider band | your fleets destroy its last raider | another band-killer gets there first, or deadline |
| Escort | see an NPC trader safely into its destination port | it docks there while one of your fleets is in that system | it is lost, docks with none of your fleets there, or deadline |
| Survey | put a ship in orbit of (or on) a body | any ship of yours orbits or lands there | deadline |

A delivery also pays the goods' asking price at the destination when posted.

## Settling

After every order (never after a query) the host refreshes boards and
settles each accepted job once:

- **Complete:** credits and `standing` with the faction holding the posting
  port (none in independent space) are paid; an event card says what was done
  and what it paid.
- **Failed** (missed deadline, impossible task, or abandoned): no pay, and
  `penalty` standing is lost with that faction.

Standing moves through the usual faction rules, so allies and rivals share
half of each change. Nothing settles once the campaign is lost or complete.

## Saves

The player state's `jobs` holds standing `boards` (port and posting run),
`offers`, `active` jobs, `next_id`, a short `history`, the completed and failed
counts, and `cleared_bands` (bands your fleets wiped out). It is omitted when
empty; older saves load without it (loading never changes the state), and the
first order after loading posts the current port's board.
