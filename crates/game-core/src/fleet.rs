//! Owned fleet membership and data-driven ship specifications.
#[cfg(test)]
mod catalog_tests;
mod operations;
use crate::{
    economy::Credits,
    travel::{PlayerId, ShipId},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FleetId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShipKind {
    Freighter,
    Scout,
    Patrol,
    /// Heavy mining rig: slow, tough and a deep hold for bulk ore.
    Prospector,
}

/// Number of hull classes; sizes per-class arrays such as shipyard stock.
pub const SHIP_KIND_COUNT: usize = 4;
/// Hull classes before the prospector; older saves and configs list this many.
const LEGACY_SHIP_KIND_COUNT: usize = 3;

/// Reads a per-class stock array in `ShipKind::ALL` order. Arrays written
/// before the prospector existed have three entries; the missing classes
/// start with zero stock, so those saves keep loading.
pub fn per_class_stock<'de, D>(deserializer: D) -> Result<[u32; SHIP_KIND_COUNT], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let stock = Vec::<u32>::deserialize(deserializer)?;
    if !(LEGACY_SHIP_KIND_COUNT..=SHIP_KIND_COUNT).contains(&stock.len()) {
        return Err(serde::de::Error::invalid_length(
            stock.len(),
            &"one stock count per ship class",
        ));
    }
    let mut padded = [0; SHIP_KIND_COUNT];
    padded[..stock.len()].copy_from_slice(&stock);
    Ok(padded)
}

impl ShipKind {
    pub const ALL: [Self; SHIP_KIND_COUNT] =
        [Self::Freighter, Self::Scout, Self::Patrol, Self::Prospector];

    /// Position in per-class arrays (`ship_stock`); matches `ALL`.
    pub fn index(self) -> usize {
        match self {
            Self::Freighter => 0,
            Self::Scout => 1,
            Self::Patrol => 2,
            Self::Prospector => 3,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Freighter => "Freighter",
            Self::Scout => "Scout",
            Self::Patrol => "Patrol ship",
            Self::Prospector => "Prospector",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fleet {
    pub id: FleetId,
    pub name: String,
    pub owner: PlayerId,
    pub ships: Vec<ShipId>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipSpec {
    pub kind: ShipKind,
    pub price: Credits,
    pub cargo_capacity: u32,
    pub fuel_capacity: u32,
    pub range_ly: u32,
    pub combat: crate::combat::CombatProfile,
}

/// Ship catalog schema; version 2 added per-hull combat profiles.
pub const SHIP_CATALOG_SCHEMA: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShipCatalog {
    pub schema_version: u32,
    pub ships: Vec<ShipSpec>,
}

impl Default for ShipCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../config/ships.json"))
            .expect("embedded ship definitions must validate")
    }
}

impl ShipCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn spec(&self, kind: ShipKind) -> Option<&ShipSpec> {
        self.ships.iter().find(|spec| spec.kind == kind)
    }

    /// Structural checks; weapon references are checked against the combat
    /// catalog by `EconomyRules::validate`.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SHIP_CATALOG_SCHEMA || self.ships.len() != SHIP_KIND_COUNT {
            return Err("Unsupported ship catalog".into());
        }
        for (index, ship) in self.ships.iter().enumerate() {
            if self.ships[..index]
                .iter()
                .any(|other| other.kind == ship.kind)
                || ship.price.0 == 0
                || ship.price.0 > 1_000_000_000
                || !(1..=10_000).contains(&ship.cargo_capacity)
                || !(1..=1_000_000).contains(&ship.fuel_capacity)
                || !(1..=1_000_000).contains(&ship.range_ly)
            {
                return Err("Invalid ship specification or duplicate class".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FleetError {
    UnknownFleet,
    DifferentLocations,
    NotDocked,
    InvalidSelection,
    SameFleet,
    ShipLimit,
    NotOwner,
    UnknownClass,
    InsufficientCredits,
    InvalidQuantity,
    MissingCargo,
    CargoFull,
    LastShip,
    InvalidName,
    ArithmeticOverflow,
}

impl std::fmt::Display for FleetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownFleet => "Unknown fleet.",
            Self::DifferentLocations => "Ships must be together at the same stationary location.",
            Self::NotDocked => "Dock at a trade port before purchasing ships.",
            Self::InvalidSelection => "Select some, but not all, ships to split a fleet.",
            Self::SameFleet => "Choose two different fleets.",
            Self::ShipLimit => "The prototype supports up to 64 owned ships.",
            Self::NotOwner => "The player does not own this fleet or ship.",
            Self::UnknownClass => "Unknown ship class.",
            Self::InsufficientCredits => "Not enough credits to buy this ship.",
            Self::InvalidQuantity => "Choose a positive amount and a different target ship.",
            Self::MissingCargo => "The source ship does not carry that much cargo.",
            Self::LastShip => "Keep at least one owned ship.",
            Self::InvalidName => "Fleet names must contain 1–64 readable characters.",
            Self::CargoFull => "The target ship has insufficient cargo space.",
            Self::ArithmeticOverflow => "Fleet identity or quantity limit reached.",
        })
    }
}

/// Errors shared by the pure movement planner and the session boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FleetMovementError {
    Movement(crate::movement::MovementError),
    Navigation(crate::navigation::NavigationError),
    Travel(crate::travel::TravelError),
    Fleet(FleetError),
}
impl From<crate::movement::MovementError> for FleetMovementError {
    fn from(error: crate::movement::MovementError) -> Self {
        Self::Movement(error)
    }
}
impl From<crate::travel::TravelError> for FleetMovementError {
    fn from(error: crate::travel::TravelError) -> Self {
        Self::Travel(error)
    }
}
impl From<FleetError> for FleetMovementError {
    fn from(error: FleetError) -> Self {
        Self::Fleet(error)
    }
}
impl std::fmt::Display for FleetMovementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Movement(error) => error.fmt(f),
            Self::Navigation(error) => error.fmt(f),
            Self::Travel(error) => error.fmt(f),
            Self::Fleet(error) => error.fmt(f),
        }
    }
}

impl From<crate::navigation::NavigationError> for FleetMovementError {
    fn from(error: crate::navigation::NavigationError) -> Self {
        Self::Navigation(error)
    }
}
