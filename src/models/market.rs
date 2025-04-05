use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use crate::models::settings::load_settings;
use crate::models::planet::PlanetSpecialization;
use crate::models::economy::Economy;
use crate::models::resource::{Resource, ResourceType};
use strum::IntoEnumIterator;
use rand::Rng;
use crate::models::ship::ship::Ship;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    pub resources: Vec<Resource>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShipMarket {
    pub ships: Vec<Ship>
}

impl Default for Market {
    fn default() -> Self {
        Market {
            resources: Vec::new()
        }
    }
}

impl Default for ShipMarket {
    fn default() -> Self {
        ShipMarket {
            ships: Vec::new()
        }
    }
}

impl Market {
    pub fn new(specialization: &PlanetSpecialization, economy: &Economy) -> Market {
        let resources = generate_market_resources(specialization, economy);
        Market {
            resources
        }
    }

    pub fn load(system_id: usize, planet_id: usize) -> std::io::Result<Market> {
        let settings = load_settings()?;
        let market_dir = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets");

        // Create market directory if it doesn't exist
        if !market_dir.exists() {
            fs::create_dir_all(&market_dir)?;
        }

        let market_path = market_dir.join(format!("market_{}_{}.json", system_id, planet_id));

        if market_path.exists() {
            let file = File::open(market_path)?;
            Ok(serde_json::from_reader(file)?)
        } else {
            // If market doesn't exist, create a new one
            let planet = crate::models::planet::load_planet(system_id, planet_id)?
                .ok_or_else(|| std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Planet not found for system {} planet {}", system_id, planet_id),
                ))?;
            
            let market = Market::new(&planet.specialization, &planet.economy);
            
            // Save the new market
            market.save(system_id, planet_id)?;
            Ok(market)
        }
    }

    pub fn save(&self, system_id: usize, planet_id: usize) -> std::io::Result<()> {
        let settings = load_settings()?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("market_{}_{}.json", system_id, planet_id));

        if let Some(parent) = market_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let market_json = serde_json::to_string_pretty(self)?;
        fs::write(market_path, market_json)
    }

    pub fn needs_update(&self, specialization: &PlanetSpecialization, economy: &Economy) -> bool {
        // Since we don't store specialization and economy anymore,
        // we'll need to regenerate the market each time
        true
    }

    pub fn update(&mut self, specialization: &PlanetSpecialization, economy: &Economy) -> std::io::Result<()> {
        self.resources = generate_market_resources(specialization, economy);
        Ok(())
    }

    pub fn buy_resource(&mut self, resource_type: ResourceType, quantity: u32, system_id: usize, planet_id: usize) -> Result<u32, &'static str> {
        let resource = self.resources.iter_mut().find(|r| r.resource_type == resource_type)
            .ok_or("Resource not available in market")?;
        
        let available_quantity = resource.quantity.unwrap_or(0);
        if available_quantity < quantity {
            return Err("Not enough resources available");
        }
        
        let buy_price = resource.buy.ok_or("Resource cannot be bought")?;
        let total_cost = (buy_price * quantity as f32) as u32;
        resource.quantity = Some(available_quantity - quantity);
        
        Ok(total_cost)
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32, system_id: usize, planet_id: usize) -> Result<u32, &'static str> {
        let resource = self.resources.iter_mut().find(|r| r.resource_type == resource_type)
            .ok_or("Resource not available in market")?;
        
        let current_quantity = resource.quantity.unwrap_or(0);
        let sell_price = resource.sell.ok_or("Resource cannot be sold")?;
        let total_value = (sell_price * quantity as f32) as u32;
        resource.quantity = Some(current_quantity + quantity);
        
        Ok(total_value)
    }
}

fn generate_market_resources(specialization: &PlanetSpecialization, economy: &Economy) -> Vec<Resource> {
    let mut resources = Vec::new();
    let mut rng = rand::thread_rng();
    
    // Base price multiplier based on economy
    let economy_multiplier = match economy {
        Economy::Booming => 1.5,
        Economy::Growing => 1.2,
        Economy::Stable => 1.0,
        Economy::Struggling => 0.8,
        Economy::Declining => 0.6,
        Economy::Crashing => 0.4,
        Economy::Nonexistent => 0.2,
    };

    // Generate market resources based on specialization
    for resource_type in ResourceType::iter() {
        let (buy_price, sell_price) = match resource_type {
            // Essential resources that all planets should trade
            ResourceType::Water | ResourceType::Food | ResourceType::Fuel => {
                let base_buy = 1.3;  // Higher buy price
                let base_sell = 0.7; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Common resources that most planets trade
            ResourceType::Minerals | ResourceType::Metals | ResourceType::Electronics => {
                let base_buy = 1.2;  // Higher buy price
                let base_sell = 0.8; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Luxury goods that most planets trade but with higher prices
            ResourceType::LuxuryGoods => {
                let base_buy = 1.5;  // Higher buy price
                let base_sell = 1.0; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Narcotics - restricted based on specialization and economy
            ResourceType::Narcotics => {
                match (specialization, economy) {
                    (PlanetSpecialization::Research, _) => (Some(1.8), Some(1.2)),  // Higher buy, lower sell
                    (_, Economy::Crashing | Economy::Nonexistent) => (Some(2.0), Some(1.5)),  // Higher buy, lower sell
                    _ => (None, None),
                }
            },
        };

        // Apply economy multiplier to prices
        let buy_price = buy_price.map(|p| p * economy_multiplier);
        let sell_price = sell_price.map(|p| p * economy_multiplier);

        // Generate random quantity if the planet trades this resource
        let quantity = if buy_price.is_some() || sell_price.is_some() {
            Some(rng.gen_range(10..100))
        } else {
            None
        };

        resources.push(Resource {
            resource_type,
            buy: buy_price,
            sell: sell_price,
            quantity,
        });
    }

    resources
} 