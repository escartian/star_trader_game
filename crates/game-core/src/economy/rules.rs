use super::{Commodity, Credits};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarketTemplate {
    pub base_prices: [Credits; 6],
    pub stock: u32,
    pub target: u32,
    pub production: [u32; 6],
    pub consumption: [u32; 6],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EconomyRules {
    pub schema_version: u32,
    pub starting_credits: Credits,
    pub cargo_capacity: u32,
    pub fuel_unit_price: Credits,
    pub hold_upgrade_price: Credits,
    pub hold_upgrade_capacity: u32,
    pub commodity_order: [Commodity; 6],
    pub markets: Vec<MarketTemplate>,
    #[serde(default = "default_market_credits")]
    pub market_starting_credits: Credits,
    #[serde(default = "default_ship_stock")]
    pub shipyard_starting_stock: [u32; 3],
    #[serde(default = "default_resale_percent")]
    pub ship_resale_percent: u32,
    #[serde(default)]
    pub planet_catalog: crate::planet::PlanetCatalog,
    #[serde(default)]
    pub ship_catalog: crate::fleet::ShipCatalog,
}

fn default_resale_percent() -> u32 {
    60
}
fn default_market_credits() -> Credits {
    Credits(1_000_000)
}
fn default_ship_stock() -> [u32; 3] {
    [2, 2, 1]
}

impl Default for EconomyRules {
    fn default() -> Self {
        Self::from_json(include_str!("../../../../config/economy.json"))
            .expect("the embedded economy definition must pass validation")
    }
}

impl EconomyRules {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let rules: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        rules.validate()?;
        Ok(rules)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.ship_catalog.validate()?;
        self.planet_catalog.validate()?;
        if !(1..=90).contains(&self.ship_resale_percent)
            || self.market_starting_credits.0 > 1_000_000_000_000
            || self
                .shipyard_starting_stock
                .iter()
                .any(|stock| *stock > 1000)
        {
            return Err("Invalid market allocation or planet market template".into());
        }
        if self.schema_version != 1 {
            return Err("Unsupported economy schema version".into());
        }
        if self.commodity_order != Commodity::ALL {
            return Err("Unexpected commodity ordering".into());
        }
        if self.markets.is_empty() || self.markets.len() > 64 {
            return Err("Expected 1–64 market templates".into());
        }
        if !(1..=10_000).contains(&self.cargo_capacity)
            || !(1..=10_000).contains(&self.hold_upgrade_capacity)
        {
            return Err("Invalid cargo capacity/upgrade".into());
        }
        if self.starting_credits.0 > 1_000_000_000_000
            || self.fuel_unit_price.0 == 0
            || self.fuel_unit_price.0 > 1_000_000_000
            || self.hold_upgrade_price.0 == 0
            || self.hold_upgrade_price.0 > 1_000_000_000_000
        {
            return Err("Invalid initial credits or service prices".into());
        }
        for market in &self.markets {
            if !(1..=1000).contains(&market.target)
                || market.stock > market.target * 2
                || market
                    .base_prices
                    .iter()
                    .any(|price| price.0 == 0 || price.0 > 1_000_000_000)
                || market
                    .production
                    .iter()
                    .chain(&market.consumption)
                    .any(|rate| *rate > 1000)
            {
                return Err("Invalid market stock, price or production rate".into());
            }
        }
        Ok(())
    }
}
