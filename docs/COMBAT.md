# Combat

Status: **first playable slice.** Hulls and weapons are data-driven. Raider
contacts resolve deterministic engagements, damage persists, destroyed ships leave
their fleet, wrecks yield salvage and markets sell hull repairs. A readable combat
report opens after each fight and survives saves.

## Definitions

- Weapons: `config/combat.json` (see [data formats](DATA_FORMATS.md)).
- Hull profiles: the `combat` object of each class in `config/ships.json`.
- Per-ship state: `hull_damage` only. Shields recharge before every engagement,
  so they are not saved. A ship whose damage reaches its hull is destroyed;
  saves reject ships at or beyond that value.
- Resolver: `game-core/src/combat/engagement.rs` (pure); raider contacts and
  applying results: `game-core/src/navigation/hostile.rs`.

Units are abstract points. Engine is a 1–10 rating for evasion and initiative.

| Hull | Size | Hull | Shields | Armor | Engine | Slots | Default loadout |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Freighter | medium | 60 | 10 | 4 | 3 | 1 | Neutron Beam |
| Scout | small | 30 | 15 | 1 | 8 | 1 | Graviton Pulse |
| Patrol ship | medium | 90 | 30 | 6 | 5 | 3 | Photon Singularity Beam, Quantum Entanglement Torpedo, Magnetic Resonance Disruptor |

| Weapon | Damage × shots | Accuracy | vs shields | Armor pierce | Role |
| --- | --- | --- | --- | --- | --- |
| Photon Singularity Beam | 14 × 1 | 90% | 120% | 3 | accurate heavy all-rounder |
| Quantum Entanglement Torpedo | 30 × 1 | 65% | 50% | 8 | unreliable armor breaker; shields blunt it |
| Neutron Beam | 7 × 2 | 85% | 100% | 2 | reliable civilian weapon |
| Graviton Pulse | 5 × 3 | 92% | 150% | 0 | rapid shield stripper |
| Magnetic Resonance Disruptor | 4 × 2 | 90% | 250% | 0 | dedicated shield breaker |

These numbers are first-pass balance. Expect them to change once the resolver exists.

## Resolution rules

Fights resolve when the player chooses **Fight** (or a failed **Evade**) on a
raider contact. One seed is drawn from the saved navigation RNG and stored in the
report, so a reload replays the same fight. At most 8 rounds:

1. Living ships act by engine rating (high first); ties break by a seeded
   initiative roll drawn once per engagement, so neither side always fires first.
2. Every fitted weapon fires `shots` times. Each shot targets the living enemy with
   the least remaining hull + shields (ties: lowest index), so fire focuses.
3. Hit chance = `accuracy_percent − 3 × target engine`, at least 5%.
4. A hit converts damage to shield points at `shield_efficiency_percent`. If the
   shields break, the leftover damage converts back at the same rate (rounded down).
   Low efficiency therefore makes shields soak *more* raw damage.
5. Damage reaching the hull loses `max(armor − armor_penetration, 0)`, deals at
   least 1 point, and never exceeds the remaining hull.

Example: 20 damage at 200% against 10 shields → 5 raw damage breaks the shields,
15 passes; armor 6 with penetration 2 leaves 11 hull damage.

If both sides still have ships after 8 rounds, they **disengage**. Ships that
were already damaged fight with their saved `hull_damage`, at full shields.

## Raider contacts

A travelling fleet's encounter roll becomes a raider contact one time in three.
The group has one raider per two fleet ships (rounded up, at most 3), all scouts,
except that the leader is a patrol hull when the fleet itself fields a patrol ship.
An unarmed trader is never met by a warship.

- **Fight:** resolve the engagement.
- **Evade:** escape chance = `35% + 10 × (slowest fleet engine − fastest raider
  engine)`, clamped to 10–90%. Success ends the contact untouched; failure forces
  the fight. Bypass, Investigate and Assist are rejected for raiders.

After the fight:

- Owned ships keep the damage from the report. Destroyed ships and their cargo are
  removed; an emptied fleet is removed along with its course.
- **Last ship rule:** the player's last owned ship is never destroyed. It survives
  at 1 hull point and its cargo is seized. This avoids a game-over state that the
  design does not yet define.
- **Victory:** each destroyed raider yields 2 ore, loaded into free cargo space
  in fleet order; anything that does not fit is lost.
- Surviving fleets resume their course on the next run.

## Repairs

**Repair hull** in the Market window restores all hull points at the current
market for `repair_point_price` (default 5.00 cr) per point. Payment goes to that
market's treasury, the same way refuelling does.

## Balance checkpoints

These are covered by tests in `combat/engagement_tests.rs` (400 seeds each):

| Matchup | Result |
| --- | --- |
| Freighter vs one scout raider | never lost (some fights disengage) |
| Scout vs scout, patrol vs patrol | 40–60% player wins |

## Not yet implemented

Mid-fight retreat, player target selection, NPC/faction hostility rules, credit
bounties, raider pursuit across runs, loadout changes and partial repairs. See the
[roadmap](../ROADMAP.md), milestones 4 and 5.
