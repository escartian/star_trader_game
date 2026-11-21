# Factions

Four powers divide the galaxy: the **Federation**, the **Empire**, the
**Republic** and the **Alliance**. They hold territory, they are allied with or
rival to each other, and each keeps a standing with you that sets your terms at
its ports. Rules live in `game-core/src/factions*`; definitions are JSON content
in `config/factions.json`.

## Setting up a game

The new-game screen (the game chooser, and Session → new game) has four faction
rows below the campaign choices:

| Row | Choices | Default |
| --- | --- | --- |
| Factions | a checkbox per faction; clear all four for a galaxy without factions | all four |
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

The galaxy map rings each claimed system in its holder's colour. Hovering a
system names its holder and your standing there; the destination card shows the
holder's name.

## Relations

Pairs of factions are allied, neutral or rivals. Classic relations
(`classic_relations` in the catalog):

| | Federation | Empire | Republic | Alliance |
| --- | --- | --- | --- | --- |
| Federation | | rival | allied | neutral |
| Empire | rival | | neutral | rival |
| Republic | allied | neutral | | allied |
| Alliance | neutral | rival | allied | |

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

Independent space neither costs nor earns standing. NPC traders' own trades
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

The adjustment applies to a whole order's total (buying and services rounded
up, selling rounded down), and the market's treasury receives or pays exactly
that amount, so credits are conserved. Terms apply to your ships only; NPC
traders trade at list prices. A hostile port still sells fuel and repairs so you
are never stranded. The Market window shows the port's holder and your terms,
and its prices already include them.

## The Factions panel

Menu → Factions (shown only when factions are in play) lists, for each faction:
its full name and temperament, the share of systems it holds, your standing and
tier with a bar, the terms that tier gives, its piracy/raider/sales rates and
its alliances and rivalries.

## Saves

`economy.factions` holds `in_play`, `allegiance`, `territory` (system and
holder pairs), non-neutral `relations`, `standing` (four numbers in faction
order) and `sales_cents` (sales not yet worth a point). The definitions travel
in the economy rules as `faction_catalog`. Saves without these fields load with
no factions in play. Loading checks that claimed systems exist and are claimed
once, that only factions in play hold territory, relations or standing, that
standing is in range and that relations name each pair once.

## Not yet implemented

Territory that shifts with influence, faction fleets and police, inspections,
wanted status from piracy, faction hostility to your fleets, missions and
faction-specific goods. See the [roadmap](../ROADMAP.md), milestone 5.
