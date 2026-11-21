# Player specializations and mission routes

The current first-delivery guide is **trader onboarding**, not a generic questline.
No contract/quest engine, espionage or warfare system is implemented yet. Piracy
has a first foundation: hailing NPC traders, demanding cargo and attacking them
(see [combat](COMBAT.md)); legal status and heat are still missing.
Navigation, travel, refuelling, saving and ship management are shared foundations.

## Working progression direction

Treat these as specializations acquired through missions, equipment and reputation,
rather than permanently locked character classes. Players should be able to mix
routes; exact unlocks and starting choices still need design/playtesting.

| Route | Primary activities | Mission-chain examples | Required consequences/systems |
| --- | --- | --- | --- |
| Trader | Commerce, hauling, logistics, supply contracts | First delivery → dependable supplier → scarce-goods logistics | Stock/pricing, cargo capacity, deposits, deadlines, delivery reputation |
| Spy | Espionage, intelligence gathering, covert delivery, infiltration | Discreet courier → reconnaissance → intelligence operation | Intel objectives, cover/detection, access restrictions, secrecy, faction trust |
| Soldier | Military service, patrol, escort, defense and combat | Patrol assignment → convoy escort → contested-system deployment | Combat resolution, equipment/damage/repair, military reputation, rules of engagement |
| Pirate | Robbery, boarding demands, combat, illicit sales and police evasion | Intercept cargo → escape pursuit → establish illicit contacts | Legal status, wanted/heat, inspections/pursuit, stolen cargo, fencing and faction hostility |

These chains are design examples, not existing missions or promises of a separate
real-time tactical combat game. Initial encounters should use understandable choices
and shared ship/equipment rules before adding specialized interfaces.

## Mission-system boundary

Shared onboarding should teach controls, travel, fuel, saving and where ship status
is shown. After that, route-specific introductions should establish different goals
and failure consequences. Completing a cargo delivery must not become a prerequisite
for every spy/soldier/pirate story unless that mission specifically uses it as cover.

The future JSON mission definitions should identify a stable mission ID, route,
prerequisites, objectives, tick deadlines, rewards, failure consequences and
reputation/legal effects. This is a proposed design; it is not a supported JSON
schema yet. Live objective progress, deadlines and outcomes belong to the authority,
are calculated on simulation ticks, and must be persisted in player/world saves.
Clients submit intent; they cannot grant rewards or complete objectives directly.

Implement reusable contracts/objectives first, then a small route-specific mission
chain, rather than four independent quest engines. Combat encounters, faction access,
intel/cover and legal heat should be shared systems usable across multiple routes.
Multiplayer will resolve mission actions within shared turns; single-player will
advance those same tick-based rules automatically.

## Implementation order

1. Finish desktop acceptance and observe the current trading loop's pacing.
2. Add authoritative contract/objective lifecycle with deadlines, outcomes and saves.
3. Introduce shared encounter/combat, faction and legal-state foundations.
4. Deliver short trader, spy, soldier and pirate introductions as those systems work.
5. Playtest the distinct decisions, route mixing and consequences before expansion.
