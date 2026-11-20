# Phone-first rework

Status: **agreed direction, not implemented.** This plan reworks the interface,
the map and battles around a phone-sized screen, and gives the game a goal.
Where it conflicts with [map design](MAP_DESIGN.md) (3D map on a 2D screen) or
[combat](COMBAT.md) (3D battle grid), this document wins once each phase lands.

## Decisions

| Topic | Decision |
| --- | --- |
| Target screen | Phone portrait first (reference 390 × 844 pt); desktop gets the same layout with more room, not a separate design. |
| Space | **Flat play.** The galaxy and every system are planes. 3D is for looks (tilted camera, 3D ship models), never for decisions. |
| Battles | **Hex board with three altitude bands** (Low, Level, High), replacing the free 3D grid. |
| Story | **Several roads to victory** (Trader, Soldier, Diplomat/Spy, Pirate) in one frontier setting. The debt opening hook is **paused**. |
| Feature surface | Systems unlock as play progresses instead of all being visible from the first minute. |

Why flat: in other space strategy games, 3D works when it is for show and does
not decide anything. Stellaris, Endless Space and Galactic Civilizations play on
a plane under a tilted 3D camera. True 3D maps (Master of Orion 3, Ascendancy,
Elite's galaxy map) are remembered as hard to read. On a phone (one finger, no
hover, no right-drag) depth is the first thing to hurt.

## Interface principles

1. **The map is the board; everything else is on demand.** Only a thin status
   strip and one action bar stay on screen.
2. **One decision at a time.** A single bottom sheet holds what was tapped. Sheets
   replace each other; they never stack.
3. **Show only what can be acted on.** Other fleets' courses, NPC labels and orbit
   rings fade unless they concern the selection.
4. **Every essential action is one tap,** with targets at least 44 pt. Never require
   hover, a keyboard, right-drag or a precise drag.
5. **One colour language:** green for yours, amber for neutral, red for hostile,
   cyan for information. Always paired with an icon or text.

## Screen layout

```
┌──────────────────────────────┐
│ 12,480 cr · Run 37 · ⚠ 2     │  status strip: credits, run, alerts badge
├──────────────────────────────┤
│                              │
│            MAP               │  galaxy or system, full screen
│                              │
├──────────────────────────────┤
│  bottom sheet (when open)    │  card for whatever was tapped
├──────────────────────────────┤
│  Fleets │  ADVANCE  │ Menu   │  action bar
└──────────────────────────────┘
```

- **Status strip:** credits, run number and an alert badge (contacts, idle fleets,
  holds full). Tapping the badge walks the attention queue.
- **Action bar:** Fleets (list sheet), a large Advance (one run per tap; hold or
  toggle for repeated runs), Menu (save, load, settings, log, guide).
- **Bottom sheet:** peeks at about a third of the screen and expands on swipe.
  - Tap a body: name, distance, market summary, Land / Plot course.
  - Tap a fleet stack: its fleets; tap one for orders.
  - A contact: Fight / Auto-resolve / Evade.
- **Attention queue (Civ-style):** when Advance stops, the sheet opens on what
  needs the player ("Fleet 2 reached Meridian. Sell 20 ore?"), with a Next button
  for the following item.
- **Full-screen sheets:** market, shipyard, fleet management, combat report,
  battle. Each is a plain list with steppers and one primary button, closed
  with a Back control at the top.
- **Desktop:** the same structure. The sheet becomes a side panel when the window
  is wider than tall, and the map gains the extra space.

## Map: declutter rules

- **Fleet stacks.** Fleets at the same place merge into one badge ("3") on the body
  or point. Tap it to list them. No tether lines anywhere.
- **Ships on bodies** sit as small icons on a ring around the body, like moons.
  Docked, orbiting and landed are told apart by the icon, not by extra lines.
- **Courses:** only the selected fleet's course is drawn in full. Other fleets
  under way show a short direction tick.
