# Factions

Four political powers divide new galaxies: the **Federation**, **Empire**,
**Republic** and **Alliance**. They hold territory, they are allied with or
rival to each other, and each keeps a standing with you that sets your terms at
its ports. Human, Miri and Auralith residents live across all four powers;
species is chosen separately and does not determine allegiance. Rules live in `crates/game-core/src/factions*`; definitions are JSON content
in `config/factions.json`.

## Setting up a game

The new-game screen (the game chooser, and Session → new game) has four faction
rows below the campaign choices:

| Row | Choices | Default |
| --- | --- | --- |
| Factions | a checkbox per political power; clear all four for a galaxy without factions | all four |
| Allegiance | none, or one faction in play: your home system becomes its capital and you start Friendly with it | none |
| Territory | Sparse (40% of systems claimed), Standard (70%) or Full (every system) | Standard |
| Relations | Classic (the catalog's alliances and rivalries) or Random (rolled from the map seed) | Classic |

Plain desktop and browser launches without the chooser start with the
defaults. Captures and tests start without factions unless they set them up
(`Session::set_up_factions`).

## Territory

- Every faction in play gets a capital. Your allegiance faction's capital is
  your home system; each other capital is the system farthest from home and the
  capitals already placed, so the powers start apart. A galaxy with fewer
  systems than capitals leaves the last factions without territory.
- Every system leans to its nearest capital. The territory share of systems
  nearest their capitals is claimed; the rest are **independent**.
- Home is independent unless it is a capital.
- Influence is the share of all systems a faction holds (shown in the panel).
  Territory does not change during play yet.

The galaxy map draws each connected group of a faction's nearby claimed
systems as one translucent territory with smooth, rounded edges. Each star
spreads an influence that fades out at 14 ly; a point belongs to the faction
whose stars' summed influence there is strongest and strong enough, so a lone
holding reaches about ten light-years, holdings up to about 20 ly apart merge
into one region, borders between rivals run near the midway line, and
independent systems cut into the region (`crates/game-client/src/map/territory.rs`;
outlines are traced on a 1.25 ly grid and cached until ownership changes). The
shield button in the galaxy map's button column hides or shows the
territories; the choice is saved with the display settings. These outlines show ownership only and do not
change travel or faction influence. Hovering a system names its holder and your
standing there; the destination card shows the holder's name.

## Relations

Pairs of factions are allied, neutral or rivals. Classic relations
(`classic_relations` in the catalog) make Federation–Republic,
Republic–Alliance allies. Federation–Empire and Empire–Alliance are rivals.
All other pairs are neutral. Older saves made when Miri and Auralith were
political factions retain those factions and their story chains, but new games
choose them as species instead (see [species](SPECIES.md)).

Random relations make each pair allied one time in four, rivals one time in
four and neutral otherwise.

## Standing

Standing with each faction runs from −100 to 100. **Every change spreads**: half
of it (rounded toward zero) also applies to the faction's allies, and the same
half against you to its rivals. Declaring allegiance starts you at +30 with that
faction, so +15 with its allies and −15 with its rivals.

| What you do | Where | Change |
| --- | --- | --- |
| Demand cargo from or attack an NPC trader (each act of piracy) | a faction's space | − its `piracy_penalty` |
| Destroy raiders in a battle (bands, random raiders, the Haven) | a faction's space | + its `raider_kill_standing` per raider destroyed |
| Sell cargo at a market (your ships, including trade routes) | a faction's space | +1 per `sale_credits_per_point` of sales, carried over between sales |
| Win a battle answering a trader's distress call | anywhere | +3 (`rescue_standing` in `config/npc.json`) with the trader's own faction |

Robbing a trader that flies for a faction (see
[traffic](MAP_CONTROLS.md)) also costs *that* faction its own `piracy_penalty`
and makes you wanted by it, unless it already holds the system; this spreads
like any change. Independent space neither costs nor earns standing. Destroying a faction's
police costs its `piracy_penalty` per patrol ship destroyed (see
[the law](#the-law-wanted-status-and-police)). NPC traders' own trades
never change your standing. Every change is announced in the log, for example
"Federation standing −25 → −25 (Unfriendly). Your terms at their ports changed."

| Faction | Piracy | Per raider | Sales per point |
| --- | --- | --- | --- |
| Federation | −25 | +2 | 5,000 cr |
| Empire | −15 | +4 | 10,000 cr |
| Republic | −15 | +1 | 2,000 cr |
| Alliance | −8 | +3 | 5,000 cr |

## Terms at a faction's ports

| Tier | Standing | Buying goods | Selling goods | Fuel and repairs | Hulls, gear, escorts, hold upgrade, charters |
| --- | --- | --- | --- | --- | --- |
| Hostile | −100 to −50 | refused | refused | +50% | refused |
| Unfriendly | −49 to −15 | +10% | −10% | +10% | open |
| Neutral | −14 to 14 | list | list | list | open |
| Friendly | 15 to 49 | −5% | +5% | −5% | open |
| Honored | 50 to 100 | −10% | +10% | −10% | open |

While a faction **wants** you, its ports treat you as Hostile whatever your
standing, until you pay your fine (below).

The adjustment applies to a whole order's total (buying and services rounded
up, selling rounded down), and the market's treasury receives or pays exactly
that amount, so credits are conserved. Terms apply to your ships only; NPC
traders trade at list prices. A hostile port still sells fuel and repairs so you
are never stranded. The Market window shows the port's holder and your terms,
and its prices already include them.

## The law: wanted status and police

Rules live in `crates/game-core/src/factions/wanted.rs` and `crates/game-core/src/police*`.

**Wanted status.** Each crime in a faction's space adds its `wanted_runs` to
your wanted status with it (at most 200 runs); every navigation run takes one
off. Crimes are each act of piracy (demanding cargo from or attacking an NPC
trader) and choosing to fight its police. While wanted:

- its ports refuse trade, hulls, gear, escorts and charters, and charge 50% more
  for fuel and repairs, exactly as at Hostile standing;
- its police patrols pursue your fleets in its space.

Hostile standing alone also sets the police on you, but does not make you wanted.

**The fine** is `fine_per_run` for every wanted run left, never less than 10
runs' worth, at today's prices (it rises with inflation). Pay it at any of the
faction's ports (**Pay fine** in the Market window; it goes to that port's
treasury) or to a patrol that stops you. Paying clears your wanted status; it
does not restore standing.

| Faction | Wanted per crime | Fine per run left | Fine for one crime |
| --- | --- | --- | --- |
| Federation | 40 runs | 15 cr | 600 cr |
| Empire | 30 runs | 10 cr | 300 cr |
| Republic | 25 runs | 10 cr | 250 cr |
| Alliance | 20 runs | 5 cr | 100 cr |

**Patrols.** Faction setup places one patrol, a patrol ship leading a scout,
in every claimed system. A patrol stays in its system and visits its bodies in
turn. Each run, after fleets, raider bands and NPC traders move:

1. If the faction pursues you, the patrol hunts your nearest fleet out in the
   open within 150 million km (docked and landed fleets are safe, as from
   raiders) and chases it on a curve at a raider band's pace (as raiders do; see
   [combat](COMBAT.md)). Within 500,000 km it stops the fleet: a **Police
   patrol** contact.
