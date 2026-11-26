# Campaign

Status: **victory framework implemented** (phone-first phase 5). The game has
roads to victory, a score, an optional campaign length, a Raider Haven to
destroy, port charters to buy and a Diplomat road through faction story
quests ([JOBS.md](JOBS.md#story-quests)), and a Pirate road through the Free
Captains' chain ([JOBS.md](JOBS.md#the-free-captains)).

Code: `game-core/src/campaign.rs` (roads, net worth, score, ends),
`economy/charters.rs` (charters, port share, lanes), `navigation/haven.rs`
(the Haven); commands and the end-of-game gate in `game-server/src/campaign.rs`;
the Victory panel, charter row and Assault prompt in
`game-client/src/session/victory.rs`; the end screen in `session/campaign_end.rs`; map marks in `map/local/campaign.rs` and
`map/galaxy_campaign.rs`.

## Roads to victory

The first road completed wins. A win may be played on; nothing more is scored
toward a second road.

| Road | Win condition | Status |
| --- | --- | --- |
| Trader | Net worth of **10% of all galaxy wealth**, or **3 port charters** | Playable |
| Soldier | Destroy the Raider Haven | Playable |
| Diplomat | Carry a great power's treaty: the last step of its story quests, offered at Honored standing | Playable with factions |
| Pirate | Complete the Free Captains' story chain: its steps open with successful cargo seizures from traders and, with factions, a police escape | Playable with NPC traders |

**Galaxy wealth** = your net worth + every market treasury + every NPC
trader's credits. The goal is a share rather than a sum (the 1,000,000 cr first
proposed, and the 25,000 cr of the first build, were both fixed), so it scales
with the number of systems and planets and will follow economies as they grow,
shrink or inflate. In today's three-system default world (about 64,000 cr in
all) 10% means roughly 7,000 cr of net worth; a bigger galaxy asks for more.
The share, charter price, charter count and port share live in the session
rules (`economy.rules.campaign`), so they can be rebalanced in data.
Galaxy wealth changes over a game: markets grow and shrink with trade and
prices inflate (see [planet economies](PLANET_ECONOMY.md#living-markets-and-inflation)).

**Net worth** = credits + each owned ship at its sale price (60% of the
shipyard price, plus 60% of today's price for each fitted upgrade and a bought
hold upgrade) + cargo at the best price any market currently pays.

## Score

Shown in the Victory panel and used when a campaign length runs out (and later
for multiplayer):

- 1 point per 100 cr of net worth;
- 100 points per port charter;
- 20 points per Haven garrison ship sunk, and 500 for destroying the Haven;
- 2 points per point of positive standing with each faction, and 50 per story
  quest step completed (*diplomacy* in the Victory panel).


## Ends

Each end opens the end screen once, after any open combat report: an
illustrated card over the dimmed map with how it ended, runs played, net
worth, the score and its parts, each road's progress and faction standings.

- **Victory:** a road completed. The end screen offers *Keep playing*.
- **Defeat:** your last ship was destroyed. Fleeing is always possible
  (Evade, withdrawing from your edge, Surrender), so losing it means a lost
  battle. The game freezes: only queries, saves and loads work. The wreck stays
  on record at 1 hull point so the final state still saves and loads. This
  replaces the earlier rule that spared the last ship crippled. Every fleet
  counts: losing one fleet while another still flies is not a defeat. The
  **GAME OVER** screen shows the last ship facing what destroyed it and offers
  *Start a new game*, *Load a save*, *Main menu* (without saving over the
  autosave) or *View the final map*. Loading a lost game shows it again.
- **Campaign complete:** with a campaign length set (Victory panel: 200, 400 or
  800 runs; open-ended by default), the game ends on score at that run and
  freezes, with the same choices as a defeat. The length can be changed while the campaign runs, to any later run.

## The Raider Haven

A fortified raider base. New games place it in the system farthest from home,
beside that system's most dangerous body that is not a station; a one-system
galaxy has none. Placement is deterministic, so saves from before the Haven
get the same one on load.

- Its garrison is three patrol ships and five scouts, the largest raider
  group a battle holds.
- Bring a fleet into orbit around (or onto the surface of) the Haven's body;
  the sheet shows **ASSAULT**. The contact card then offers Fight,
  Auto-resolve or **Call off** (always succeeds).
- Garrison ships you destroy stay lost; there is no rebuilding, so a Haven can
  be worn down over several assaults. When none are left, it falls: the
  Soldier road is won.
- While the Haven stands, wiped-out raider bands return after 20 runs; once it
  falls they never return.
- The map shows it as a red fortress beside its body (with its garrison) and
  beside its system in the galaxy view; struck through and grey once destroyed.

## Port charters

A port authority's licence, bought once in the port's market (**Buy charter**,
5,000 cr; the price goes to the port's treasury). Each port has one charter.

- **Port share:** the holder receives 2% of the value of every trade at the
  port, NPC traders' included, paid from the port's treasury (never more than
  it holds). Credits are conserved.
- **Shipping lanes:** a trade route between two of your chartered ports is never
  struck by raiders: no random raider contact, and bands do not hunt the fleet.
  Lanes are drawn as bold cyan lines between those ports (between systems in
  the galaxy view, between bodies in the system view).
- Three charters win the Trader road.

## Not yet

- Unlock order and guide cards for these systems (phase 6).
- The Spy side of the Diplomat road.
- The Haven's own defences as a distinct hull (it fields existing hulls today).
- Balance from playtests: the Trader goals, charter price and garrison size are
  first-pass numbers.
