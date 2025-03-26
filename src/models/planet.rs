use crate::constants::PRINT_DEBUG;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use strum::IntoEnumIterator;

use super::position::{random_nonzero_position, Position};
use super::resource::{Resource, ResourceType};
use super::player::Player;

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
    pub market: Vec<Resource>,
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
enum Biome {
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
#[derive(Serialize, Deserialize, Debug, Clone)]
enum PlanetSpecialization {
    Agriculture,
    Mining,
    Manufacturing,
    Technology,
    Research,
    Tourism,
    Service,
    None,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Economy {
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
        let market = generate_planet_market(&specialization, &economy);
        
        // Create a new planet with the generated name, coordinates, and other properties
        let planet = Planet {
            name,
            economy,
            specialization,
            danger,
            position,
            biome,
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
        let (buy_price, sell_price) = match specialization {
            PlanetSpecialization::Agriculture => match resource_type {
                ResourceType::Food => (Some(0.5), Some(1.5)),
                ResourceType::Water => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Mining => match resource_type {
                ResourceType::Minerals => (Some(0.5), Some(1.5)),
                ResourceType::Metals => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Manufacturing => match resource_type {
                ResourceType::Electronics => (Some(0.5), Some(1.5)),
                ResourceType::Metals => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Technology => match resource_type {
                ResourceType::Electronics => (Some(0.5), Some(1.5)),
                ResourceType::LuxuryGoods => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Research => match resource_type {
                ResourceType::Electronics => (Some(0.5), Some(1.5)),
                ResourceType::LuxuryGoods => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Tourism => match resource_type {
                ResourceType::LuxuryGoods => (Some(0.5), Some(1.5)),
                ResourceType::Food => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::Service => match resource_type {
                ResourceType::Food => (Some(0.5), Some(1.5)),
                ResourceType::Water => (Some(0.7), Some(1.3)),
                _ => (None, None),
            },
            PlanetSpecialization::None => (None, None),
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
    pub fn buy_resource(&mut self, resource_type: ResourceType, quantity: u32, player: &mut Player) -> Result<(), String> {
        // Find the resource in the planet's market
        if let Some(market_resource) = self.market.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(buy_price) = market_resource.buy_price() {
                let cost = quantity as f32 * buy_price;

                // Check if the player has enough credits
                if player.credits >= cost {
                    // Check if the planet has enough quantity
                    if let Some(available_quantity) = market_resource.quantity {
                        if available_quantity >= quantity {
                            // Update planet's market
                            market_resource.quantity = Some(available_quantity - quantity);
                            
                            // Update player's inventory and credits
                            player.credits -= cost;
                            player.add_resource(Resource::new(resource_type, quantity));

                            if PRINT_DEBUG {
                                println!(
                                    "Successfully bought {} {} from {} for {} credits",
                                    quantity, resource_type, self.name, cost
                                );
                            }
                            return Ok(());
                        }
                    }
                    return Err(format!("Planet does not have enough {} to sell", resource_type));
                }
                return Err(format!("Player does not have enough credits to buy {}", resource_type));
            }
            return Err(format!("Planet does not buy {}", resource_type));
        }
        Err(format!("Planet does not trade {}", resource_type))
    }

    pub fn sell_resource(&mut self, resource_type: ResourceType, quantity: u32, player: &mut Player) -> Result<(), String> {
        // Find the resource in the planet's market
        if let Some(market_resource) = self.market.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(sell_price) = market_resource.sell_price() {
                let earnings = quantity as f32 * sell_price;

                // Check if the player has enough of the resource
                if let Some(player_resource) = player.resources.iter_mut().find(|r| r.resource_type == resource_type) {
                    if let Some(player_quantity) = player_resource.quantity {
                        if player_quantity >= quantity {
                            // Update planet's market
                            if let Some(available_quantity) = market_resource.quantity {
                                market_resource.quantity = Some(available_quantity + quantity);
                            } else {
                                market_resource.quantity = Some(quantity);
                            }
                            
                            // Update player's inventory and credits
                            player.credits += earnings;
                            player.remove_resource(Resource::new(resource_type, quantity), quantity);

                            if PRINT_DEBUG {
                                println!(
                                    "Successfully sold {} {} to {} for {} credits",
                                    quantity, resource_type, self.name, earnings
                                );
                            }
                            return Ok(());
                        }
                    }
                    return Err(format!("Player does not have enough {} to sell", resource_type));
                }
                return Err(format!("Player does not have any {} to sell", resource_type));
            }
            return Err(format!("Planet does not buy {}", resource_type));
        }
        Err(format!("Planet does not trade {}", resource_type))
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
            market: Vec::new(),
        };
        let p2 = Planet {
            name: "B".to_string(),
            position: Position { x: 1, y: 2, z: 3 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            market: Vec::new(),
        };
        let p3 = Planet {
            name: "C".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            market: Vec::new(),
        };
        let p3 = Planet {
            name: "D".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
            market: Vec::new(),
        };

        // Add the planets to a vector
        let mut planets = vec![p1, p2, p3];

        // Ensure that there are no planets with the same position
        remove_colliding_planets(&mut planets);
        //there should only be 2 planets in the vactor
        assert_eq!(planets.len(), 2);
    }
}
