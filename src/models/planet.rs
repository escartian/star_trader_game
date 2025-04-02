use crate::constants::PRINT_DEBUG;
use crate::GAME_ID;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use strum::IntoEnumIterator;
use std::fs::{self, File};
use std::path::Path;

use super::position::{random_nonzero_position, Position};
use super::resource::{Resource, ResourceType};
use super::player::Player;
use super::market::Market;
use crate::models::ship::ship::{Ship, ShipEngine, ShipSize, ShipType};
use crate::models::fleet::Fleet;

//PLANET DETAILS
// Define a struct to represent a planet
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Planet {
    pub name: String,
    pub position: Position,
    pub economy: Economy,
    pub specialization: PlanetSpecialization,
    pub danger: PlanetDanger,
    pub biome: Biome,
    pub credits: f32,
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Economy {
    Booming,
    Growing,
    Stable,
    Struggling,
    Declining,
    Crashing,
    Nonexistent,
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

impl Distribution<Economy> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Economy {
        match rng.gen_range(0..6) {
            0 => Economy::Booming,
            1 => Economy::Growing,
            2 => Economy::Stable,
            3 => Economy::Struggling,
            4 => Economy::Declining,
            5 => Economy::Crashing,
            _ => Economy::Nonexistent,
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
    let mut planets = Vec::new();

    // Loop to generate the specified number of planets
    for i in 0..num_planets {
        // Generate a name for the planet
        let name = format!("Planet {}", i + 1);
        let position = random_nonzero_position(map_width, map_height, map_length);
        let economy: Economy = rand::random();
        let specialization: PlanetSpecialization = rand::random();
        let biome: Biome = rand::random();
        let danger: PlanetDanger = rand::random();
        
        // Create a new planet with the generated name, coordinates, and other properties
        let planet = Planet {
            name,
            economy,
            specialization,
            danger,
            position,
            biome,
            credits: 0.0,
        };

        // Add the planet to the vector of planets
        planets.push(planet);
    }

    remove_colliding_planets(&mut planets);
    // Return the vector of planets
    planets
}

fn remove_colliding_planets(planets: &mut Vec<Planet>) {
    let mut unique_positions: HashSet<&Position> = HashSet::new();
    let mut duplicates: Vec<usize> = vec![];

    for (i, planet) in planets.iter().enumerate() {
        if unique_positions.contains(&planet.position) {
            duplicates.push(i);
        } else {
            unique_positions.insert(&planet.position);
        }
    }

    // Remove the duplicates from the planets vector
    for i in duplicates.iter().rev() {
        if PRINT_DEBUG {
            println!("Planets generated in same location. Removing all but one");
        }
        planets.remove(*i);
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
        let buy_price = buy_price.map(|p| p * economy_multiplier);
        let sell_price = sell_price.map(|p| p * economy_multiplier);

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
    pub fn get_ship_market(&self, system_id: usize, planet_id: usize) -> std::io::Result<Vec<Ship>> {
        let market_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("markets")
            .join(format!("Star_System_{}_Planet_{}_ships.json", system_id, planet_id));

        if market_path.exists() {
            let contents = fs::read_to_string(market_path)?;
            Ok(serde_json::from_str(&contents)?)
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
        let market_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("markets")
            .join(format!("Star_System_{}_Planet_{}_ships.json", system_id, planet_id));

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
        let mut market = Market::load(system_id, planet_id).map_err(|e| format!("Failed to load market: {}", e))?;
        
        // Check if player has enough credits
        let cost = market.buy_resource(resource_type, quantity)?;
        if player.credits >= cost {
            // Update player's inventory and credits
            player.credits -= cost;
            player.add_resource(Resource::new(resource_type, quantity));

            if PRINT_DEBUG {
                println!(
                    "Successfully bought {} {} from {} for {} credits",
                    quantity, resource_type, self.name, cost
                );
            }
            Ok(())
        } else {
            Err(format!(
                "Insufficient credits to buy {} {}. Required: {:.2}, Available: {:.2}",
                quantity, resource_type, cost, player.credits
            ))
        }
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        let mut market = Market::load(system_id, planet_id).map_err(|e| format!("Failed to load market: {}", e))?;
        
        // Check if the player has enough of the resource
        if let Some(player_resource) = player.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(player_quantity) = player_resource.quantity {
                if player_quantity >= quantity {
                    // Update player's inventory and credits
                    let earnings = market.sell_resource(resource_type, quantity)?;
                    player.credits += earnings;
                    player.remove_resource(Resource::new(resource_type, quantity), quantity);

                    if PRINT_DEBUG {
                        println!(
                            "Successfully sold {} {} to {} for {} credits",
                            quantity, resource_type, self.name, earnings
                        );
                    }
                    Ok(())
                } else {
                    Err(format!(
                        "Not enough {} in your inventory. Requested: {}, Available: {}",
                        resource_type, quantity, player_quantity
                    ))
                }
            } else {
                Err(format!("You don't have any {} to sell", resource_type))
            }
        } else {
            Err(format!("You don't have any {} in your inventory", resource_type))
        }
    }

    pub fn buy_ship(&mut self, ship_name: &str, fleet_name: &str, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        // Load the fleet
        let fleet_path = format!("data/game/{}/fleets/{}.json", GAME_ID, fleet_name);
        let mut fleet = Fleet::load(&fleet_path).map_err(|e| format!("Failed to load fleet: {}", e))?;

        // Load the ship market
        let ship_market = self.get_ship_market(system_id, planet_id).map_err(|e| format!("Failed to load ship market: {}", e))?;

        // Find the ship in the market
        let ship = ship_market.iter()
            .find(|s| s.name == ship_name)
            .ok_or_else(|| "Ship not found in market".to_string())?;

        // Get the ship price
        let ship_price = ship.price.ok_or_else(|| "Ship is not for sale".to_string())?;

        // Check if player has enough credits
        if player.credits < ship_price {
            return Err(format!("Insufficient credits. Need: {}, Have: {}", ship_price, player.credits));
        }

        // Add the new ship to the fleet
        let mut new_ship = ship.clone();
        new_ship.owner = fleet_name.to_string();
        new_ship.price = None; // Clear price as it's now owned
        fleet.ships.push(new_ship);

        // Update player credits
        player.credits -= ship_price;

        // Save the updated fleet
        fleet.save(&fleet_path).map_err(|e| format!("Failed to save fleet: {}", e))?;

        // Update and save the ship market
        let mut updated_market = ship_market;
        updated_market.retain(|s| s.name != ship_name);
        self.save_ship_market(&updated_market, system_id, planet_id).map_err(|e| format!("Failed to save ship market: {}", e))?;

        Ok(())
    }

    pub fn save_market(&self, market: &[Resource]) -> std::io::Result<()> {
        let market_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("markets")
            .join(format!("{}_resources.json", self.name));

        if let Some(parent) = market_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let market_json = serde_json::to_string(market)?;
        fs::write(market_path, market_json)
    }

    pub fn sell_ship(&mut self, ship_name: &str, fleet_name: &str, player: &mut Player, system_id: usize, planet_id: usize) -> Result<(), String> {
        // Load the player's fleet
        let fleet_path = format!("data/game/{}/fleets/{}.json", GAME_ID, fleet_name);
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

        let condition_multiplier = (ship.hp as f32 / 100.0).max(0.5);

        let ship_value = base_value * specialization_multiplier * engine_multiplier * condition_multiplier;

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
}

#[cfg(test)]
mod planet_tests {
    use crate::models::{planet::{Planet, remove_colliding_planets}, position::Position};

    #[test]
    fn test_remove_colliding_planets() {
        // Create some planets with the same position
        let p1 = Planet {
            name: "A".to_string(),
            position: Position { x: 1, y: 2, z: 3 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            credits: 0.0,
        };
        let p2 = Planet {
            name: "B".to_string(),
            position: Position { x: 1, y: 2, z: 3 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            credits: 0.0,
        };
        let p3 = Planet {
            name: "C".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            credits: 0.0,
        };
        let p3 = Planet {
            name: "D".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            credits: 0.0,
        };

        // Add the planets to a vector
        let mut planets = vec![p1, p2, p3];

        // Ensure that there are no planets with the same position
        remove_colliding_planets(&mut planets);
        //there should only be 2 planets in the vactor
        assert_eq!(planets.len(), 2);
    }
}
