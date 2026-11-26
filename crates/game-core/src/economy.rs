//! Fixed-unit commerce. Prices, stock and cargo are authoritative simulation data.
use serde::{Deserialize, Serialize};
mod bounties;
pub(crate) mod cargo_json;
mod charters;
mod dynamics;
mod market;
mod rules;
pub use bounties::RaiderBounties;
pub use charters::{CampaignRules, Charter};
pub use dynamics::{MarketDynamics, MarketFlow, PRICE_LEVEL_UNIT, custom_inflation_ppb};
pub use rules::{EconomyRules, MarketTemplate};
mod outfitting;
pub use outfitting::ShipUpgrade;
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
    /// List price at a new game's price level (see `price_level_ppm`).
    pub base_price: Credits,
    pub production: u32,
    pub consumption: u32,
    /// The galaxy's price level (inflation), copied in each tick so a good
    /// prices itself; parts per million of a new game's level.
    #[serde(
        default = "unit_price_level",
        skip_serializing_if = "is_unit_price_level"
    )]
    pub price_level_ppm: u64,
}

fn unit_price_level() -> u64 {
    PRICE_LEVEL_UNIT
}

fn is_unit_price_level(level: &u64) -> bool {
    *level == PRICE_LEVEL_UNIT
}

fn is_zero(value: &u64) -> bool {
    *value == 0
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
    /// Resident shares by Species::ALL, summing to 100 percent.
    #[serde(
        default = "market::legacy_residents",
        deserialize_with = "market::deserialize_residents"
    )]
    pub residents_percent: [u32; 4],
    /// Total residents of this port; species shares are percentages of this count.
    #[serde(default = "market::legacy_population")]
    pub total_population: u64,
    pub credits: Credits,
    #[serde(deserialize_with = "crate::fleet::per_class_stock")]
    pub ship_stock: [u32; crate::fleet::SHIP_KIND_COUNT],
    pub goods: [MarketGood; 6],
    /// Recent trade value (credits in cents), decaying each tick.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub activity: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Economy {
    pub credits: Credits,
    #[serde(default)]
    pub traders: Vec<crate::npc::Trader>,
    pub markets: Vec<Market>,
    pub rules: EconomyRules,
    /// Galaxy-wide price level in parts per million of a new game's prices.
    #[serde(default = "unit_price_level")]
    pub price_level_ppm: u64,
    /// Fractional custom inflation carried across ticks, so 0.01% settings
    /// still take effect. The denominator comes from `MarketDynamics`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub inflation_remainder: u64,
    /// Port charters bought so far (see `charters`). Omitted from saves when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub charters: Vec<Charter>,
    /// Territory, relations and your standing (`factions`); none in play by default.
    #[serde(default)]
    pub factions: crate::factions::Factions,
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
    CargoFull,
    InsufficientStock,
    InsufficientCargo,
    StockFull,
    FuelFull,
    AlreadyUpgraded,
    NoDamage,
    AlreadyChartered,
    /// That gear is already fitted to this ship.
    AlreadyFitted,
    NotFitted,
    NoShipyard,
    /// The fitting is restricted to another hull model or size.
    IncompatibleGear,
    /// This port's faction is hostile to you (`factions::Tier::Hostile`).
    PortClosed,
    /// The port's faction does not want you: there is no fine to pay.
    NotWanted,
    ArithmeticOverflow,
}

impl std::fmt::Display for EconomyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MustBeDocked => "Arrive at a market before trading or refuelling.",
            Self::UnknownMarket => "There is no market at this body.",
            Self::MarketInsufficientCredits => "The local market cannot afford this purchase.",
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
            Self::AlreadyChartered => "This port's charter has already been sold.",
            Self::AlreadyFitted => "This ship already has that gear fitted.",
            Self::NotFitted => "That upgrade is not fitted to this ship.",
            Self::NoShipyard => "This port has no active shipyard.",
            Self::IncompatibleGear => "This upgrade does not fit this ship model or size.",
            Self::PortClosed => {
                "This port's faction is hostile to you, or wants you for your crimes, and refuses your business."
            }
            Self::NotWanted => "This port's faction does not want you; there is no fine to pay.",
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
mod dynamics_tests;
#[cfg(test)]
mod tests;
