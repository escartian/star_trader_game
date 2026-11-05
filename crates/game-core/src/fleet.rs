//! Owned fleet membership and data-driven ship specifications.
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
}

impl ShipKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Freighter => "Freighter",
            Self::Scout => "Scout",
            Self::Patrol => "Patrol ship",
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
}

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

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.ships.len() != 3 {
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
    InvalidSelection,
    SameFleet,
    ShipLimit,
    NotOwner,
    UnknownClass,
    InsufficientCredits,
    InvalidQuantity,
    MissingCargo,
    CargoFull,
    ArithmeticOverflow,
}

impl std::fmt::Display for FleetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::UnknownFleet => "Unknown fleet.",
            Self::DifferentLocations => "Ships must be docked at the same system.",
            Self::InvalidSelection => "Select some, but not all, ships to split a fleet.",
            Self::SameFleet => "Choose two different fleets.",
            Self::ShipLimit => "The prototype supports up to 64 owned ships.",
            Self::NotOwner => "The player does not own this fleet or ship.",
            Self::UnknownClass => "Unknown ship class.",
            Self::InsufficientCredits => "Not enough credits to buy this ship.",
            Self::InvalidQuantity => "Choose a positive amount and a different target ship.",
            Self::MissingCargo => "The source ship does not carry that much cargo.",
            Self::CargoFull => "The target ship has insufficient cargo space.",
            Self::ArithmeticOverflow => "Fleet identity or quantity limit reached.",
        })
    }
}
