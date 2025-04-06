use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::models::planet::Planet;
use crate::models::planet::generate_planets;

use super::position::random_position;
use super::{star::{generate_star, Star}, position::Position};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StarSystem {
    pub star: Star,
    //this is used to maintain position of the StarSystem in the galaxy
    pub position: Position,
    pub planets: Vec<Planet>,
    pub radius: f64, // Add radius field
}

impl StarSystem {
    // Calculate the system's radius based on its properties
    pub fn calculate_radius(&self) -> f64 {
        // Base radius depends on the number of planets and their distances
        let mut max_planet_distance: f64 = 0.0;
        for planet in &self.planets {
            let dx = (planet.position.x - self.position.x) as f64;
            let dy = (planet.position.y - self.position.y) as f64;
            let dz = (planet.position.z - self.position.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            max_planet_distance = max_planet_distance.max(distance);
        }
        
        // System radius is the maximum planet distance plus a buffer
        // The buffer is larger for systems with more planets
        let planet_count_factor = (self.planets.len() as f64).sqrt();
        let buffer = 20.0 * planet_count_factor;
        
        max_planet_distance + buffer
    }
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
pub fn generate_star_system_default() -> StarSystem {
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let mut existing_names: std::collections::HashSet<String> = std::collections::HashSet::new();
    let star = generate_star(1000, 1000, 1000, &existing_names); // Default to 1000x1000x1000 if no dimensions provided
    existing_names.insert(star.name.clone());
    let planets = generate_planets(rng.gen_range(3..10), 1000, 1000, 1000); // Default to 1000x1000x1000 if no dimensions provided
    let position = random_position(1000, 1000, 1000); // Default to 1000x1000x1000 if no dimensions provided

    let mut star_system = StarSystem { 
        star, 
        position, 
        planets,
        radius: 0.0, // Will be calculated below
    };
    
    // Calculate and set the radius
    star_system.radius = star_system.calculate_radius();
    
    star_system
}

/// Generates a star system with random properties at the specified position
/// 
/// # Arguments
/// * `map_width` - Width of the galaxy map
/// * `map_height` - Height of the galaxy map
/// * `map_length` - Length of the galaxy map
/// * `existing_names` - A mutable reference to a set of existing names
/// 
/// # Returns
/// A new StarSystem with random properties
pub fn generate_star_system(map_width: i32, map_height: i32, map_length: i32, existing_names: &mut std::collections::HashSet<String>) -> StarSystem {
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let planet_count = rng.gen_range(3..10);
    let star = generate_star(map_width, map_height, map_length, existing_names);
    existing_names.insert(star.name.clone());
    let planets = generate_planets(planet_count, map_width, map_height, map_length);
    let position = random_position(map_width, map_height, map_length);

    let mut star_system = StarSystem { 
        star, 
        position, 
        planets,
        radius: 0.0, // Will be calculated below
    };
    
    // Calculate and set the radius
    star_system.radius = star_system.calculate_radius();
    
    star_system
}