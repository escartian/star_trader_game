use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use crate::GAME_ID;
use crate::models::planet::{PlanetSpecialization, Economy};
use crate::models::resource::{Resource, ResourceType};
use strum::IntoEnumIterator;
use rand::Rng;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Market {
    pub planet_name: String,
    pub system_id: usize,
    pub planet_id: usize,
    pub specialization: PlanetSpecialization,
    pub economy: Economy,
    pub resources: Vec<Resource>,
}

impl Market {
    pub fn new(planet_name: String, system_id: usize, planet_id: usize, specialization: PlanetSpecialization, economy: Economy) -> Market {
        let resources = generate_market_resources(&specialization, &economy);
        Market {
            planet_name,
            system_id,
            planet_id,
            specialization,
            economy,
            resources,
        }
    }

    pub fn load(system_id: usize, planet_id: usize) -> std::io::Result<Market> {
        let market_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("markets")
            .join(format!("Star_System_{}_Planet_{}_market.json", system_id, planet_id));

        if market_path.exists() {
            let contents = fs::read_to_string(market_path)?;
            Ok(serde_json::from_str(&contents)?)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Market not found for system {} planet {}", system_id, planet_id),
            ))
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let market_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("markets")
            .join(format!("Star_System_{}_Planet_{}_market.json", self.system_id, self.planet_id));

        if let Some(parent) = market_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let market_json = serde_json::to_string(self)?;
        fs::write(market_path, market_json)
    }

    pub fn needs_update(&self, specialization: &PlanetSpecialization, economy: &Economy) -> bool {
        self.specialization != *specialization || self.economy != *economy
    }

    pub fn update(&mut self, specialization: &PlanetSpecialization, economy: &Economy) -> std::io::Result<()> {
        self.specialization = specialization.clone();
        self.economy = economy.clone();
        self.resources = generate_market_resources(specialization, economy);
        self.save()
    }

    pub fn buy_resource(&mut self, resource_type: ResourceType, quantity: u32) -> Result<f32, String> {
        if let Some(resource) = self.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(buy_price) = resource.buy_price() {
                if let Some(available_quantity) = resource.quantity {
                    if available_quantity >= quantity {
                        resource.quantity = Some(available_quantity - quantity);
                        self.save().map_err(|e| e.to_string())?;
                        return Ok(quantity as f32 * buy_price);
                    }
                    return Err(format!("Not enough {} available. Requested: {}, Available: {}", 
                        resource_type, quantity, available_quantity));
                }
                return Err(format!("{} is currently out of stock", resource_type));
            }
            return Err(format!("{} does not buy {} at this time", self.planet_name, resource_type));
        }
        Err(format!("{} does not trade {} at all", self.planet_name, resource_type))
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32) -> Result<f32, String> {
        if let Some(resource) = self.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(sell_price) = resource.sell_price() {
                if let Some(available_quantity) = resource.quantity {
                    resource.quantity = Some(available_quantity + quantity);
                    self.save().map_err(|e| e.to_string())?;
                    return Ok(quantity as f32 * sell_price);
                }
                resource.quantity = Some(quantity);
                self.save().map_err(|e| e.to_string())?;
                return Ok(quantity as f32 * sell_price);
            }
            return Err(format!("{} does not buy {} at this time", self.planet_name, resource_type));
        }
        Err(format!("{} does not trade {} at all", self.planet_name, resource_type))
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