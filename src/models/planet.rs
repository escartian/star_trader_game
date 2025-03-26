use crate::constants::PRINT_DEBUG;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;

use super::position::{random_nonzero_position, Position};

//PLANET DETAILS
// Define a struct to represent a planet
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Planet {
    name: String,
    pub position: Position,
    economy: Economy,
    specialization: PlanetSpecialization,
    danger: PlanetDanger,
    biome: Biome,
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
        // Create a new planet with the generated name, coordinates, and other properties
        let planet = Planet {
            name,
            economy,
            specialization,
            danger,
            position,
            biome,
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
        };
        let p2 = Planet {
            name: "B".to_string(),
            position: Position { x: 1, y: 2, z: 3 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
        };
        let p3 = Planet {
            name: "C".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
        };
        let p3 = Planet {
            name: "D".to_string(),
            position: Position { x: 4, y: 5, z: 6 },
            economy: rand::random(),
            specialization: rand::random(),
            danger: rand::random(),
            biome: rand::random(),
        };

        // Add the planets to a vector
        let mut planets = vec![p1, p2, p3];

        // Ensure that there are no planets with the same position
        remove_colliding_planets(&mut planets);
        //there should only be 2 planets in the vactor
        assert_eq!(planets.len(), 2);
    }
}
