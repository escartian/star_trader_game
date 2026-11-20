# Combat

Status: **turn-based battles you command in a 3D grid view.** Hulls and
weapons are data-driven. Raider contacts resolve deterministic grid battles, damage persists, destroyed ships leave
their fleet, wrecks yield salvage and markets sell hull repairs. A readable combat
report opens after each fight and survives saves.

## Definitions

- Weapons: `config/combat.json` (see [data formats](DATA_FORMATS.md)).
- Hull profiles: the `combat` object of each class in `config/ships.json`.
- Per-ship state: `hull_damage` only. Shields recharge before every engagement,
  so they are not saved. A ship whose damage reaches its hull is destroyed;
  saves reject ships at or beyond that value.
- Battles: `game-core/src/combat/tactical.rs` and `tactical/ai.rs`; shared damage
  rules: `combat/engagement.rs` (all pure); raider contacts and applying results:
  `game-core/src/navigation/hostile.rs`; battles in progress:
  `game-core/src/navigation/battle.rs`; the battle window:
  `game-client/src/battle*` and `session/battle.rs`.

Units are abstract points. Engine is a 1–10 rating for evasion and initiative.

| Hull | Size | Hull | Shields | Armor | Engine | Slots | Default loadout |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Freighter | medium | 60 | 10 | 4 | 3 | 1 | Neutron Beam |
| Scout | small | 30 | 15 | 1 | 8 | 1 | Graviton Pulse |
| Patrol ship | medium | 90 | 30 | 6 | 5 | 3 | Photon Singularity Beam, Quantum Entanglement Torpedo, Magnetic Resonance Disruptor |

| Weapon | Damage × shots | Accuracy | vs shields | Armor pierce | Range | Role |
| --- | --- | --- | --- | --- | --- | --- |
| Photon Singularity Beam | 14 × 1 | 90% | 120% | 3 | 5 | accurate heavy all-rounder |
| Quantum Entanglement Torpedo | 30 × 1 | 65% | 50% | 8 | 6 | unreliable armor breaker; shields blunt it |
| Neutron Beam | 7 × 2 | 85% | 100% | 2 | 4 | reliable civilian weapon |
| Graviton Pulse | 5 × 3 | 92% | 150% | 0 | 3 | rapid shield stripper |
| Magnetic Resonance Disruptor | 4 × 2 | 90% | 250% | 0 | 3 | dedicated shield breaker |

These numbers are first-pass balance. Expect them to change once the resolver exists.

## Resolution rules

Fights are turn-based battles on a 3D grid (`game-core/src/combat/tactical.rs`,
pure and deterministic). One seed is drawn from the saved navigation RNG and
stored in the report, so the same seed and actions replay the same battle.

**Battle space.** 9 cells along x (between the two sides), 9 along y and 5
vertical layers along z. Distance is Chebyshev: one step may change all three
axes, so every movement and weapon range is a cube. Player ships deploy on the
second column (x = 1), raiders on x = 7, nearest the centre first; each side
withdraws from its own edge (x = 0 or x = 8).

**Turns (HoMM-style initiative).** Each round every active ship acts once, by
engine rating (high first); ties break by a seeded roll drawn once per battle.
On its turn a ship may:

1. **Move** once, up to `1 + engine / 3` cells (freighter 2, patrol 2, scout 3),
   to a free cell inside the volume.
2. **Fire** once: every fitted weapon whose `range` reaches the chosen enemy fires
   its `shots`. Hit chance = `accuracy − 3 × target engine − 4 × (distance − 1)`,
   at least 5%. A target destroyed mid-volley takes no further shots.
3. **Withdraw** instead, if it stands on its own edge: it leaves the battle with
   the damage taken so far.
4. **End turn.**

A hit converts damage to shield points at `shield_efficiency_percent`. If the
shields break, the leftover damage converts back at the same rate (rounded down).
Damage reaching the hull loses `max(armor − armor_penetration, 0)`, deals at
least 1 point, and never exceeds the remaining hull. Example: 20 damage at 200%
against 10 shields → 5 raw damage breaks the shields, 15 passes; armor 6 with
penetration 2 leaves 11 hull damage.

The battle ends when a side has no active ships (destroyed or withdrawn), or
after 10 rounds. A side wins only if every enemy ship was destroyed; otherwise
the result is **Disengaged**. Ships that were already damaged fight with their
saved `hull_damage`, at full shields.

**AI and Auto-resolve.** Raiders, and both sides under Auto-resolve, focus the
active enemy with the least hull + shields, move to the reachable cell nearest it
when no weapon reaches, then fire on the weakest enemy in range.

## Playing a battle

**Fight** and a failed **Evade** open the battle window. The open battle is
saved with the session (`navigation.battle`) and its contact stays pending, so
navigation runs wait for it. The server plays raider turns right after each
player action, so a saved battle is always waiting on one of your ships.

- The view is a 3D grid: your edge is the cyan strip on the left, the raiders'
  edge the red strip on the right. Ships stand on stalks over the floor so their
  layer reads at a glance; bars show hull and shields. Drag to orbit, scroll or
  pinch to zoom.
- **Move:** green dots are the free cells your ship can reach on the shown layer.
  **Lower** / **Higher** (or Q / E) change the layer; it follows your ship at the
  start of each turn. Click a dot to move.
- **Fire:** raiders your weapons reach have a red ring. Hover one to see each
  weapon's hit chance at that distance; click it to fire.
- **Withdraw** from your edge column, **End turn**, or **Auto-resolve** to let
  the AI fight every remaining turn on both sides.
- The turn-order bar lists every ship, outlines the one acting and marks lost or
  departed ships. The last shots are listed below the grid.
- Hide the window (×) to look at the map; **Return to battle** in the contact
  panel brings it back. The combat report opens when the battle ends.

Freighters and patrol ships are drawn as their real 3D models (see the asset
notes); the scout still uses its sprite until it has a lighter mesh.

## Raider contacts

A travelling fleet's encounter roll becomes a raider contact one time in three.
The group has one raider per two fleet ships (rounded up, at most 3), all scouts,
except that the leader is a patrol hull when the fleet itself fields a patrol ship.
An unarmed trader is never met by a warship.

- **Fight:** open the battle and command your ships turn by turn.
- **Auto-resolve:** fight the whole battle at once with the AI on both sides.
- **Evade:** escape chance = `35% + 10 × (slowest fleet engine − fastest raider
  engine)`, clamped to 10–90%. Success ends the contact untouched; failure opens
  the battle. Bypass, Investigate and Assist are rejected for raiders.

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

These are covered by Auto-resolve tests in `combat/tactical/tests.rs` (400 seeds each):

| Matchup | Result |
| --- | --- |
| Freighter vs one scout raider | never lost (some fights disengage) |
| Scout vs scout, patrol vs patrol | 40–60% player wins |

## Not yet implemented

Animated raider turns and weapon effects, 3D models for every hull with a GPU
renderer, NPC/faction hostility rules, credit
bounties, raider pursuit across runs, loadout changes and partial repairs. See the
[roadmap](../ROADMAP.md), milestones 4 and 5.
