# Campaign

Status: **victory framework implemented** (phone-first phase 5). The game has
roads to victory, a score, an optional campaign length, a Raider Haven to
destroy and port charters to buy. Diplomat/Spy and Pirate roads wait for
factions and missions (milestones 5 and 6).

Code: `game-core/src/campaign.rs` (roads, net worth, score, ends),
`economy/charters.rs` (charters, port share, lanes), `navigation/haven.rs`
(the Haven); commands and the end-of-game gate in `game-server/src/campaign.rs`;
the Victory panel, end card, charter row and Assault prompt in
`game-client/src/session/victory.rs`; map marks in `map/local/campaign.rs` and
`map/galaxy_campaign.rs`.

## Roads to victory

The first road completed wins. A win may be played on; nothing more is scored
toward a second road.

| Road | Win condition | Status |
| --- | --- | --- |
| Trader | Net worth of **25,000 cr**, or **3 port charters** | Playable |
| Soldier | Destroy the Raider Haven | Playable |
| Diplomat / Spy | Highest standing with a great power, then the frontier treaty | Locked until factions and missions |
| Pirate | Take over the Raider Haven | Locked until factions and missions |

**Why 25,000 cr and not the 1,000,000 cr first proposed:** trades only move
money between wallets, and a new world holds about 64,000 cr in all (about
59,000 cr in nine market treasuries, 4,000 cr with NPC traders, 1,000 cr with
the player); bounties are the only new money. The goal, charter price, charter
count and port share live in the session rules (`economy.rules.campaign`), so
they can be rebalanced in data.

**Net worth** = credits + each owned hull at its resale price (60% of the
shipyard price) + cargo at the best price any market currently pays.

## Score

Shown in the Victory panel and used when a campaign length runs out (and later
for multiplayer):

- 1 point per 100 cr of net worth;
- 100 points per port charter;
- 20 points per Haven garrison ship sunk, and 500 for destroying the Haven.

Faction standing will add points once factions exist.

## Ends

- **Victory:** a road completed. The end card offers *Keep playing*.
- **Defeat:** your last ship was destroyed. Fleeing is always possible
  (Evade, withdrawing from your edge, Surrender), so losing it means a lost
  battle. The game freezes: only queries, saves and loads work. The wreck stays
  on record at 1 hull point so the final state still saves and loads. This
  replaces the earlier rule that spared the last ship crippled.
- **Campaign complete:** with a campaign length set (Victory panel: 200, 400 or
  800 runs; open-ended by default), the game ends on score at that run and
  freezes. The length can be changed while the campaign runs, to any later run.

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
- Faction standing in the score; Diplomat/Spy and Pirate roads.
- The Haven's own defences as a distinct hull (it fields existing hulls today).
- Balance from playtests: the Trader goals, charter price and garrison size are
  first-pass numbers.
