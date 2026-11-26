# Species and fleet captains

New games choose a starting captain's species: **Human**, **Miri**, **Auralith**
or **Kethari**. The entire name and description form one selectable option in
the portrait menu. The species portraits are 512 px game textures derived from
the original images in `assets/source/species/`. The selected portrait is larger than the
menu entries. The choice sets the player character and starting fleet captain's
species; the ships themselves have no species. Political allegiance is selected
separately. Every inhabited port has residents of all four species, and NPC
trader crews cycle through them regardless of the faction they serve. None of
the current species is xenophobic. Each inhabited port starts with a
majority species chosen from its market identity and small communities of the
other three. Every economy tick moves one percentage point from the largest
community to the smallest until the four shares differ by at most one point.
Faction ownership does not restrict that migration.

Each port also stores a total resident count in people. New station counts are
120,000–999,999 and inhabited planet counts are 2,000,000–19,999,999, chosen
deterministically from the port ID. The species percentages always sum to 100
and describe shares of this count. The count stays fixed while the shares mix;
older saves without a count load with 1,000,000 residents per port.

| Captain species | Fleet effect in battle | Escape chance |
| --- | --- | --- |
| Human | Balanced baseline | unchanged |
| Miri | +1 engine, −2 shields to each ship | +10 percentage points |
| Auralith | +4 shields, −1 engine to each ship | −10 percentage points |
| Kethari | +2 armor, −1 engine to each ship | −5 percentage points |

The engine rating stays within 1–10 and escape chance within 10–90%. Fleet
captains earn one experience point per navigation run while their fleet travels
and five for a victory. Experience caps at 100; each 25 points adds one armor
to every ship, and each 20 points adds one percentage point to escape chance.
Splitting a fleet appoints a new novice captain of the player's species;
merging retains the target fleet's captain. At a port, **Menu → Fleets → Appoint
captain** can select any species for a fleet; the replacement starts at zero
experience. Captain traits are applied when a battle begins and are visible in
**Menu → Fleets**. Saves retain species and
experience, and old saves default to Human novices.

The Miri are small green explorers with a strong sense of space. Auralith are
luminous beings who can join their minds in a shared choir. Kethari are
insectoid colony-born navigators who favor coordinated formations. These are
original species inspired by broad science fiction themes.

Old three-species saves keep their Human, Miri and Auralith population shares;
Kethari begin at 0% in those ports and enter through subsequent migration.
