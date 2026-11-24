# Combat

Status: **turn-based battles you command on a single-layer hex board with
asteroids and debris.** Hulls and weapons are data-driven. Raider contacts resolve
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
  ranges, deployment), `tactical/terrain.rs` (asteroids, debris, paths, lines
  of fire), `tactical/actions.rs` and `tactical/ai.rs`; shared damage
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

| Weapon | Damage × shots | Accuracy | vs shields | Armor pierce | Range | Effect | Role |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Photon Singularity Beam | 14 × 1 | 90% | 120% | 3 | 5 | Piercing | accurate heavy lance |
| Quantum Entanglement Torpedo | 30 × 1 | 65% | 50% | 8 | 6 | Guided | armor breaker that reaches behind rocks |
| Neutron Beam | 7 × 2 | 85% | 100% | 2 | 4 | Steady | reliable civilian weapon at any range |
| Graviton Pulse | 5 × 3 | 92% | 150% | 0 | 3 | Ripple | shield stripper that punishes bunching |
| Magnetic Resonance Disruptor | 4 × 2 | 90% | 250% | 0 | 3 | Disrupt | shield breaker that pins ships down |

**Weapon effects** (the `effect` field in `config/combat.json`, applied in
`tactical/actions.rs`):

- **Piercing:** ignores debris cover.
- **Guided:** needs no line of fire: it steers around asteroids. Point defense
  counts double against it (−30 points instead of −15).
- **Steady:** loses no accuracy to range (hit chance as if adjacent).
- **Ripple:** each hit also deals half its damage (at least 1, before shields
  and armor as usual) to every active enemy on a hex next to the target.
  Ripples cannot miss and do not spread further. Keep your ships apart against
  scouts.
- **Disrupt:** a hit jams the target's engines: one movement point fewer on its
  next turn (it wears off when that turn ends). The ship card and hover card
  say "ENGINES JAMMED".

The battle's weapon list tags each weapon with its effect and shows when an
asteroid blocks a weapon that is in range. The AI weighs line of fire only for
weapons that need it. These numbers are first-pass balance.

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

**One layer.** One ship per hex, so every ship is one tap target. **Range** is
hex steps. (Schema-8 saves had Low/Level/High altitude bands; schema 9 dropped
them for terrain.)

**Terrain.** Each battle's terrain comes from its seed: `radius / 2` asteroid
hexes and as many debris hexes on each half of the board (2 + 2 on a duel board,
3 + 3 at radius 6), mirrored through the centre so both sides face the same
field. Edge and deployment columns stay clear.

- **Asteroids** are impassable: ships move around them (ships themselves can be
  passed), and they block the **line of fire**, traced hex by hex between the
  two ships' centres.
- **Debris** is open ground that gives **cover**: shots at a ship standing in
  it have 20% less chance to hit.

**Turns (HoMM-style initiative).** Each round every active ship acts once, by
engine rating (high first); ties break by a seeded roll drawn once per battle.
On its turn a ship may:

1. **Move** with `1 + engine / 3` movement points (freighter 2, patrol 2, scout
   3): each hex step costs 1, along the shortest path around asteroids. Points
   may be spent in several moves to free hexes on the board.
2. **Fire** once at an enemy in a clear line of fire: every fitted weapon whose
   `range` reaches it fires its `shots`. Hit chance = `accuracy − 3 × target
   engine − 4 × (range − 1)`, **−20%** when the target is in debris, at least 5%
   and at most 100%. A target destroyed
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
active enemy with the least hull + shields and head for a hex with a clear line
of fire from which even their shortest-ranged weapon reaches it, preferring one
in debris; until one is in reach they close in along the shortest path around
asteroids. They then fire on the weakest enemy in sight. A ship with 30%
of its hull or less retreats, and so does every ship of a broken side (a rout):
it fires a parting shot at the weakest enemy in reach, moves toward its own edge
(away from enemies on ties) and withdraws as soon as it is there. Ships you
command yourself never retreat on their own.

With Auto-resolve over 400 seeds on the single-layer board with terrain:

