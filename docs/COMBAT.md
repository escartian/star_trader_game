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

**Battle space.** A cube that grows with the fleets: `9 + ships / 2` cells a
side, so 10 for a duel (and for 3 ships), 11 for 4–5 ships, 16 for 7 v 7 and 17
for the largest battle (16 ships). x runs between the two sides, y across them,
z is the vertical layers. Battles saved before grids scaled keep their 9 × 9 × 5
volume. Distance is Chebyshev: one step may change all three axes, so every
movement and weapon range is a cube. Player ships deploy on the second column
(x = 1), raiders one column in from the far edge, in a checkerboard around the
centre (centre first, then the middle layer, then the layers beside it) so no two
ships of a side share a face; each side withdraws from its own edge column.

Bigger grids mean a longer approach: ships start `edge − 3` cells apart (7 in a
duel, 13 at 7 v 7), so slow hulls spend a round or two closing before weapons
reach, and long-range weapons (torpedo 6, photon beam 5) get the first shots.

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

Battles are fought to a finish: there is no round limit. A battle ends when a
side has no active ships (destroyed or withdrawn). It also ends if three whole
rounds pass in which nobody fires, moves or withdraws (a ceasefire: both sides
break off); movement counts, so a long approach never trips it. Outcomes:

- **Victory:** no raider is left fighting, whether destroyed or fled.
- **Defeat:** every one of your ships was destroyed.
- **Disengaged:** your ships escaped by withdrawing, or a ceasefire.
- **Surrendered:** you gave up on one of your turns. The raiders seize every
  cargo unit aboard the fleet and let its ships go with the damage they have;
  nothing is destroyed and the course resumes.

A 200-round safety cap bounds saved shot logs; normal play never reaches it,
because every shot has at least a 5% hit chance and every hull hit deals at
least 1 point. Ships that were already damaged fight with their saved
`hull_damage`, at full shields.

**Morale.** Each side has a morale of 0–100, derived from the battle so far
(`tactical/morale.rs`):

`100 − 50 × share of own hull lost − 30 × share of own ships destroyed
− 20 × share of own ships withdrawn + 15 × share of enemy ships destroyed`.

Below 50 the side **breaks**. The header shows both sides' morale.

**AI and Auto-resolve.** Raiders, and both sides under Auto-resolve, focus the
active enemy with the least hull + shields, move to the reachable cell nearest it
when no weapon reaches, then fire on the weakest enemy in range. A ship with 30%
of its hull or less retreats, and so does every ship of a broken side (a rout):
it fires a parting shot at the weakest enemy in reach, moves toward its own edge
(away from enemies on ties) and withdraws as soon as it is there. Ships you
command yourself never retreat on their own.

With Auto-resolve over 400 seeds:

| Matchup | You win | You lose | You escape | Avg rounds (max) | Ships lost / killed |
| --- | --- | --- | --- | --- | --- |
| Freighter v scout | 400 | 0 | 0 | 6.6 (12) | 0 / 0.8 |
| Scout v scout | 200 | 112 | 88 | 5.3 (7) | 0.3 / 0.3 |
| Patrol v patrol | 198 | 61 | 141 | 5.9 (13) | 0.2 / 0.2 |
| 7 v 7 fleet-battle scenario | 271 | 8 | 121 | 12.5 (18) | 5.1 / 5.8 |
| 8 patrols v 8 patrols | 205 | 1 | 194 | 19.9 (34) | 5.3 / 5.5 |

Mirrors are even: each side is driven from the field about half the time.
Big battles stay bloody (retreating ships are often caught), but almost never
end in your fleet's destruction.

## Playing a battle

**Fight** and a failed **Evade** open the battle screen, which fills the whole
window until the battle ends: fight it out, withdraw, auto-resolve or surrender.
The open battle is
saved with the session (`navigation.battle`) and its contact stays pending, so
navigation runs wait for it. The server plays raider turns right after each
player action, so a saved battle is always waiting on one of your ships.

- The header shows the round and each side's remaining hull and ships.
- The view is a 3D grid over a starfield: your edge is the cyan strip on the
  left, the raiders' edge the red strip on the right. Ships stand on dashed
  stalks over the floor so their layer reads at a glance; rings lie in the
  ship's own layer, and bars show hull and shields.
