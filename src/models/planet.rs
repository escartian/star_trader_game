use rand::distributions::{Distribution, Standard};
use rand::{Rng};
use serde::{Deserialize, Serialize};
//PLANET DETAILS
// Define a struct to represent a planet
#[derive(Serialize, Deserialize, Debug)]
pub struct Planet {
    name: String,
    x: i32,
    y: i32,
    z: i32,
    economy: Economy,
    specialization: PlanetSpecialization,
    danger: PlanetDanger,
    biome: Biome,
}

#[derive(Serialize, Deserialize, Debug)]
enum PlanetDanger {
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

#[derive(Serialize, Deserialize, Debug)]
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
#[derive(Serialize, Deserialize, Debug)]
enum PlanetSpecialization {
    Agriculture,
    Mining,
    Manufacturing,
    Technology,
    Tourism,
    Service,
    None,
}

#[derive(Serialize, Deserialize, Debug)]
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
        match rng.gen_range(0..5) {
            0 => PlanetSpecialization::Agriculture,
            1 => PlanetSpecialization::Mining,
            2 => PlanetSpecialization::Manufacturing,
            3 => PlanetSpecialization::Technology,
            4 => PlanetSpecialization::Tourism,
            5 => PlanetSpecialization::Service,
            _ => PlanetSpecialization::None,
        }
    }
}
pub fn generate_world_map(num_planets: u32, map_width: u32, map_height: u32) -> Vec<Planet> {
    // Initialize a vector to hold the planets
    let mut planets = Vec::new();

    // Create a random number generator
    let mut rng = rand::thread_rng();

    // Loop to generate the specified number of planets
    for i in 0..num_planets {
        // Generate a random name for the planet
        let name = format!("Planet {}", i + 1);

        // Generate random x and y coordinates for the planet
        let x = rng.gen_range(0..=map_width as i32);
        let y = rng.gen_range(0..=map_height as i32);
        let z = rng.gen_range(0..=map_height as i32);
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
            x,
            y,
            z,
            biome,
        };

        // Add the planet to the vector of planets
        planets.push(planet);
    }

    // Return the vector of planets
    planets
}