- **Labels:** body names appear when zoomed in or selected. System names show in
  the galaxy view; body names do not.
- **NPC traffic:** small amber dots without labels; tap one for its card.
- **Raider bands:** a single red threat icon. A heading line appears only while it
  hunts one of your fleets.
- **Orbit rings:** drawn faintly for bodies near the selection; hidden otherwise.

## Flat play

- **Galaxy:** systems lie on one plane (the vertical coordinate is 0). The view is
  top-down with a slight fixed tilt. Pan and pinch-zoom only: no orbit camera,
  no axis presets, no candidate popover for depth overlaps.
- **Systems:** bodies lie on the same plane, already on circular orbits. Collision
  volumes become circles; landing, mining and routing work unchanged.
- **Rules:** movement keeps its integer 3D maths with the vertical axis always 0,
  so fuel, range and durations stay exact.
- **Saves:** new games are generated flat. Saves from before flattening are
  rejected with a clear message (prototype rule: backward compatibility is not
  required).

## Hex battles with altitude bands

**Board.** A hexagonal board in axial coordinates. Radius 4 (61 hexes) for up to
three ships, one more ring per four more ships, at most radius 6 (127 hexes).
Sides deploy one column in from opposite edges and withdraw from their own edge.

**Altitude bands.** Each ship is in one of three bands: Low, Level or High. One
ship per hex, whatever its band, so every ship has one tap target.

**Turns.** Unchanged: initiative by engine, one move and one volley per turn,
withdraw from your own edge, End turn, Auto-resolve, Surrender.

- **Move:** `1 + engine / 3` points. Each hex step costs 1; **Climb** or **Dive**
  one band costs 1.
- **Range:** hex distance plus the band difference. Weapon ranges keep their
  current values (3 to 6).
- **High ground:** firing at a lower band gets +10% to hit; firing at a higher
  band gets −10%. Other hit rules (evasion, range penalty, 5% floor) stay.
- **Kept as is:** morale, retreats, ceasefire, bounties and the AI. The AI gains
  one preference: take the higher band when it costs nothing.

**View.** A flat hex board under a tilted camera with the 3D ship models; a ship's
band shows as height above its hex with a shadow on the board. No layer slider
and no orbit camera. Pinch-zoom and pan only when the board does not fit.

**Controls.**
1. The acting ship is already selected, and its reachable hexes are lit.
2. Tap a lit hex to move there; Climb and Dive buttons change band.
3. Tap a ringed enemy to see each weapon's hit chance, then tap **Fire** to
   confirm. The two steps prevent misfires on a phone.
4. End turn is the large button at the bottom right.

**Saves.** A battle saved in the old 3D format is auto-resolved on load, with a
note in the log.

## Story: several roads to victory

**Setting.** The frontier: Sol, Meridian, Far Reach and the systems beyond. The
great powers are pulling back, and raider bands fill the gap. A **Raider Haven**,
a fortified base in one far system, is where bands come from. Destroying or
seizing it changes the frontier.

**Opening.** For now the game starts as today: one freighter at Sol, with the
first-delivery guide as onboarding. The debt hook is paused, not dropped.

**Victory paths.** A Victory panel in the Menu shows progress on every road; the
first road completed wins. Campaigns are open-ended by default; an optional
campaign-length setting (for example 400 runs) ends the game on score for shorter
sessions and for multiplayer.

| Road | Win condition | Builds on | Needs |
| --- | --- | --- | --- |
| Trader | Net worth of 1,000,000 cr, **or** the trade charters of three ports (bought from port authorities) | Markets, routes, fleets | Charters (new) |
| Soldier | Destroy the Raider Haven (a large defended battle) | Battles, bands, bounties | Haven (new) |
| Diplomat / Spy | Highest standing with a great power, then broker the frontier treaty | Factions, reputation, missions | Milestones 5 and 6 |
| Pirate | Take over the Raider Haven and become its master | Piracy, wanted status | Milestones 5 and 6 |

