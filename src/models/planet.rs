use crate::constants::PRINT_DEBUG;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use std::fs::{self, File};
use std::path::Path;
use crate::models::settings::load_settings;
use crate::models::game_state::game_path;
use crate::models::economy::Economy;
use crate::models::market::{Market, ShipMarket};

use super::position::{random_nonzero_position, Position};
use super::resource::{Resource, ResourceType};
use super::player::Player;
use crate::models::ship::ship::{Ship, ShipEngine, ShipSize, ShipType};
use crate::models::fleet::Fleet;
use crate::models::trade::{buy_from_planet, sell_to_planet};

// TODO: Implement planet factions and relationships
// TODO: Add planet population and development levels
// TODO: Implement planet events and disasters

//PLANET DETAILS
// Define a struct to represent a planet
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Planet {
    pub name: String,
    pub description: String,
    pub position: Position,
    pub economy: Economy,
    pub specialization: PlanetSpecialization,
    pub danger: PlanetDanger,
    pub biome: Biome,
    pub credits: f32,
    pub market: Market,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlanetDanger {
    VerySafe,
    Safe,
    Harmless,
    Benign,
    Normal,
    Tainted,
    Hazardous,
    Corrosive,
    Deadly,
    Insidious,
}
impl fmt::Display for PlanetDanger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PlanetDanger::VerySafe => write!(f, "VerySafe"),
            PlanetDanger::Safe => write!(f, "Safe"),
            PlanetDanger::Harmless => write!(f, "Harmless"),
            PlanetDanger::Benign => write!(f, "Benign"),
            PlanetDanger::Normal => write!(f, "Normal"),
            PlanetDanger::Tainted => write!(f, "Tainted"),
            PlanetDanger::Hazardous => write!(f, "Hazardous"),
            PlanetDanger::Corrosive => write!(f, "Corrosive"),
            PlanetDanger::Deadly => write!(f, "Deadly"),
            PlanetDanger::Insidious => write!(f, "Insidious"),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Biome {
    Terran,
    Jungle,
    Ocean,
    Arid,
    Steppe,
    Desert,
    Minimal,
    Barren,
    Tundra,
    Dead,
    Inferno,
    Toxic,
    Radiated,
    Inhospitable,
}

impl fmt::Display for Biome {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, EnumIter)]
pub enum PlanetSpecialization {
    Agriculture,
    Mining,
    Manufacturing,
    Technology,
    Research,
    Tourism,
    Service,
    None,
}

impl Distribution<PlanetDanger> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PlanetDanger {
        match rng.gen_range(0..10) {
            0 => PlanetDanger::VerySafe,
            1 => PlanetDanger::Safe,
            2 => PlanetDanger::Harmless,
            3 => PlanetDanger::Benign,
            4 => PlanetDanger::Normal,
            5 => PlanetDanger::Tainted,
            6 => PlanetDanger::Hazardous,
            7 => PlanetDanger::Corrosive,
            8 => PlanetDanger::Deadly,
            _ => PlanetDanger::Insidious,
        }
    }
}

impl Distribution<Biome> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Biome {
        match rng.gen_range(0..12) {
            0 => Biome::Terran,
            1 => Biome::Jungle,
            2 => Biome::Ocean,
            3 => Biome::Arid,
            4 => Biome::Steppe,
            5 => Biome::Desert,
            6 => Biome::Minimal,
            7 => Biome::Barren,
            8 => Biome::Tundra,
            9 => Biome::Dead,
            10 => Biome::Inferno,
            11 => Biome::Radiated,
            _ => Biome::Inhospitable,
        }
    }
}

impl Distribution<PlanetSpecialization> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> PlanetSpecialization {
        match rng.gen_range(0..7) {
            0 => PlanetSpecialization::Agriculture,
            1 => PlanetSpecialization::Mining,
            2 => PlanetSpecialization::Manufacturing,
            3 => PlanetSpecialization::Technology,
            4 => PlanetSpecialization::Tourism,
            5 => PlanetSpecialization::Service,
            6 => PlanetSpecialization::Research,
            _ => PlanetSpecialization::None,
        }
    }
}

