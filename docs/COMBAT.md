# Combat

Status: **turn-based battles you command on a hex board with three altitude
bands.** Hulls and weapons are data-driven. Raider contacts resolve
deterministic battles, damage persists, destroyed ships leave their fleet,
wrecks yield salvage and markets sell hull repairs. A readable combat
report opens after each fight and survives saves.

## Definitions

- Weapons: `config/combat.json` (see [data formats](DATA_FORMATS.md)).
- Hull profiles: the `combat` object of each class in `config/ships.json`.
- Per-ship state: `hull_damage` only. Shields recharge before every engagement,
  so they are not saved. A ship whose damage reaches its hull is destroyed;
  saves reject ships at or beyond that value.
- Battles: `game-core/src/combat/tactical.rs`, `tactical/board.rs` (hexes,
  bands, ranges), `tactical/actions.rs` and `tactical/ai.rs`; shared damage
  rules: `combat/engagement.rs` (all pure); raider contacts and applying results:
  `game-core/src/navigation/hostile.rs`; battles in progress:
  `game-core/src/navigation/battle.rs`; the battle window:
  `game-client/src/battle*`, `session/battle.rs` and `session/battle_controls.rs`.

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

Fights are turn-based battles on a hex board (`game-core/src/combat/tactical.rs`,
pure and deterministic). One seed is drawn from the saved navigation RNG and
stored in the report, so the same seed and actions replay the same battle.

**Board.** A hexagon of hexes in axial coordinates that grows with the fleets:
radius 4 (61 hexes) for up to three ships, one more ring per four more ships
(`4 + ships / 4`), at most radius 6 (127 hexes) from 8 ships. Columns run
between the two sides; each side deploys on the column one in from its own
edge, middle row first, and withdraws from its edge column. That column holds
`radius + 2` ships, always enough for a side. A duel opens six hexes apart.

**Altitude bands.** Every ship flies in one of three bands: Low, Level or High.
Ships deploy on Level. One ship per hex whatever its band, so every ship is one
tap target. **Range** is hex steps plus bands crossed.

**Turns (HoMM-style initiative).** Each round every active ship acts once, by
engine rating (high first); ties break by a seeded roll drawn once per battle.
On its turn a ship may:

1. **Move** with `1 + engine / 3` movement points (freighter 2, patrol 2, scout
   3): each hex step costs 1, and so does each **Climb** or **Dive** of one band.
   Points may be spent in several moves (a step, then a climb) to free hexes on
   the board.
2. **Fire** once: every fitted weapon whose `range` reaches the chosen enemy fires
   its `shots`. Hit chance = `accuracy − 3 × target engine − 4 × (range − 1)`,
   **+10%** when firing at a lower band and **−10%** at a higher one (a flat
   modifier, not per band), at least 5% and at most 100%. A target destroyed
   mid-volley takes no further shots.
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
active enemy with the least hull + shields and close in until even their
shortest-ranged weapon reaches it (on ties, the higher band). Before firing on
the weakest enemy in range they climb one band when that costs nothing: a
movement point is spare, the target is not below them, and no weapon would lose
its reach. A ship with 30%
of its hull or less retreats, and so does every ship of a broken side (a rout):
it fires a parting shot at the weakest enemy in reach, moves toward its own edge
(away from enemies on ties) and withdraws as soon as it is there. Ships you
command yourself never retreat on their own.

With Auto-resolve over 400 seeds on the hex board:

| Matchup | You win | You lose | You escape | Avg rounds (max) | Ships lost / killed |
| --- | --- | --- | --- | --- | --- |
| Freighter v scout | 400 | 0 | 0 | 6.0 (10) | 0 / 0.8 |
| Scout v scout | 204 | 48 | 148 | 4.5 (7) | 0.1 / 0.2 |
| Patrol v patrol | 183 | 158 | 59 | 5.0 (8) | 0.4 / 0.3 |
| 7 v 7 fleet-battle scenario | 243 | 9 | 148 | 10.1 (14) | 5.1 / 6.0 |
| 8 patrols v 8 patrols | 168 | 33 | 199 | 10.3 (14) | 6.1 / 5.9 |

Mirrors stay even: each side is driven from the field about half the time.
The board is smaller than the old 3D grid, so fights start sooner and end in
fewer rounds, and a retreating patrol (two moves) is caught more often than it
was: patrol duels end in a loss about 40% of the time, against 15% on the grid.
Retreat and board balance are open playtest questions.

## Playing a battle

**Fight** and a failed **Evade** open the battle screen, which fills the whole
window until the battle ends: fight it out, withdraw, auto-resolve or surrender.
The open battle is
saved with the session (`navigation.battle`) and its contact stays pending, so
navigation runs wait for it. The server plays raider turns right after each
player action, so a saved battle is always waiting on one of your ships.

- The header shows the round and each side's remaining hull and ships.
- The view is a flat hex board under a fixed tilted camera: your edge is the
  cyan column on the left, the raiders' edge the red column on the right. Ships
  float over their hex at their band's height, with a shadow and a dashed
  stalk down to the board; a HIGH or LOW tag marks bands other than Level, and
  bars show hull and shields.
