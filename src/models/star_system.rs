use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::models::planet::Planet;
use crate::models::planet::generate_planets;

use super::position::random_position;
use super::{star::{generate_star, Star}, position::Position};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StarSystem{
    star: Star,
    //this is used to maintain position of the StarSystem in the galaxy
    pub position: Position,
    pub planets: Vec<Planet>
}

impl Default for StarSystem {
    fn default() -> Self {
        generate_star_system_default()
    }
}

/// Generates a star system with random properties at the specified position
/// 
/// # Arguments
/// * `map_width` - Width of the galaxy map
/// * `map_height` - Height of the galaxy map
/// * `map_length` - Length of the galaxy map
/// 
/// # Returns
/// A new StarSystem with random properties
pub fn generate_star_system_default() -> StarSystem{
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let star = generate_star(1000, 1000, 1000); // Default to 1000x1000x1000 if no dimensions provided
    let planets = generate_planets(rng.gen_range(3..10), 1000, 1000, 1000); // Default to 1000x1000x1000 if no dimensions provided
    let position = random_position(1000, 1000, 1000); // Default to 1000x1000x1000 if no dimensions provided

    let star_system = StarSystem { star, position, planets };
    
    star_system
}

/// Generates a star system with random properties at the specified position
/// 
/// # Arguments
/// * `map_width` - Width of the galaxy map
/// * `map_height` - Height of the galaxy map
/// * `map_length` - Length of the galaxy map
/// 
/// # Returns
/// A new StarSystem with random properties
pub fn generate_star_system(map_width: i32, map_height: i32, map_length: i32) -> StarSystem {
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let planet_count = rng.gen_range(3..10);
    let star = generate_star(map_width, map_height, map_length);
    let planets = generate_planets(planet_count, map_width, map_height, map_length);
    let position = random_position(map_width, map_height, map_length);

    StarSystem { star, position, planets }
}