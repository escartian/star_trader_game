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
use crate::{combat::HullSize, economy::Credits, fleet::ShipKind};
use serde::{Deserialize, Serialize};

pub const AFTERBURNER_MOVES: u32 = 1;
pub const AFTERBURNER_EVADE_PERCENT: u32 = 15;
pub const POINT_DEFENSE_PERCENT: i32 = 15;
pub const TARGETING_PERCENT: i32 = 10;
pub const EVASION_JETS_PERCENT: i32 = 10;
pub const SHIELD_ARRAY_POINTS: u32 = 10;
pub const BULKHEAD_ARMOR_POINTS: u32 = 3;
pub const SCOUT_RANGE_LY: u32 = 4;
pub const FREIGHTER_CARGO_UNITS: u32 = 20;
pub const PROSPECTOR_ORE_PER_RUN: u32 = 1;
/// Navigation runs an escort contract lasts.
pub const ESCORT_RUNS: u32 = 10;
/// The hull a hired escort flies.
pub const ESCORT_KIND: ShipKind = ShipKind::Patrol;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gear {
    Afterburners,
    PointDefense,
    ScoutDrive,
    FreighterRacks,
    PatrolTargeting,
    ProspectorDrill,
    EvasionJets,
    ShieldArray,
    ArmoredBulkheads,
}

impl Gear {
    pub const ALL: [Gear; 9] = [
        Gear::Afterburners,
        Gear::PointDefense,
        Gear::ScoutDrive,
        Gear::FreighterRacks,
        Gear::PatrolTargeting,
        Gear::ProspectorDrill,
        Gear::EvasionJets,
        Gear::ShieldArray,
        Gear::ArmoredBulkheads,
    ];
    pub const NPC: [Gear; 2] = [Gear::Afterburners, Gear::PointDefense];

    pub fn label(self) -> &'static str {
        match self {
            Gear::Afterburners => "Afterburners",
            Gear::PointDefense => "Point defense",
            Gear::ScoutDrive => "Scout jump drive",
            Gear::FreighterRacks => "Freighter cargo racks",
            Gear::PatrolTargeting => "Patrol targeting computer",
            Gear::ProspectorDrill => "Prospector drill",
            Gear::EvasionJets => "Evasion jets",
            Gear::ShieldArray => "Shield array",
            Gear::ArmoredBulkheads => "Armored bulkheads",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Gear::Afterburners => "+1 move per battle turn; better odds of evading raiders",
            Gear::PointDefense => "shots at this ship have 15% less chance to hit",
            Gear::ScoutDrive => "+4 light-years of jump range (Scout only)",
            Gear::FreighterRacks => "+20 cargo spaces (Freighter only)",
            Gear::PatrolTargeting => "+10 points to shot accuracy (Patrol only)",
            Gear::ProspectorDrill => "+1 ore per mining run (Prospector only)",
            Gear::EvasionJets => "shots have 10 points less chance to hit (small ships)",
            Gear::ShieldArray => "+10 shields at the start of battle (medium ships)",
            Gear::ArmoredBulkheads => "+3 armor against hull hits (large ships)",
        }
    }

    pub fn fits(self, kind: ShipKind, size: HullSize) -> bool {
        match self {
            Self::Afterburners | Self::PointDefense => true,
            Self::ScoutDrive => kind == ShipKind::Scout,
            Self::FreighterRacks => kind == ShipKind::Freighter,
            Self::PatrolTargeting => kind == ShipKind::Patrol,
            Self::ProspectorDrill => kind == ShipKind::Prospector,
            Self::EvasionJets => size == HullSize::Small,
            Self::ShieldArray => size == HullSize::Medium,
            Self::ArmoredBulkheads => size == HullSize::Large,
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
    #[serde(default = "scout_drive_price")]
    pub scout_drive: Credits,
    #[serde(default = "freighter_racks_price")]
    pub freighter_racks: Credits,
    #[serde(default = "patrol_targeting_price")]
    pub patrol_targeting: Credits,
    #[serde(default = "prospector_drill_price")]
    pub prospector_drill: Credits,
    #[serde(default = "evasion_jets_price")]
    pub evasion_jets: Credits,
    #[serde(default = "shield_array_price")]
    pub shield_array: Credits,
    #[serde(default = "armored_bulkheads_price")]
    pub armored_bulkheads: Credits,
    /// One escort contract of [`ESCORT_RUNS`] runs.
    pub escort: Credits,
}

fn scout_drive_price() -> Credits {
    Credits(80_000)
}
fn freighter_racks_price() -> Credits {
    Credits(90_000)
}
fn patrol_targeting_price() -> Credits {
    Credits(120_000)
}
fn prospector_drill_price() -> Credits {
    Credits(100_000)
}
fn evasion_jets_price() -> Credits {
    Credits(60_000)
}
fn shield_array_price() -> Credits {
    Credits(80_000)
}
fn armored_bulkheads_price() -> Credits {
    Credits(100_000)
}

impl Default for GearPrices {
    fn default() -> Self {
        Self {
            afterburners: Credits(50_000),
            point_defense: Credits(70_000),
            scout_drive: scout_drive_price(),
            freighter_racks: freighter_racks_price(),
            patrol_targeting: patrol_targeting_price(),
            prospector_drill: prospector_drill_price(),
            evasion_jets: evasion_jets_price(),
            shield_array: shield_array_price(),
            armored_bulkheads: armored_bulkheads_price(),
            escort: Credits(25_000),
        }
    }
}

impl GearPrices {
    pub fn of(&self, gear: Gear) -> Credits {
        match gear {
            Gear::Afterburners => self.afterburners,
            Gear::PointDefense => self.point_defense,
            Gear::ScoutDrive => self.scout_drive,
            Gear::FreighterRacks => self.freighter_racks,
            Gear::PatrolTargeting => self.patrol_targeting,
            Gear::ProspectorDrill => self.prospector_drill,
            Gear::EvasionJets => self.evasion_jets,
            Gear::ShieldArray => self.shield_array,
            Gear::ArmoredBulkheads => self.armored_bulkheads,
        }
    }

    pub fn valid(&self) -> bool {
        Gear::ALL
            .into_iter()
            .map(|gear| self.of(gear))
            .chain([self.escort])
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

    #[test]
    fn model_and_size_restrictions_are_distinct() {
        assert!(Gear::ScoutDrive.fits(ShipKind::Scout, HullSize::Small));
        assert!(!Gear::ScoutDrive.fits(ShipKind::Freighter, HullSize::Small));
        assert!(Gear::ShieldArray.fits(ShipKind::Patrol, HullSize::Medium));
        assert!(Gear::ShieldArray.fits(ShipKind::Freighter, HullSize::Medium));
        assert!(!Gear::ShieldArray.fits(ShipKind::Scout, HullSize::Small));
        assert!(Gear::ArmoredBulkheads.fits(ShipKind::Prospector, HullSize::Large));
    }

    #[test]
    fn old_gear_prices_gain_defaults_for_new_upgrades() {
        let old = r#"{"afterburners":50000,"point_defense":70000,"escort":25000}"#;
        let prices: GearPrices = serde_json::from_str(old).unwrap();
        assert_eq!(prices, GearPrices::default());
    }
}
