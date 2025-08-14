use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::Path;
use crate::models::settings::load_settings;
use crate::models::planet::PlanetSpecialization;
use crate::models::economy::Economy;
use crate::models::resource::{Resource, ResourceType};
use strum::IntoEnumIterator;
use rand::Rng;
use crate::models::ship::ship::{Ship, ShipSize, ShipType, ShipEngine};
use crate::models::game_state::{load_json, save_json, game_path};
use crate::models::game_world::get_global_game_world;
use std::error::Error;
use rand::thread_rng;

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

    pub fn buy_resource(&mut self, resource_type: ResourceType, quantity: u32, _system_id: usize, _planet_id: usize) -> Result<f64, &'static str> {
        let resource = self.resources.iter_mut().find(|r| r.resource_type == resource_type)
            .ok_or("Resource not available in market")?;
        
        let available_quantity = resource.quantity.unwrap_or(0);
        if available_quantity < quantity {
            return Err("Not enough resources available");
        }
        
        let buy_price = resource.buy.ok_or("Resource cannot be bought")?;
        let total_cost = ((buy_price * quantity as f64) * 100.0).round() / 100.0;
        resource.quantity = Some(available_quantity - quantity);
        
        Ok(total_cost)
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32, _system_id: usize, _planet_id: usize) -> Result<f64, &'static str> {
        let resource = self.resources.iter_mut().find(|r| r.resource_type == resource_type)
            .ok_or("Resource not available in market")?;
        
        let current_quantity = resource.quantity.unwrap_or(0);
        let sell_price = resource.sell.ok_or("Resource cannot be sold")?;
        let total_value = ((sell_price * quantity as f64) * 100.0).round() / 100.0;
        resource.quantity = Some(current_quantity + quantity);
        
        Ok(total_value)
    }
}