pub fn generate_planets(
    num_planets: u32,
    map_width: i32,
    map_height: i32,
    map_length: i32,
) -> Vec<Planet> {
    // Initialize a vector to hold the planets
    let mut planets = Vec::with_capacity(num_planets as usize);
    let mut unique_positions: HashSet<Position> = HashSet::new();

    // Loop to generate the specified number of planets
    for i in 0..num_planets {
        // Generate a name for the planet
        let name = format!("Planet {}", i + 1);
        let mut position;
        let mut attempts = 0;
        
        // Try to find a unique position
        loop {
            position = random_nonzero_position(map_width, map_height, map_length);
            if !unique_positions.contains(&position) || attempts >= 10 {
                break;
            }
            attempts += 1;
        }
        
        if attempts >= 10 {
            continue; // Skip this planet if we couldn't find a unique position
        }
        
        unique_positions.insert(position);
        let economy: Economy = rand::random();
        let specialization: PlanetSpecialization = rand::random();
        let biome: Biome = rand::random();
        let danger: PlanetDanger = rand::random();
        
        let market = Market::new(&specialization, &economy);
        let planet = Planet {
            name,
            description: format!("A {} planet with {} economy", biome, economy),
            position,
            economy,
            specialization,
            danger,
            biome,
            credits: 0.0,
            market,
        };

        // Add the planet to the vector of planets
        planets.push(planet);
    }

    remove_colliding_planets(&mut planets);
    // Return the vector of planets
    planets
}

fn remove_colliding_planets(planets: &mut Vec<Planet>) {
    let mut unique_positions: HashSet<Position> = HashSet::new();
    let mut i = 0;
    
    while i < planets.len() {
        if unique_positions.contains(&planets[i].position) {
            if PRINT_DEBUG {
                println!("Planets generated in same location. Removing duplicate");
            }
            planets.remove(i);
        } else {
            unique_positions.insert(planets[i].position);
            i += 1;
        }
    }
}

fn generate_planet_market(specialization: &PlanetSpecialization, economy: &Economy) -> Vec<Resource> {
    let mut market = Vec::new();
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
        let buy_price = buy_price.map(|p| (p * economy_multiplier).into());
        let sell_price = sell_price.map(|p| (p * economy_multiplier).into());

        // Generate random quantity if the planet trades this resource
        let quantity = if buy_price.is_some() || sell_price.is_some() {
            Some(rng.gen_range(10..100))
        } else {
            None
        };

        market.push(Resource {
            resource_type,
            buy: buy_price,
            sell: sell_price,
            quantity,
        });
    }

    market
}

pub trait PlanetTrait {
    type PlanetDanger;
    fn get_danger(&self) -> &Self::PlanetDanger;
}

// Implement the trait for the Planet structure
impl PlanetTrait for Planet {
    type PlanetDanger = PlanetDanger;

    fn get_danger(&self) -> &Self::PlanetDanger {
        // Return the danger level of the planet
        &self.danger
    }
}

impl Planet {
    pub fn new(name: String, position: Position, specialization: PlanetSpecialization, economy: Economy) -> Self {
        let biome: Biome = rand::random();
        let danger: PlanetDanger = rand::random();
        let market = Market::new(&specialization, &economy);
        
        Planet {
            name,
            description: format!("A {} planet with {} economy", biome, economy),
            position,
            economy,
            specialization,
            danger,
            biome,
            credits: 0.0,
            market,
        }
    }

    pub fn get_ship_market(&self, system_id: usize, planet_id: usize) -> std::io::Result<Vec<Ship>> {
        let settings = load_settings()?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("ships_{}_{}.json", system_id, planet_id));