| Matchup | You win | You lose | You escape | Avg rounds (max) | Ships lost / killed |
| --- | --- | --- | --- | --- | --- |
| Freighter v scout | 400 | 0 | 0 | 7.3 (19) | 0 / 0.8 |
| Scout v scout | 204 | 50 | 146 | 5.5 (10) | 0.1 / 0.2 |
| Patrol v patrol | 198 | 153 | 49 | 5.8 (10) | 0.4 / 0.4 |
| 7 v 7 fleet-battle scenario | 220 | 31 | 149 | 10.3 (16) | 5.2 / 6.2 |
| 8 patrols v 8 patrols | 186 | 67 | 147 | 10.8 (16) | 6.0 / 6.0 |

Mirrors stay even: each side is driven from the field about half the time.
Weapon effects made patrol fights deadlier: torpedoes reach ships sheltering
behind asteroids and disruptors jam a retreating ship, so patrol duels end in a
loss about 38% of the time (24% before effects) and far fewer patrols escape.
The steady neutron beam wins every freighter–scout duel. Terrain density, cover
strength and effect strengths are open playtest questions.

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
  hover just over their hex with a shadow beneath, and bars show hull and
  shields. Asteroids rise from the board as rocks (drawn in depth order with
  the ships); debris hexes are tinted and strewn with wreckage.
- **Camera:** drag to pan; scroll or pinch to zoom (from fitting the whole board
  up to four times closer); arrow keys slide the view. **F** (or the crosshair
  button) centres on the acting ship, **R** (or the reset button) restores the
  fitted view. There is no orbit and no layer control. While a battle is open
  it owns these keys, so F and Escape do not reach the map behind it.
- **Move:** the acting ship is already selected, and green hexes are the free
  hexes it can still reach around asteroids. Hovering one draws the move and
  dashed lines to every raider your weapons would reach, in a clear line of
  fire, from there; tap it to move. The ship card counts the moves left this
  turn and notes when the ship is in cover.
- **Fire:** raiders your weapons reach in a clear line of fire have a red ring.
  Tap one to **aim** (a bold line of fire and the weapon list's hit chances,
  cover included),
  then tap **Fire at …** (or Enter) to shoot. The two steps prevent misfires on
  a phone. Hovering any ship shows a card with its condition, whether it is in
  cover and, for a raider, its range and each weapon's hit chance.
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
ships and two scouts against two raider patrols and five scouts. Pick other
sizes with `--scenario fleet-battle:NvM`, from 1 to 8 ships a side (`8v8`
fills the 16-ship board); extra player hulls cycle patrol, scout, freighter
after the starting freighter, and raiders are the first M of patrol, scout,
scout, patrol, scout, scout, scout, patrol. Saves accept up
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