pub fn generate_market_resources(specialization: &PlanetSpecialization, economy: &Economy) -> Vec<Resource> {
    let mut resources = Vec::new();
    let mut rng = rand::thread_rng();
    
    // Base price multiplier based on economy
    let economy_multiplier: f64 = match economy {
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
        let (buy_price, sell_price): (Option<f64>, Option<f64>) = match resource_type {
            // Essential resources that all planets should trade
            ResourceType::Water | ResourceType::Food | ResourceType::Fuel => {
                let base_buy = 1.3_f64;  // Higher buy price
                let base_sell = 0.7_f64; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Common resources that most planets trade
            ResourceType::Minerals | ResourceType::Metals | ResourceType::Electronics => {
                let base_buy = 1.2_f64;  // Higher buy price
                let base_sell = 0.8_f64; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Luxury goods that most planets trade but with higher prices
            ResourceType::LuxuryGoods => {
                let base_buy = 1.5_f64;  // Higher buy price
                let base_sell = 1.0_f64; // Lower sell price
                (Some(base_buy), Some(base_sell))
            },
            // Narcotics - restricted based on specialization and economy
            ResourceType::Narcotics => {
                match (specialization, economy) {
                    (PlanetSpecialization::Research, _) => (Some(1.8_f64), Some(1.2_f64)),  // Higher buy, lower sell
                    (_, Economy::Crashing | Economy::Nonexistent) => (Some(2.0_f64), Some(1.5_f64)),  // Higher buy, lower sell
                    _ => (None, None),
                }
            },
        };

        // Apply economy multiplier to prices
        let buy = buy_price.map(|p| p * economy_multiplier);
        let sell = sell_price.map(|p| p * economy_multiplier);

        // Generate random quantity if the planet trades this resource
        let quantity = if buy.is_some() || sell.is_some() {
            Some(rng.gen_range(10..100))
        } else {
            None
        };

        resources.push(Resource {
            resource_type,
            buy,
            sell,
            quantity,
        });
    }

    resources
}

pub fn generate_market_for_planet(planet_name: &str, system_id: usize, planet_id: usize, specialization: &PlanetSpecialization, economy: &Economy) -> Market {
    Market {
        resources: generate_market_resources(specialization, economy)
    }
}

pub fn generate_ship_market() -> ShipMarket {
    println!("Starting ship market generation");
    let mut rng = rand::thread_rng();
    let ship_count = rng.gen_range(3..=8); // Generate 3-8 ships
    println!("Generating {} ships", ship_count);
    let mut ships = Vec::with_capacity(ship_count);

    for i in 0..ship_count {
        println!("Generating ship {}/{}", i + 1, ship_count);
        let mut ship: Ship = rand::random();
        println!("Generated ship: {}", ship.name);
        ship.price = Some(calculate_ship_price(&ship));
        ships.push(ship);
    }

    println!("Completed ship market generation with {} ships", ships.len());
    ShipMarket { ships }
}

pub fn calculate_ship_price(ship: &Ship) -> f64 {
    let base_price = match ship.size {
        ShipSize::Tiny => 1000.0,
        ShipSize::Small => 2500.0,
        ShipSize::Medium => 5000.0,
        ShipSize::Large => 10000.0,
        ShipSize::Huge => 20000.0,
        ShipSize::Planetary => 50000.0,
    };

    let specialization_multiplier = match ship.specialization {
        ShipType::Fighter => 1.1,
        ShipType::Battleship => 1.8,
        ShipType::Freighter => 1.3,
        ShipType::Explorer => 1.5,
        ShipType::Shuttle => 0.7,
        ShipType::Capital => 2.5,
    };

    let engine_multiplier = match ship.engine {
        ShipEngine::Basic => 0.8,
        ShipEngine::Advanced => 1.2,
        ShipEngine::Experimental => 1.5,
    };

    let condition_multiplier = (ship.hp as f64 / 100.0).max(0.5);

    base_price * specialization_multiplier * engine_multiplier * condition_multiplier
}

pub fn regenerate_system_markets(system_id: usize) -> Result<(), Box<dyn Error>> {
    println!("Starting market regeneration for system {}", system_id);
    let settings = load_settings()?;
    let game_path = Path::new("data").join("game").join(&settings.game_id);
    let markets_path = game_path.join("markets");
    
    if !markets_path.exists() {
        println!("Creating markets directory at {}", markets_path.display());
        fs::create_dir_all(&markets_path)?;
    }

    println!("Loading game world for system {}", system_id);
    let game_world = get_global_game_world();
    if game_world.is_empty() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Game world is empty. Please ensure the game world is properly loaded."
        )));
    }

    let system = game_world.get(system_id)
        .ok_or_else(|| Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("System {} not found in game world", system_id)
        )))?;

    // Validate the system data
    if system.planets.len() > 20 { // Reasonable upper limit for planets per system
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid number of planets ({}) in system {}", system.planets.len(), system_id)
        )));
    }

    println!("Found system with {} planets", system.planets.len());
    for (planet_id, planet) in system.planets.iter().enumerate() {
        println!("Generating market for planet {}: {}", planet_id, planet.name);
        // Generate and save planet market
        let market = generate_market_for_planet(&planet.name, system_id, planet_id, &planet.specialization, &planet.economy);
        let market_path = markets_path.join(format!("market_{}_{}.json", system_id, planet_id));
        println!("Saving planet market to {}", market_path.display());
        if let Err(e) = save_json(&market_path, &market) {
            println!("Error saving planet market: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save planet market: {}", e))));
        }
        println!("Successfully saved planet market");

        // Generate and save ship market
        println!("Generating ship market for planet {}", planet.name);
        let ship_market = generate_ship_market();
        let ship_market_path = markets_path.join(format!("ships_{}_{}.json", system_id, planet_id));
        println!("Saving ship market to {}", ship_market_path.display());
        if let Err(e) = save_json(&ship_market_path, &ship_market) {
            println!("Error saving ship market: {}", e);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to save ship market: {}", e))));
        }
        println!("Successfully saved ship market");
    }
    println!("Completed market regeneration for system {}", system_id);
    Ok(())
} 