        if market_path.exists() {
            let file = File::open(market_path)?;
            Ok(serde_json::from_reader(file)?)
        } else {
            // Generate new ship market if none exists
            let ships = self.generate_ship_market();
            // Save the generated market
            if let Some(parent) = market_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let market_json = serde_json::to_string(&ships)?;
            fs::write(&market_path, market_json)?;
            Ok(ships)
        }
    }

    pub fn generate_ship_market(&self) -> Vec<Ship> {
        let mut rng = rand::thread_rng();
        let ship_count = rng.gen_range(3..=8); // Generate 3-8 ships
        let mut ships = Vec::new();

        for _ in 0..ship_count {
            let ship_type = match self.specialization {
                PlanetSpecialization::Technology => ShipType::Battleship,
                PlanetSpecialization::Manufacturing => ShipType::Freighter,
                PlanetSpecialization::Mining => ShipType::Explorer,
                _ => {
                    // Random ship type for other specializations
                    let types = vec![ShipType::Fighter, ShipType::Freighter, ShipType::Explorer];
                    types[rng.gen_range(0..types.len())].clone()
                }
            };

            let mut ship: Ship = rand::random();
            ship.specialization = ship_type;
            
            // Set price based on ship size and planet economy
            let base_price = match ship.size {
                ShipSize::Tiny => 1000.0,
                ShipSize::Small => 2500.0,
                ShipSize::Medium => 5000.0,
                ShipSize::Large => 10000.0,
                _ => 20000.0,
            };

            let economy_multiplier = match self.economy {
                Economy::Booming => 1.5,
                Economy::Crashing => 0.8,
                _ => 1.0,
            };

            ship.price = Some(base_price * economy_multiplier);
            ships.push(ship);
        }

        ships
    }

    fn save_ship_market(&self, market: &[Ship], system_id: usize, planet_id: usize) -> std::io::Result<()> {
        let settings = load_settings()?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("ships_{}_{}.json", system_id, planet_id));

        if let Some(parent) = market_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let market_json = serde_json::to_string(market)?;
        fs::write(market_path, market_json)
    }

    pub fn refresh_ship_market(&mut self, system_id: usize, planet_id: usize) -> std::io::Result<()> {
        let ships = self.generate_ship_market();
        self.save_ship_market(&ships, system_id, planet_id)
    }

    pub fn buy_resource(&mut self, resource_type: ResourceType, quantity: u32, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        buy_from_planet(self, player, resource_type, quantity, system_id, planet_id)
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        sell_to_planet(self, player, resource_type, quantity, system_id, planet_id)
    }

    pub fn buy_ship(&mut self, ship_name: &str, fleet_name: &str, player: &mut Player, system_id: usize, planet_id: usize, trade_in_ship: Option<&str>) -> Result<(), String> {
        println!("Starting buy_ship operation:");
        println!("  Ship to buy: {}", ship_name);
        println!("  Fleet: {}", fleet_name);
        println!("  Trade-in ship: {:?}", trade_in_ship);

        let settings = load_settings().map_err(|e| e.to_string())?;
        let fleet_path = format!("data/game/{}/fleets/{}.json", settings.game_id, fleet_name);
        println!("Loading fleet from: {}", fleet_path);
        let mut fleet = Fleet::load(&fleet_path).map_err(|e| format!("Failed to load fleet: {}", e))?;
        println!("Loaded fleet with {} ships", fleet.ships.len());

        // Load the ship market
        println!("Loading ship market for system {} planet {}", system_id, planet_id);
        let mut ship_market = self.get_ship_market(system_id, planet_id).map_err(|e| format!("Failed to load ship market: {}", e))?;
        println!("Loaded market with {} ships", ship_market.len());

        // Find the ship in the market
        println!("Looking for ship {} in market", ship_name);
        let ship = ship_market.iter()
            .find(|s| s.name == ship_name)
            .ok_or_else(|| "Ship not found in market".to_string())?;
        println!("Found ship in market: {}", ship.name);

        // Get the ship price
        let ship_price = ship.price.ok_or_else(|| "Ship is not for sale".to_string())?;
        println!("Ship price: {}", ship_price);

        // Calculate trade-in value if applicable
        let (trade_in_value, trade_in_ship) = if let Some(trade_in_name) = trade_in_ship {
            println!("Processing trade-in for ship: {}", trade_in_name);
            // Find the trade-in ship in the fleet
            let trade_in_index = fleet.ships.iter()
                .position(|s| s.name == trade_in_name)
                .ok_or_else(|| "Trade-in ship not found in fleet".to_string())?;
            println!("Found trade-in ship at index {}", trade_in_index);

            // Get the trade-in ship and calculate its value
            let trade_in_ship = fleet.ships.remove(trade_in_index);
            println!("Removed trade-in ship from fleet. Fleet now has {} ships", fleet.ships.len());
            
            // Calculate trade-in value based on attributes
            let base_value = match trade_in_ship.size {
                ShipSize::Tiny => 500.0,
                ShipSize::Small => 1250.0,
                ShipSize::Medium => 2500.0,
                ShipSize::Large => 5000.0,
                ShipSize::Huge => 10000.0,
                ShipSize::Planetary => 25000.0,
            };

            let specialization_multiplier = match trade_in_ship.specialization {
                ShipType::Fighter => 1.1,
                ShipType::Battleship => 1.8,
                ShipType::Freighter => 1.3,
                ShipType::Explorer => 1.5,
                ShipType::Shuttle => 0.7,
                ShipType::Capital => 2.5,
            };

            let engine_multiplier = match trade_in_ship.engine {
                ShipEngine::Basic => 0.8,
                ShipEngine::Advanced => 1.2,
                ShipEngine::Experimental => 1.5,
            };

            let condition_multiplier = (trade_in_ship.hp as f32 / 100.0).max(0.5);

            let final_value = base_value * specialization_multiplier * engine_multiplier * condition_multiplier;
            println!("Calculated trade-in value: {} (base: {}, spec: {}, engine: {}, condition: {})", 
                final_value, base_value, specialization_multiplier, engine_multiplier, condition_multiplier);

            (final_value, Some(trade_in_ship))
        } else {
            println!("No trade-in ship specified");
            (0.0, None)
        };

        // Calculate final price after trade-in
        let final_price: f64 = ship_price as f64 - trade_in_value as f64;
        println!("Final price after trade-in: {} (original: {}, trade-in: {})", 
            final_price, ship_price, trade_in_value);

        // Check if player has enough credits
        if player.credits < final_price {
            println!("Insufficient credits. Need: {}, Have: {}", final_price, player.credits);
            return Err(format!("Insufficient credits. Need: {}, Have: {}", final_price, player.credits));
        }

        // Add the new ship to the fleet
        let mut new_ship = ship.clone();
        new_ship.owner = fleet_name.to_string();
        new_ship.price = None; // Clear price as it's now owned
        fleet.ships.push(new_ship);
        println!("Added new ship to fleet. Fleet now has {} ships", fleet.ships.len());

        // Update player credits
        player.credits -= final_price;
        println!("Updated player credits: {} (deducted {})", player.credits, final_price);

        // If there was a trade-in, add the traded ship to the market
        if let Some(mut market_ship) = trade_in_ship {
            println!("Adding trade-in ship to market");
            market_ship.price = Some(trade_in_value as f64);
            market_ship.owner = "".to_string(); // Clear owner as it's now in the market
            ship_market.push(market_ship);
            println!("Market now has {} ships", ship_market.len());
        }

        // Remove the purchased ship from the market
        println!("Removing purchased ship from market");
        ship_market.retain(|s| s.name != ship_name);
        println!("Market now has {} ships after removing purchased ship", ship_market.len());

        // Save the updated fleet
        println!("Saving updated fleet");
        fleet.save(&fleet_path).map_err(|e| format!("Failed to save fleet: {}", e))?;

        // Save the updated market
        println!("Saving updated market");
        self.save_ship_market(&ship_market, system_id, planet_id).map_err(|e| format!("Failed to save ship market: {}", e))?;

        println!("Buy ship operation completed successfully");
        Ok(())
    }

    pub fn save_market(&self, market: &[Resource]) -> std::io::Result<()> {
        let settings = load_settings()?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("{}_resources.json", self.name));

        if let Some(parent) = market_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let market_json = serde_json::to_string(market)?;
        fs::write(market_path, market_json)
    }

    pub fn sell_ship(&mut self, ship_name: &str, fleet_name: &str, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let fleet_path = format!("data/game/{}/fleets/{}.json", settings.game_id, fleet_name);
        // Load the player's fleet
        let mut fleet = Fleet::load(&fleet_path).map_err(|e| format!("Failed to load fleet: {}", e))?;

        // Find the ship in the fleet
        let ship_index = fleet.ships.iter()
            .position(|s| s.name == ship_name)
            .ok_or_else(|| "Ship not found in fleet".to_string())?;

        // Get the ship and calculate its value
        let ship = fleet.ships.remove(ship_index);
        
        // Calculate ship value based on attributes
        let base_value = match ship.size {
            ShipSize::Tiny => 500.0,
            ShipSize::Small => 1250.0,
            ShipSize::Medium => 2500.0,
            ShipSize::Large => 5000.0,
            ShipSize::Huge => 10000.0,
            ShipSize::Planetary => 25000.0,
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

        let ship_value = (base_value * specialization_multiplier * engine_multiplier * condition_multiplier) as f64;

        // Update player's credits
        player.credits += ship_value;

        // Save the updated fleet
        fleet.save(&fleet_path).map_err(|e| format!("Failed to save fleet: {}", e))?;

        // Add ship to market with calculated value
        let mut market_ships = self.get_ship_market(system_id, planet_id).map_err(|e| e.to_string())?;
        let mut market_ship = ship.clone();
        market_ship.price = Some(ship_value);
        market_ship.owner = "".to_string(); // Clear owner as it's now in the market
        market_ships.push(market_ship);
        
        // Save the updated market
        self.save_ship_market(&market_ships, system_id, planet_id).map_err(|e| e.to_string())?;

        Ok(())
    }

    // TODO: Implement planet colonization
    // TODO: Add planet development mechanics
    // TODO: Implement planet events system
    // TODO: Add planet diplomacy system
}