**Score** (for run limits and multiplayer): net worth, plus charters, haven
damage and faction standing, each converted to points in a published formula.

**Losing.** If your last ship is destroyed, the game is over. Fleeing is always
possible (Evade, withdrawing from your edge, surrender), so this replaces today's
rule that spares the last ship crippled.

**Raider Haven.** Placed per seed, always in a system far from the starting zone.
Destroying it is a big set-piece battle: the Haven's own defences plus several
bands, near the board's largest size.

**Trade charters** (proposed). A charter is a port authority's licence, bought once
at a high price:
- **Shipping lanes:** two chartered ports are joined by a patrolled lane. Your
  trade routes along it are never struck by raiders, and the lane is drawn as a
  bold line on the map.
- **Port share:** a small cut (for example 2%) of every trade at that port, NPC
  traders included.

Three charters is the Trader victory, so that road builds a visible network.

**Tone.** Corporate with adventure, not gritty: company memos, contracts and
rival trading houses, with discovery and close calls. Event text, guide cards,
names and reports are written in that voice.

## Unlock order

Rules exist from the start; controls appear when they become relevant, each with
a one-line guide card.

1. **Captain:** one freighter; market, refuel, plot a course, Advance.
2. **First 2,000 cr of profit:** shipyard, more ships, fleets.
3. **Two fleets, or ten deliveries:** trade routes.
4. **First raider contact:** battles (the first one offers Auto-resolve first,
   then the battle screen with a short guide).
5. **First orbit of a moon or asteroid:** landing and mining.
6. **First bounty, or 50,000 cr net worth:** the Victory panel and charters.

## What stays, changes or goes

| Feature | Fate |
| --- | --- |
| Navigation runs, Advance, Wait orders | Stay; Advance moves to the action bar |
| Markets, refuel, repairs, shipyard | Stay; become full-screen list sheets |
| Trade routes, landing, mining | Stay; unlocked later |
| Fleet roster card, Nav/Trade/Save cards, toolbar | Replaced by the action bar, sheets and Menu |
| Galaxy orbit camera, view presets, axis gizmo | Removed (flat play) |
| 3D battle grid, layer slider, orbit camera | Replaced by the hex board with bands |
| Battle models, animations, morale, bounties | Stay |
| Orbits and collision volumes | Stay, on the plane |
| Raider bands, NPC raids | Stay; bands spawn from the Raider Haven |
| Debt hook | Paused |

## Implementation phases

Each phase ships on its own, keeps `./tools/check.sh` green and adds captures at
390 × 844 and 1280 × 800.

1. **Declutter on the current desktop UI** (done): fleet stacks that cycle on
   tap, no tethers, heading ticks, labels by zoom or selection, quiet traffic
   dots, band lines only while hunting, the orbit ring only when selected. The
   list sheet for a stack arrives with phase 2.
2. **Phone shell:** status strip, action bar, bottom sheet, attention queue,
   full-screen sheets, responsive layout. Removes the floating cards.
3. **Flat play:** flat generation and saves, top-down map camera, controls removed.
4. **Hex battles:** core rules first (hex coordinates, bands, high ground, AI and
   balance tests carried over), then the board view and touch controls.
5. **Victory framework:** Victory panel, score, campaign length, Raider Haven
   (bands spawn from it; destroying it wins), trade charters.
6. **Unlocks:** progressive disclosure and guide cards.

Diplomat/Spy and Pirate roads follow factions and missions (milestones 5 and 6).
Mobile packaging stays deferred; phases 1 to 6 only make the layout and controls
phone-ready.

## Decided questions

| Question | Answer |
| --- | --- |
| Last ship destroyed | Game over (fleeing is always possible) |
| Run limit | None by default; optional setting |
| Raider Haven | Random per seed, far from the starting zone; a big final battle |
| Charters | Shipping lanes between chartered ports plus a port share (proposed) |
| Tone | Corporate with adventure, not gritty |