2. Otherwise it answers the nearest raider band in its system that is hunting a
   fleet or stalking a trader (and has no strike on you pending), and fights it
   on reaching it, auto-resolved. Both sides lose the ships destroyed; the band
   backs off for 3 runs, the patrol rests for 6. If the band was hunting one of
   your fleets, the log tells you.
3. Otherwise it flies on to its next body.

A patrol backs off for 3 runs after a contact with you. A patrol wiped out is
replaced after 40 runs (twice a raider band's wait) while its faction still holds the system. Patrol ships
repair between fights.

**At a Police patrol contact** the card names the faction, why it stopped you,
the fine and the consequences:

| Choice | Result |
| --- | --- |
| Comply · fine | Pay the fine; if the wallet falls short, the patrol takes what you have and impounds every cargo unit aboard the fleet. Your record is cleared. |
| Fight / Auto-resolve | A tactical battle against the patrol's ships. Choosing it is a crime (wanted again). Each patrol ship destroyed costs standing as above. No bounty, no salvage beyond the usual ore. Surrendering in the battle impounds your cargo and clears your record. |
| Evade N% | The raiders' engine roll; failure opens the battle (not a further crime). Escaping leaves your record as it was. |

Secondary fleets always comply, paying from your wallet.

The system map draws each patrol as a ship marker (its heaviest hull, with a
count) ringed friend-or-foe (red while it pursues you) with a pip in its
faction's colour, tagged with its ships, and a dashed line toward the fleet it is pursuing ("PURSUING
YOU") or the band it is engaging. The Factions panel shows your record with each
faction and its rates; the log announces when a faction starts wanting you, when
the status grows and when it ends.

## The Factions panel

Menu → Factions (shown only when factions are in play) lists, for each faction:
its full name and temperament, the share of systems it holds, your standing and
tier with a bar, the terms that tier gives, your record (clean, wanted with runs
left and the fine, or Hostile), its wanted and fine rates, its
piracy/raider/sales rates and its alliances and rivalries.

## Saves

`economy.factions` holds `in_play`, `allegiance`, `territory` (system and
holder pairs), non-neutral `relations`, `standing` (four numbers in faction
order), `sales_cents` (sales not yet worth a point) and `wanted` (runs left,
four numbers in faction order). Police patrols are in
`player.navigation.police`: `patrols` (each with `id`, `faction`, `system`,
`position`, `heading`, `ships`, optional `pursuing` fleet or `answering` band,
`stops` and `cooldown`), `respawns` (system, run) and `next_id`. A police
contact carries `police` (`patrol`, `faction`, and `intercepted: true` when
your flagship ran the patrol down; see [pursuit](NAVIGATION_RUNS.md#pursuit)); a combat report against police
carries `police` (the faction). The definitions travel in the economy rules as
`faction_catalog`. Each NPC trader may carry `affiliation` (its faction) and `home` (its home
port's market id); without them it is independent and its home port is the
`id`-th planet market. Saves without these fields load with
no factions in play. Loading checks that claimed systems exist and are claimed
once, that only factions in play hold territory, relations or standing, that
standing is in range and that relations name each pair once, that traders fly
only for factions in play and call a planet market home, that wanted status
is at most 200 runs and only with factions in play, and that every patrol (and
pending replacement) is in a system its faction holds, one per system.

## Not yet implemented

Territory that shifts with influence, faction war fleets and wars between
factions, police patrols crossing systems, bribes, inspections that search for
contraband, bounty hunters, missions and faction-specific goods. See the
[roadmap](../ROADMAP.md), milestone 5.
