use super::SaveData;
use game_core::combat::gear::{FREIGHTER_CARGO_UNITS, Gear, SCOUT_RANGE_LY};
use game_core::travel::PlayerId;
use std::collections::HashSet;

impl SaveData {
    pub(super) fn validate_fleets(&self) -> Result<(), String> {
        if self.player.ships.is_empty()
            || self.player.ships.len() > 64
            || self.player.fleets.is_empty()
            || self.player.fleets.len() > 64
        {
            return Err("Invalid ship/fleet count".into());
        }
        let mut ids = HashSet::new();
        for ship in &self.player.ships {
            if ship.id.0 >= game_core::npc::NPC_ID_BASE
                || ship.id.0 == 0
                || !ids.insert(ship.id)
                || ship.owner != PlayerId(1)
                || ship.name.is_empty()
                || ship.name.len() > 128
                || ship.fuel > ship.fuel_capacity
                || ship.fuel_capacity == 0
                || ship.fuel_capacity > 1_000_000
                || ship.range_ly == 0
                || ship.range_ly > 1_000_000
                || !(1..=10_000).contains(&ship.base_cargo_capacity)
            {
                return Err("Invalid owned ship".into());
            }
            let capacity = ship.base_cargo_capacity
                + if ship.upgraded {
                    self.economy.rules.hold_upgrade_capacity
                } else {
                    0
                }
                + if ship.gear.contains(&Gear::FreighterRacks) {
                    FREIGHTER_CARGO_UNITS
                } else {
                    0
                };
            let Some(spec) = self.economy.rules.ship_catalog.spec(ship.kind) else {
                return Err("Invalid owned ship model".into());
            };
            if ship.cargo_capacity != capacity
                || ship.cargo_used() > u64::from(capacity)
                || (ship.gear.contains(&Gear::ScoutDrive)
                    && ship.range_ly != spec.range_ly + SCOUT_RANGE_LY)
                || ship
                    .gear
                    .iter()
                    .any(|gear| !gear.fits(ship.kind, spec.combat.size))
            {
                return Err("Invalid ship cargo".into());
            }
        }
        let mut fleets = HashSet::new();
        let mut members = HashSet::new();
        for fleet in &self.player.fleets {
            if fleet.id.0 >= game_core::npc::NPC_ID_BASE
                || fleet.id.0 == 0
                || !fleets.insert(fleet.id)
                || fleet.owner != PlayerId(1)
                || fleet.name.is_empty()
                || fleet.name.len() > 128
                || fleet.ships.is_empty()
                || fleet.captain.experience > game_core::species::FleetCaptain::MAX_EXPERIENCE
            {
                return Err("Invalid fleet identity".into());
            }
            let mut location = None;
            for id in &fleet.ships {
                if !members.insert(*id) {
                    return Err("Ship belongs to multiple fleets".into());
                }
                let ship = self.player.ship(*id).map_err(|_| "Unknown fleet ship")?;
                if location.is_some_and(|previous| previous != &ship.location) {
                    return Err("Fleet members are not together".into());
                }
                location = Some(&ship.location);
            }
        }
        if members != ids
            || self
                .player
                .ships
                .iter()
                .any(|ship| ship.id.0 >= self.player.next_ship_id)
            || self
                .player
                .fleets
                .iter()
                .any(|fleet| fleet.id.0 >= self.player.next_fleet_id)
        {
            return Err("Invalid fleet membership or identity counters".into());
        }
        Ok(())
    }
}
