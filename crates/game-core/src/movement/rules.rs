use super::{MovementError, SpacePosition};
use crate::{StarSystem, travel::Location};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    pub schema_version: u32,
    #[serde(default = "default_run_ticks")]
    pub ticks_per_navigation_run: u64,
    #[serde(default = "default_encounter_chance")]
    pub encounter_chance_percent: u32,
    pub galaxy_half_extent_ly: i64,
    pub system_half_extent_km: i64,
    pub star_exclusion_radius_km: i64,
    pub local_km_per_tick: u64,
    pub local_km_per_fuel: u64,
}

fn default_run_ticks() -> u64 {
    4
}
fn default_encounter_chance() -> u32 {
    12
}

impl Default for MovementRules {
    fn default() -> Self {
        Self::from_json(include_str!("../../../../config/movement.json"))
            .expect("embedded movement rules must validate")
    }
}

impl MovementRules {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let rules: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        rules.validate().map_err(|error| error.to_string())?;
        Ok(rules)
    }

    pub fn validate(&self) -> Result<(), MovementError> {
        if !(1..=100).contains(&self.ticks_per_navigation_run)
            || self.encounter_chance_percent > 100
            || self.schema_version != 1
            || !(1..=10_000).contains(&self.galaxy_half_extent_ly)
            || !(10..=1_000_000_000).contains(&self.system_half_extent_km)
            || !(1..self.system_half_extent_km).contains(&self.star_exclusion_radius_km)
            || !(1..=1_000_000_000).contains(&self.local_km_per_tick)
            || !(1..=1_000_000_000).contains(&self.local_km_per_fuel)
        {
            return Err(MovementError::InvalidRules);
        }
        Ok(())
    }

    pub fn validate_world(&self, systems: &[StarSystem]) -> Result<(), MovementError> {
        self.validate()?;
        for system in systems {
            let center = SpacePosition::from_galaxy(system.position)?;
            let extent = self.system_half_extent_km + 1;
            for coordinate in center.coordinates() {
                if !self.contains_space(
                    coordinate
                        .checked_add(extent)
                        .ok_or(MovementError::Coordinates)?,
                ) || !self.contains_space(
                    coordinate
                        .checked_sub(extent)
                        .ok_or(MovementError::Coordinates)?,
                ) {
                    return Err(MovementError::InvalidRules);
                }
            }
            for body in &system.bodies {
                let coordinates = [body.position.x_km, body.position.y_km, body.position.z_km];
                if coordinates
                    .iter()
                    .any(|value| value.unsigned_abs() > self.system_half_extent_km as u64)
                    || coordinates
                        .iter()
                        .map(|value| i128::from(*value).pow(2))
                        .sum::<i128>()
                        <= i128::from(self.star_exclusion_radius_km).pow(2)
                {
                    return Err(MovementError::InvalidRules);
                }
            }
        }
        // Overlapping cubes would make entry/exit ambiguous.
        for (index, system) in systems.iter().enumerate() {
            for other in &systems[..index] {
                if system.position == other.position {
                    return Err(MovementError::InvalidRules);
                }
            }
        }
        Ok(())
    }

    pub fn contains_space(&self, coordinate_km: i64) -> bool {
        coordinate_km.unsigned_abs()
            <= (self.galaxy_half_extent_ly * super::KM_PER_LIGHT_YEAR) as u64
    }

    pub fn validate_location(
        &self,
        location: &Location,
        systems: &[StarSystem],
    ) -> Result<(), MovementError> {
        match location {
            Location::DeepSpace(position) => {
                if position
                    .coordinates()
                    .iter()
                    .any(|value| !self.contains_space(*value))
                {
                    return Err(MovementError::OutsideGalaxy);
                }
            }
            Location::InSystem { system, position } => {
                if !systems.iter().any(|known| known.id == *system) {
                    return Err(MovementError::WrongSpace);
                }
                let coordinates = [position.x_km, position.y_km, position.z_km];
                if coordinates
                    .iter()
                    .any(|value| value.unsigned_abs() > self.system_half_extent_km as u64)
                {
                    return Err(MovementError::Coordinates);
                }
                if coordinates
                    .iter()
                    .map(|value| i128::from(*value).pow(2))
                    .sum::<i128>()
                    <= i128::from(self.star_exclusion_radius_km).pow(2)
                {
                    return Err(MovementError::StarCollision);
                }
            }
            Location::Orbiting { system, body } => {
                if !systems.iter().any(|known| {
                    known.id == *system
                        && known.bodies.iter().any(|known| {
                            known.id == *body && known.kind != crate::BodyKind::Station
                        })
                }) {
                    return Err(MovementError::UnknownBody);
                }
            }
            Location::Docked(system) => {
                if !systems.iter().any(|known| {
                    known.id == *system
                        && known
                            .bodies
                            .iter()
                            .any(|body| body.kind == crate::BodyKind::Station)
                }) {
                    return Err(MovementError::UnknownBody);
                }
            }
            Location::Travelling(_) | Location::InTransit(_) => {} // Legacy journeys are checked separately on load.
        }
        Ok(())
    }
}
