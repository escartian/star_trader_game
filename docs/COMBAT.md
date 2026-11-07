# Combat

Status: **data foundation only.** Hulls and weapons are defined, validated, saved
and shown in the Fleets/shipyard window. No engagement is resolved yet.

## Definitions

- Weapons: `config/combat.json` (see [data formats](DATA_FORMATS.md)).
- Hull profiles: the `combat` object of each class in `config/ships.json`.
- Per-ship state: `hull_damage` only. Shields recharge before every engagement,
  so they are not saved. A ship whose damage would reach its hull is destroyed;
  saves reject ships at or beyond that value.

Units are abstract points. Engine is a 1–10 rating for evasion and initiative.

| Hull | Size | Hull | Shields | Armor | Engine | Slots | Default loadout |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Freighter | medium | 60 | 10 | 4 | 3 | 1 | Neutron Beam |
| Scout | small | 30 | 15 | 1 | 8 | 1 | Graviton Pulse |
| Patrol ship | medium | 90 | 30 | 6 | 5 | 3 | Photon Singularity Beam, Quantum Entanglement Torpedo, Magnetic Resonance Disruptor |

| Weapon | Damage × shots | Accuracy | vs shields | Armor pierce | Role |
| --- | --- | --- | --- | --- | --- |
| Photon Singularity Beam | 14 × 1 | 90% | 120% | 3 | accurate heavy all-rounder |
| Quantum Entanglement Torpedo | 30 × 1 | 65% | 50% | 8 | unreliable armor breaker that slips past shields |
| Neutron Beam | 7 × 2 | 85% | 100% | 2 | reliable civilian weapon |
| Graviton Pulse | 5 × 3 | 92% | 150% | 0 | rapid shield stripper |
| Magnetic Resonance Disruptor | 4 × 2 | 90% | 250% | 0 | dedicated shield breaker |

These numbers are first-pass balance. Expect them to change once the resolver exists.

## Intended resolution (next slice, not implemented)

Fights resolve at navigation-run boundaries, deterministically from the saved
seeded RNG, in rounds. Within a round, ships act in engine order (ties broken by
ID). For each fitted weapon, each shot:

1. Rolls against `accuracy_percent`, reduced by the target's engine rating.
2. On a hit, applies `damage × shield_efficiency_percent / 100` to remaining
   shields. Damage the shields cannot absorb converts back at the same rate.
3. Reduces whatever reaches the hull by `max(armor − armor_penetration, 0)`,
   with a minimum of 1, and adds it to `hull_damage`.

Destroyed ships leave their fleet, and their cargo may become salvage. Retreat,
target selection, hostility/engagement rules and a readable combat report are
also part of the next slices (see [roadmap](../ROADMAP.md), milestone 4).
