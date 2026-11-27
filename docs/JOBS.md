# Jobs

Status: **job boards, fence work and story quests implemented** (roadmap
milestone 6, first part). Faction story quests lead to the Diplomat road, the
Free Captains' chain to the Pirate road (see [CAMPAIGN.md](CAMPAIGN.md)).
Miri Observatory and Auralith Confluence chains remain available in legacy
saves that had those political factions; new games treat them as species.

Code: `crates/game-core/src/jobs.rs` (types, accept, catalog), `jobs/board.rs`
(offers; fence work in `jobs/board/fence.rs`), `jobs/settle.rs` (completion,
failure, payout), `jobs/quests.rs` (story quests; definitions in
`jobs/quests/catalog.rs`); commands and the
after-command pass in `crates/game-server/src/jobs.rs`; the port's Jobs tab, the Jobs
window and the outcome card in `crates/game-client/src/session/jobs.rs`, quest cards
in `session/quests.rs`. Tuning and text: `config/jobs.json`, `config/quests.json`.

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
Kinds marked `lawless` in the catalog (**fence work**: raids and smuggling) are
posted only at ports in systems no faction holds; they pay no standing and
cost none.

| Kind | Task | Done when | Fails when |
| --- | --- | --- | --- |
| Delivery | bring N units of a good to another port | a ship of yours at that port carries them: they are taken into its stock | deadline |
| Bounty | wipe out a raider band | your fleets destroy its last raider | another band-killer gets there first, or deadline |
| Escort | see an NPC trader safely into its destination port | it docks there while one of your fleets is in that system | it is lost, docks with none of your fleets there, or deadline |
| Survey | put a ship in orbit of (or on) a body | any ship of yours orbits or lands there | deadline |
| Raid (fence) | seize N units from NPC traders, by demand or plunder | units seized since accepting reach N | deadline |
| Smuggling (fence) | bring N units of a good into a faction port | as a delivery | that faction's police stop one of your fleets on the way, or deadline |

A delivery or smuggling run also pays the goods' asking price at the
destination when posted. Police only stop fleets their faction pursues (wanted,
or Hostile standing), so smuggling is safe until you turn pirate.

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

## Story quests

Each great power has a chain of hand-written steps (four today): requests that
tell its side of the frontier's story, ending with a treaty. They run on the
job lifecycle above.

- With factions in play, a faction offers its next step when your standing
  with it reaches the step's tier: **Friendly** (15) for the requests,
  **Honored** (50) for the treaty. One step per faction at a time.
- The offer arrives as an event card with its story: **Accept** or **Later**
  (it then waits under *Requests* in Menu → Jobs). Quest steps are accepted
  anywhere, never count against the job limit, and their deadline starts when
  accepted.
- A step's task is drafted from the faction's port nearest your flagship: a
  delivery of a set good to one of its ports, a bounty, an escort or a survey
  nearby. The treaty is a delivery to the nearest port of the faction's rival
  (or of another faction when it has none).
- Completing a step pays like a job and opens the next one; failing it costs
  its penalty and the step returns after `retry_runs` (6).
- **Carrying a treaty wins the Diplomat road.** The Jobs window shows each
  faction's story progress, and the score counts every step done.

### The Free Captains

The pirate chain (`pirate` in `config/quests.json`, four steps today) runs the
same way with these differences:

- It is offered in any galaxy with NPC traders. Its steps open with your
  pirate record instead of standing: a step's `seizures` (successful cargo
  seizures from traders) and `escapes` (police patrols evaded after a seizure;
  ignored without factions). Today: a raid after your first seizure, a
  smuggling run after the second, a survey for a hidden anchorage once you
  have also escaped the police, and a final raid after the third seizure.
- Steps are drafted from the lawless port nearest your flagship (any port when
  every system is held) and pay credits only.
- **Completing the chain wins the Pirate road.** Its offers arrive as red
  event cards, and the Jobs window and Victory panel say what the next step
  waits for.

## Saves

The player state's `jobs` holds standing `boards` (port and posting run),
`offers`, `active` jobs, `next_id`, a short `history`, the completed and failed
counts, `cleared_bands` (bands your fleets wiped out), and for quests
`quest_offers`, `quest_steps` and `quest_retry` (by faction), `treaty`, and
`pirate_steps` and `pirate_retry` for the Free Captains. A raid job's task
records the units `taken` so far. It is omitted when
empty; older saves load without it (loading never changes the state), and the
first order after loading posts the current port's board.
