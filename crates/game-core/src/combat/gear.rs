//! Anti-piracy gear and hired escorts, for player ships and NPC traders alike.
//!
//! - **Afterburners** (per ship): +[`AFTERBURNER_MOVES`] movement point each
//!   battle turn, and +[`AFTERBURNER_EVADE_PERCENT`] points on the chance to
//!   evade raiders when every ship of the fleet has them.
//! - **Point defense** (per ship): shots at the ship have
//!   [`POINT_DEFENSE_PERCENT`] points less chance to hit.
//! - **Escort** (per fleet, or per NPC trader): a patrol ship hired at a market
//!   for [`ESCORT_RUNS`] navigation runs. It joins every battle the fleet
//!   fights, on the fleet's side; destroyed, the contract ends. Hiring again
//!   renews the contract to its full length.
//!
//! Gear is fitted once per ship at a market and stays fitted; prices are in
//! [`GearPrices`] at a new game's price level.
use crate::{economy::Credits, fleet::ShipKind};
use serde::{Deserialize, Serialize};

pub const AFTERBURNER_MOVES: u32 = 1;
pub const AFTERBURNER_EVADE_PERCENT: u32 = 15;
pub const POINT_DEFENSE_PERCENT: i32 = 15;
/// Navigation runs an escort contract lasts.
pub const ESCORT_RUNS: u32 = 10;
/// The hull a hired escort flies.
pub const ESCORT_KIND: ShipKind = ShipKind::Patrol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gear {
    Afterburners,
    PointDefense,
}

impl Gear {
    pub const ALL: [Gear; 2] = [Gear::Afterburners, Gear::PointDefense];

    pub fn label(self) -> &'static str {
        match self {
            Gear::Afterburners => "Afterburners",
            Gear::PointDefense => "Point defense",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Gear::Afterburners => "+1 move per battle turn; better odds of evading raiders",
            Gear::PointDefense => "shots at this ship have 15% less chance to hit",
        }
    }
}

/// Whether `gear` is a valid fitted list: known items, each at most once.
pub fn valid_fit(gear: &[Gear]) -> bool {
    gear.len() <= Gear::ALL.len()
        && gear
            .iter()
            .enumerate()
            .all(|(index, item)| !gear[..index].contains(item))
}

/// Gear and escort prices (credits in cents, at a new game's price level).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GearPrices {
    pub afterburners: Credits,
    pub point_defense: Credits,
    /// One escort contract of [`ESCORT_RUNS`] runs.
    pub escort: Credits,
}

impl Default for GearPrices {
    fn default() -> Self {
        Self {
            afterburners: Credits(50_000),
            point_defense: Credits(70_000),
            escort: Credits(25_000),
        }
    }
}

impl GearPrices {
    pub fn of(&self, gear: Gear) -> Credits {
        match gear {
            Gear::Afterburners => self.afterburners,
            Gear::PointDefense => self.point_defense,
        }
    }

    pub fn valid(&self) -> bool {
        [self.afterburners, self.point_defense, self.escort]
            .iter()
            .all(|price| (1..=1_000_000_000).contains(&price.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fit_holds_each_item_once() {
        assert!(valid_fit(&[]));
        assert!(valid_fit(&[Gear::PointDefense, Gear::Afterburners]));
        assert!(!valid_fit(&[Gear::Afterburners, Gear::Afterburners]));
        assert!(GearPrices::default().valid());
    }
}