- **Camera:** drag to pan; scroll or pinch to zoom (from fitting the whole board
  up to four times closer); arrow keys slide the view. **F** (or the crosshair
  button) centres on the acting ship, **R** (or the reset button) restores the
  fitted view. There is no orbit and no layer control. While a battle is open
  it owns these keys, so F and Escape do not reach the map behind it.
- **Move:** the acting ship is already selected, and green hexes are the free
  hexes it can still reach on its own band. Hovering one draws the path and
  dashed lines to every raider your weapons would reach from there; tap it to
  move. **Climb** (E) and **Dive** (Q) change band for one movement point. The
  ship card counts the moves left this turn.
- **Fire:** raiders your weapons reach have a red ring. Tap one to **aim** (a
  bold line of fire and the weapon list's hit chances, high ground included),
  then tap **Fire at …** (or Enter) to shoot. The two steps prevent misfires on
  a phone. Hovering any ship shows a card with its band, condition and, for a
  raider, its range and each weapon's hit chance.
- **End turn** (the large button beside Fire, or Space), **Withdraw** from your
  edge column, or **Auto-resolve** to let the AI fight every remaining turn on
  both sides.
- On windows narrower than 700 points (phones) the controls stack under the
  board, with Fire and End turn side by side as 44-point buttons.
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

**Saves from before the hex board.** A save (schema 7) with a battle open on the
old 3D grid still loads: the battle is rebuilt on a fresh hex board with every
ship's damage, shields, withdrawal and shot record, auto-resolved at once, and
the load reports it.

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
- **Losing the last ship ends the campaign** (see [campaign](CAMPAIGN.md)): the
  report marks it destroyed, the game freezes, and its wreck stays on record at
  1 hull point with its cargo lost so the final state still saves and loads.
- **Victory:** each destroyed raider yields 2 ore, loaded into free cargo space
  in fleet order; anything that does not fit is lost. Raiders that fled leave
  no wreck to salvage.
- **Bounty:** port authorities pay for every raider destroyed, whatever the
  outcome: scout 150.00 cr, patrol 400.00 cr, freighter 100.00 cr
  (`economy.json` → `raider_bounties`). A raider that fled pays nothing. The
  bounty is shown in the combat report and credited once, when the battle ends.
  It is a reward from outside the market economy, not a trade.
- Surviving fleets resume their course on the next run.

## Roaming raider bands

Raiders also roam as persistent bands you can see coming
(`game-core/src/navigation/raiders.rs`):

- Each system except the home system holds one band: two scouts, or a patrol
  ship leading a scout one time in three. Bands stay in their system and are
  saved with the world.
- After every run, each band hunts the nearest of your fleets in the open within
  150 million km. Fleets docked at a station or landed on a surface are safe;
  fleets in orbit or in flight are not. Without a quarry it patrols near a body,
  preferring dangerous planets (danger ratings weight the choice; stations are
  avoided).
- A band moves up to 100 million km per run (half a fleet's local speed),
  routing around planets. You can outrun one, or fly at it to pick a fight.
- Within 5 million km it strikes: a raider contact with the band's real ships
  (Fight, Auto-resolve or Evade).
- Destroyed raiders leave the band; after any fight or escape it backs off for 3
  runs. A band wiped out returns to its system 20 runs later.
- With `encounter_chance_percent` at 0 bands still roam but never strike.
- The system map shows bands as red diamonds tagged with their ships ("HUNTING"
  while chasing), with a dashed line toward their heading; the galaxy map marks
  systems that hold one; a hunted fleet's card warns "Raiders closing in".

Random raider contacts during voyages still happen as before.

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

## Hailing and robbing NPC traders

Once you have met raiders, the map roster's TRAFFIC list shows **Hail** beside
every NPC trader your flagship can reach (`game-core/src/piracy.rs`): within 20
million km, with both ships in open space or orbit. Your fleet must not be docked
or landed; a trader docked at a station, or laid up for a refit, is protected.
Hailing opens a contact that shows the trader's hold and hull. You can:

- **Let it go:** nothing happens.
- **Demand cargo:** only when the trader is outgunned. Your fleet's fighting
  strength (hull left plus shields, times weapons fitted, summed over ships) must
  reach 150% of a cautious trader's or 300% of an opportunist's. It hands over
  half its cargo, rounded up, most valuable goods first, as far as your holds have
  room. The card says whether it will give way.
- **Attack** or **Auto-resolve:** the usual tactical battle against the trader's
  ship, which fights under its own name and with its current damage. Destroy it
  and you plunder its hold (as much as fits, shown in the combat report), and it
  is refitted at its home port for 8 runs. If it withdraws it escapes with its
  cargo and keeps its damage. Traders pay no bounty. Giving up (**Break off**,
  Esc) just ends the attack: a trader seizes nothing.

Every demand and attack counts as an act of piracy (`navigation.piracies` in
the save). Nothing reads the count yet: wanted status, police response and
faction hostility arrive with factions (milestone 5). Secondary fleets never
hail; if command passes to another fleet while a hail is pending, that fleet
lets the trader go.

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
hostility rules, raider bands crossing between systems or striking NPC traders,
legal consequences of piracy, loadout changes and partial repairs. See the
[roadmap](../ROADMAP.md), milestones 4 and 5.
