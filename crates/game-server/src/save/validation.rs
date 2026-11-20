use super::SaveData;
use game_core::travel::Location;
use std::collections::HashSet;

impl SaveData {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema_version != 6 {
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
        self.player
            .movement_rules
            .validate_world(&self.systems)
            .map_err(|error| error.to_string())?;
        self.economy.rules.validate()?;
        self.validate_fleets()?;
        self.player.validate_navigation(&self.systems)?;
        self.player.validate_battle(&self.economy.rules)?;
        for ship in &self.player.ships {
            let hull = self
                .economy
                .rules
                .ship_catalog
                .spec(ship.kind)
                .map_or(0, |spec| spec.combat.hull);
            if ship.hull_damage >= hull {
                return Err(format!("{} has impossible hull damage", ship.name));
            }
            self.player
                .movement_rules
                .validate_location(&ship.location, &self.systems)
                .map_err(|error| error.to_string())?;
            match &ship.location {
                Location::Docked(system) if systems.contains(system) => {}
                Location::InTransit(_)
                | Location::Orbiting { .. }
                | Location::InSystem { .. }
                | Location::DeepSpace(_) => {}
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
                let selected = preview
                    .ship_mut(ship.id)
                    .map_err(|_| "Unknown saved ship")?;
                selected.location = Location::Docked(journey.origin);
                selected.fuel = selected.fuel_capacity;
                let quote = preview
                    .quote_for(
                        ship.id,
                        journey.destination,
                        origin.position,
                        target.position,
                    )
                    .map_err(|error| format!("Invalid saved route: {error}"))?;
                if journey.arrival_tick - self.player.tick > quote.duration_ticks {
                    return Err("Invalid remaining travel duration".into());
                }
            }
        }
        if self.economy.credits.0 > 1_000_000_000_000_000 {
            return Err("Invalid player credits".into());
        }
        self.validate_markets()?;
        self.economy
            .validate_traders(&self.systems, self.player.navigation.run)?;
        Ok(())
    }
}