pub fn load_planet(system_id: usize, planet_id: usize) -> std::io::Result<Option<Planet>> {
    let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let planet_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("systems")
        .join(format!("System_{}", system_id))
        .join(format!("Planet_{}.json", planet_id));

    if !planet_path.exists() {
        return Ok(None);
    }

    let file = File::open(planet_path)?;
    let planet: Planet = serde_json::from_reader(file)?;
    Ok(Some(planet))
}

pub fn save_planet(system_id: usize, planet_id: usize, planet: &Planet) -> std::io::Result<()> {
    let planet_path = game_path(&["systems", &format!("System_{}", system_id), &format!("Planet_{}.json", planet_id)]);

    if let Some(parent) = planet_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(planet_path)?;
    serde_json::to_writer(file, planet)?;
    Ok(())
}

pub fn get_fleet_path(fleet_name: &str) -> String {
    let path = game_path(&["fleets", &format!("{}.json", fleet_name)]);
    path.to_string_lossy().into_owned()
}

pub fn load_planet_market(system_id: usize, planet_id: usize) -> Result<Market, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let market_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("markets")
        .join(format!("market_{}_{}.json", system_id, planet_id));

    if !market_path.exists() {
        return Err("Market not found".to_string());
    }

    let file = File::open(market_path).map_err(|e| e.to_string())?;
    let market: Market = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    Ok(market)
}

pub fn load_planet_ship_market(system_id: usize, planet_id: usize) -> Result<ShipMarket, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let market_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("markets")
        .join(format!("ships_{}_{}.json", system_id, planet_id));

    if !market_path.exists() {
        return Err("Ship market not found".to_string());
    }

    let file = File::open(market_path).map_err(|e| e.to_string())?;
    let market: ShipMarket = serde_json::from_reader(file).map_err(|e| e.to_string())?;
    Ok(market)
}