- **Camera:** drag to orbit; right-drag, middle-drag or Shift+drag to pan; scroll
  or pinch to zoom (from half to six times the framing distance); arrow keys
  slide the view and Page Up / Page Down raise and lower it. **F** (or the
  crosshair button over the grid) centres on the acting ship, **R** (or the reset
  button) restores the default view. The view never leaves the grid. While a
  battle is open it owns these keys, so F and Escape do not reach the map
  behind it.
- **Move:** green dots are the free cells your ship can reach on the shown layer.
  The **Layer** slider (or Q / E) changes the layer; it follows your ship at
  the start of each turn. Hovering a dot draws the path and dashed lines to every
  raider your weapons would reach from there. Click a dot to move.
- **Fire:** raiders your weapons reach have a red ring. Hovering any ship shows
  a card with its condition and, for a raider, the distance and each weapon's
  hit chance; click a ringed raider to fire.
- **End turn** (or Space), **Withdraw** from your edge column, or
  **Auto-resolve** to let the AI fight every remaining turn on both sides.
- Moves glide and shots fly as beams, flashing cyan on shields, amber on hulls
  and bursting when a ship is destroyed, so raider turns played by the server are
  visible. The animation is display-only; captures turn it off.
- The turn-order bar shows every ship with its portrait and hull, lights the one
  acting and marks lost or departed ships. The ship card shows whether it can
  still move and fire this turn, and the combat log lists the latest shots.
- **×** (or Escape) asks whether to surrender, naming the cost; **SURRENDER**
  confirms, **Keep fighting** (or Escape again) backs out. The combat report
  opens when the battle ends.

Every hull is drawn as its real 3D model (see the asset notes); sprites remain
as the fallback and for portraits.

**Large battles.** Play caps raider groups at three, but battles hold up to 16
ships. The developer scenario `--scenario fleet-battle` (dev-tools build) opens a
7 v 7 battle on your first turn: a strike group of three freighters, two patrol
ships and two scouts against two raider patrols and five scouts. Saves accept up
to 8 raiders per contact for such fixtures. With more than six ships the
turn-order bar switches to compact portrait chips (names on hover).

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
  in fleet order; anything that does not fit is lost. Raiders that fled leave
  no wreck to salvage.
- **Bounty:** port authorities pay for every raider destroyed, whatever the
  outcome: scout 150.00 cr, patrol 400.00 cr, freighter 100.00 cr
  (`economy.json` → `raider_bounties`). A raider that fled pays nothing. The
  bounty is shown in the combat report and credited once, when the battle ends.
  It is a reward from outside the market economy, not a trade.
- Surviving fleets resume their course on the next run.

## NPC traders and raiders

NPC traders face raiders too. After each run in which a trader was under way,
it is attacked with `npc_raid_chance_percent` (default 4%: the player's 12%
contact chance × one contact in three being raiders) by one scout, or two one
time in three. The fight is the same tactical battle, auto-resolved for both
sides, so retreats apply:

- **Fought off:** the trader keeps its cargo.
- **Escaped:** the trader dumped its whole cargo to get away.
- **Destroyed:** the cargo is lost and the trader is refitted at its home port,
  out of service for 8 runs before trading again.

Damage sticks until the trader next stops at a market, where it repairs from its
own wallet into the market's treasury. Results show in the traffic panel (raid
news in amber). Over 400 runs the two starting traders met about 18 raids;
a few cost them their cargo, and a single freighter is very rarely destroyed
because it withdraws when badly damaged.

## Repairs

**Repair hull** in the Market window restores all hull points at the current
market for `repair_point_price` (default 5.00 cr) per point. Payment goes to that
market's treasury, the same way refuelling does.

## Balance checkpoints

These are covered by Auto-resolve tests in `combat/tactical/tests.rs` (400 seeds each):

| Matchup | Result |
| --- | --- |
| Freighter vs one scout raider | never lost |
| Scout vs scout, patrol vs patrol | player holds the field 40–60% of the time |
| 8 patrols a side | ends with a side off the field in under 50 rounds |

## Not yet implemented

A GPU renderer with a depth buffer, morale effects on accuracy, faction
hostility rules, persistent raider fleets that roam and pursue across runs,
attacking or hailing NPC traders, loadout changes and partial repairs. See the
[roadmap](../ROADMAP.md), milestones 4 and 5.
