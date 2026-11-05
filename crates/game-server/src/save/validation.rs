use super::SaveData;
use game_core::travel::{Location, PlayerId, ShipId};
use std::collections::HashSet;

impl SaveData {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported save schema version".into());
        }
        if self.systems.is_empty() || self.systems.len() > 1024 {
            return Err("Invalid galaxy size".into());
        }
        let mut systems = HashSet::new();
        for system in &self.systems {
            if !systems.insert(system.id)
                || system.id.0 == 0
                || system.name.is_empty()
                || system.name.len() > 128
            {
                return Err("Invalid or duplicate system identity".into());
            }
            if [system.position.x, system.position.y, system.position.z]
                .iter()
                .any(|p| p.unsigned_abs() > 1_000_000)
            {
                return Err("Unsupported galaxy coordinates".into());
            }
            if system.bodies.len() > 1024 {
                return Err("Too many local destinations".into());
            }
            let mut bodies = HashSet::new();
            for body in &system.bodies {
                if body.id.0 == 0
                    || !bodies.insert(body.id)
                    || body.name.is_empty()
                    || body.name.len() > 128
                    || [body.position.x_km, body.position.y_km, body.position.z_km]
                        .iter()
                        .any(|p| p.unsigned_abs() > 1_000_000_000_000)
                {
                    return Err("Invalid local destination".into());
                }
            }
        }
        let ship = &self.player.ship;
        if ship.owner != PlayerId(1)
            || ship.id != ShipId(1)
            || ship.fuel > ship.fuel_capacity
            || ship.fuel_capacity == 0
            || ship.fuel_capacity > 1_000_000
            || ship.range_ly == 0
            || ship.range_ly > 1_000_000
        {
            return Err("Invalid player ship, ownership or fuel".into());
        }
        match &ship.location {
            Location::Docked(system) if systems.contains(system) => {}
            Location::Travelling(journey)
                if systems.contains(&journey.origin)
                    && systems.contains(&journey.destination)
                    && journey.origin != journey.destination
                    && journey.arrival_tick > self.player.tick
                    && journey.arrival_tick.saturating_sub(self.player.tick)
                        <= u64::from(ship.range_ly).max(1) => {}
            _ => return Err("Invalid ship location or journey time".into()),
        }
        if let Location::Travelling(journey) = &ship.location {
            let origin = self
                .systems
                .iter()
                .find(|s| s.id == journey.origin)
                .ok_or("Missing journey origin")?;
            let target = self
                .systems
                .iter()
                .find(|s| s.id == journey.destination)
                .ok_or("Missing journey destination")?;
            let mut preview = self.player.clone();
            preview.ship.location = Location::Docked(journey.origin);
            preview.ship.fuel = preview.ship.fuel_capacity;
            let quote = preview
                .quote(journey.destination, origin.position, target.position)
                .map_err(|error| format!("Invalid saved route: {error}"))?;
            if journey.arrival_tick - self.player.tick > quote.duration_ticks {
                return Err("Invalid remaining travel duration".into());
            }
        }
        self.economy.rules.validate()?;
        let capacity = self.economy.rules.cargo_capacity
            + if self.economy.upgraded {
                self.economy.rules.hold_upgrade_capacity
            } else {
                0
            };
        if self.economy.cargo_capacity != capacity
            || self.economy.cargo_used() > u64::from(capacity)
            || self.economy.credits.0 > 1_000_000_000_000_000
        {
            return Err("Invalid player credits or cargo".into());
        }
        if self.economy.markets.len() != systems.len() {
            return Err("Missing markets".into());
        }
        let mut markets = HashSet::new();
        for market in &self.economy.markets {
            if !systems.contains(&market.system) || !markets.insert(market.system) {
                return Err("Invalid market identity".into());
            }
            for good in &market.goods {
                if !(1..=1000).contains(&good.target)
                    || good.stock > good.capacity()
                    || good.base_price.0 == 0
                    || good.base_price.0 > 1_000_000_000
                    || good.production > 1000
                    || good.consumption > 1000
                {
                    return Err("Invalid market balances or rates".into());
                }
            }
        }
        Ok(())
    }
}
