//! Fixed-unit commerce. Prices, stock and cargo are authoritative simulation data.
use serde::{Deserialize, Serialize};
mod bounties;
pub(crate) mod cargo_json;
mod market;
mod rules;
pub use bounties::RaiderBounties;
pub use rules::{EconomyRules, MarketTemplate};
mod services;
mod shipyard;
mod transactions;
use crate::{BodyId, SystemId, travel::Ship};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Credits(pub u64);

impl std::fmt::Display for Credits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:02}", self.0 / 100, self.0 % 100)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Commodity {
    Food,
    Ore,
    Medicine,
    Machinery,
    Textiles,
    Electronics,
}

impl Commodity {
    pub const ALL: [Self; 6] = [
        Self::Food,
        Self::Ore,
        Self::Medicine,
        Self::Machinery,
        Self::Textiles,
        Self::Electronics,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Food => "Food",
            Self::Ore => "Ore",
            Self::Medicine => "Medicine",
            Self::Machinery => "Machinery",
            Self::Textiles => "Textiles",
            Self::Electronics => "Electronics",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeSide {
    Buy,
    Sell,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketGood {
    pub stock: u32,
    pub target: u32,
    pub base_price: Credits,
    pub production: u32,
    pub consumption: u32,
}

impl MarketGood {
    pub fn capacity(&self) -> u32 {
        self.target.saturating_mul(2)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketId {
    pub system: SystemId,
    pub body: BodyId,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Market {
    pub id: MarketId,
    pub is_station: bool,
    pub credits: Credits,
    #[serde(deserialize_with = "crate::fleet::per_class_stock")]
    pub ship_stock: [u32; crate::fleet::SHIP_KIND_COUNT],
    pub goods: [MarketGood; 6],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Economy {
    pub credits: Credits,
    #[serde(default)]
    pub traders: Vec<crate::npc::Trader>,
    pub markets: Vec<Market>,
    pub rules: EconomyRules,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TradeQuote {
    pub market: MarketId,
    pub commodity: Commodity,
    pub side: TradeSide,
    pub quantity: u32,
    pub first_unit_price: Credits,
    pub total: Credits,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomyError {
    MustBeDocked,
    UnknownMarket,
    InvalidQuantity,
    InsufficientCredits,
    MarketInsufficientCredits,
    ShipOutOfStock,
    NonEmptyHold,
    CargoFull,
    InsufficientStock,
    InsufficientCargo,
    StockFull,
    FuelFull,
    AlreadyUpgraded,
    NoDamage,
    ArithmeticOverflow,
}

impl std::fmt::Display for EconomyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MustBeDocked => "Arrive at a market before trading or refuelling.",
            Self::UnknownMarket => "There is no market at this body.",
            Self::MarketInsufficientCredits => "The local market cannot afford this purchase.",
            Self::NonEmptyHold => "Empty this ship’s cargo before selling it.",
            Self::ShipOutOfStock => "This ship class is out of stock here.",
            Self::InvalidQuantity => "Choose a positive quantity.",
            Self::InsufficientCredits => "Not enough credits.",
            Self::CargoFull => "Not enough cargo space.",
            Self::InsufficientStock => "The market does not have enough stock.",
            Self::InsufficientCargo => "You do not have enough of this cargo.",
            Self::StockFull => "The market cannot store that much cargo.",
            Self::FuelFull => "That amount would exceed the fuel tank capacity.",
            Self::AlreadyUpgraded => "The cargo hold is already upgraded.",
            Self::NoDamage => "This ship has no hull damage to repair.",
            Self::ArithmeticOverflow => {
                "The transaction exceeds supported quantity or credit limits."
            }
        })
    }
}

impl Economy {
    pub fn market(&self, id: MarketId) -> Result<&Market, EconomyError> {
        self.markets
            .iter()
            .find(|market| market.id == id)
            .ok_or(EconomyError::UnknownMarket)
    }

    pub fn market_mut(&mut self, id: MarketId) -> Result<&mut Market, EconomyError> {
        self.markets
            .iter_mut()
            .find(|market| market.id == id)
            .ok_or(EconomyError::UnknownMarket)
    }

    pub fn current_market(&self, ship: &Ship) -> Result<&Market, EconomyError> {
        let id = match ship.location {
            // Primary station IDs are stable in generated worlds and legacy saves.
            crate::travel::Location::Docked(system) => self
                .markets
                .iter()
                .find(|market| market.id.system == system && market.is_station)
                .map(|market| market.id)
                .ok_or(EconomyError::UnknownMarket)?,
            // A planet's market serves fleets in orbit and on its surface alike.
            crate::travel::Location::Orbiting { system, body }
            | crate::travel::Location::Landed { system, body } => MarketId { system, body },
            _ => return Err(EconomyError::MustBeDocked),
        };
        self.market(id)
    }
}

#[cfg(test)]
mod tests;