- Each system except the home system starts with one band: two scouts, or a
  patrol ship leading a scout one time in three. Bands are saved with the world
  and can cross between systems (see below).
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
- Bands prey on NPC traders too (`game-core/src/npc/band_raids.rs`): a band with
  no nearer fleet of yours in range stalks the nearest laden trader out in the
  open (not docked at a station or being refitted) and strikes once it reaches
  where the trader stood, or the trader comes within 5 million km. A trader it
  outguns as you would need to for a demand (150% of a cautious trader's
  strength, 300% of an opportunist's) hands over half its cargo without a fight;
  otherwise they fight it out (auto-resolved, as below). Afterwards the band
  lies low for 12 runs. Stalking bands show a dashed line and "STALKING TRADER"
  on the system map.
- With `encounter_chance_percent` at 0 bands still roam but never strike.
- Both maps show bands as red-ringed ship markers (the heaviest hull, with a
  ship count); the system map tags them with their ships ("HUNTING" while
  chasing) and a dashed line toward their heading, the galaxy map puts them
  beside the system that holds them (or on the red line while crossing); you
  can **Pursue** a band in your flagship's system ([pursuit](NAVIGATION_RUNS.md#pursuit)); a hunted fleet's card warns "Raiders closing in".

### Bands crossing systems

Bands do not stay put (`game-core/src/navigation/band_roaming.rs`):

- **Pursuit:** a band hunting your fleet when it leaves the system follows it
  to the next system on its course, if that lies within 40 ly. The fleet's card
  then reads "Raiders following you". A fleet that pulled out of detection range
  before leaving, or that leaves without a course into another system, loses it.
- **Roaming:** a band with nothing to chase and not backing off moves on one run
  in 40, to a random system within 30 ly.
- Bands never enter the home system (fleeing home shakes off a pursuit), and a
  system holds at most two bands, counting bands on their way there. A wiped-out
  band's system that is full when its replacement is due tries again 20 runs later.
- A crossing takes one run per 2 ly (half a fleet's deep-space speed), at least
  one. Meanwhile the band neither hunts nor strikes; it arrives on the
  destination's cube face toward the system it left and acts from the next run.

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
news in amber). On top of these random raids, roaming bands strike the traders
they stalk (see above). Over 400 runs on four seeds the two starting traders met
15–32 random raids and 22–42 band strikes, and none was destroyed: bands that
could win easily take cargo instead, and scout pairs rarely beat a freighter.

## Anti-piracy gear and escorts

Gear and escorts are bought in the Market window (`game-core/src/combat/gear.rs`,
prices in the economy rules' `gear_prices`, at today's price level). The payment
goes to that market's treasury.

| Item | Applies to | Effect | Price |
| --- | --- | --- | --- |
| Afterburners | one ship | +1 move each battle turn; +15 points on the Evade chance when every ship in the fleet has them (at most 90%) | 500 cr |
| Point defense | one ship | shots at the ship have 15 points less chance to hit (stacks with debris cover) | 700 cr |
| Escort | a fleet | a patrol ship joins every battle the fleet fights, on its side, for 10 runs; you command it like your own ships. If it is destroyed the contract ends. Hiring again renews it to 10 runs | 250 cr |

Each item is fitted once per ship and stays fitted. A battle never grows beyond
16 ships, so an escort stays out of a battle that is already full.

**NPC traders arm themselves** (`game-core/src/npc/outfit.rs`). At a market,
after repairs and fuel, a trader whose wallet holds more than twice its starting
credits plus the price fits afterburners, then point defense (one item per stop).
With more than three times its starting credits it hires an escort whenever it
has none. The traffic list and the hail card show a trader's defences. Over 400
runs on four seeds, traders fitted their first gear between runs 18 and 99 and
flew with an escort 40–60% of the time afterwards. A trader's escort fights
beside it against raiders and pirates and counts toward its strength when
someone demands its cargo.

## Hailing and robbing NPC traders

Once you have met raiders, the map roster's TRAFFIC list shows **Hail** beside
every NPC trader your flagship can reach (`game-core/src/piracy.rs`): within 20
million km, with both ships in open space or orbit. Your fleet must not be docked
or landed; a trader laid up for a refit is protected. A trader **docked** at a
station can be hailed only by a flagship docked at the same port (**Hail ·
trade**), and only to trade: the station's guns rule out demands and attacks,
so the hail opens its offer of its hold below list price, as when a trader
comes alongside ([navigation runs](NAVIGATION_RUNS.md#encounters)). Traders at a
port never approach you on their own. Otherwise hailing opens a contact that
shows the trader's hold and hull. You can:

- **Let it go:** nothing happens.
- **Demand cargo:** only when the trader is outgunned. Your fleet's fighting
  strength (hull left plus shields, times weapons fitted, summed over ships) must
  reach 150% of a cautious trader's or 300% of an opportunist's. It hands over
  half its cargo, rounded up, most valuable goods first, as far as your holds have
  room. The card says whether it will give way.
- **Attack** or **Auto-resolve:** the usual tactical battle against the trader's
  ship, which fights under its own name with its current damage and gear,
  joined by its escort if it has one (destroying the escort ends its contract). Destroy it
  and you plunder its hold (as much as fits, shown in the combat report), and it
  is refitted at its home port for 8 runs. If it withdraws it escapes with its
  cargo and keeps its damage. Traders pay no bounty. Giving up (**Break off**,
  Esc) just ends the attack: a trader seizes nothing.

Every demand and attack counts as an act of piracy (`navigation.piracies` in
the save). In a faction's space each act costs standing with it
([factions](FACTIONS.md)) and makes you wanted there: the faction's police
patrols then stop your fleets, and fighting them is a battle like a raider
fight with no bounty ([the law](FACTIONS.md#the-law-wanted-status-and-police)).
Destroyed raiders earn standing there. Secondary fleets never
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

A GPU renderer with a depth buffer, per-effect visuals (beams are drawn alike),
morale effects on accuracy, faction war fleets, weapon loadout changes and
partial repairs. See the
[roadmap](../ROADMAP.md), milestones 4 and 5.
