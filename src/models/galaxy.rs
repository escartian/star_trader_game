use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::models::star_system::{generate_star_system, generate_star_system_default};
use crate::models::position::{Position, random_nonzero_position};
use super::star_system::StarSystem;

#[derive(Serialize, Deserialize, Debug)]
pub struct Galaxy{
    
    //this is used to maintain position of the StarSystems in the galaxy
    star_systems: Vec<StarSystem>,

    //planets: Vec<Planet>
}

/// Generates a galaxy with the specified dimensions and number of stars
/// 
/// # Arguments
/// * `map_width` - Width of the galaxy map
/// * `map_height` - Height of the galaxy map
/// * `map_length` - Length of the galaxy map
/// * `star_count` - Number of star systems to generate
/// 
/// # Returns
/// A vector of star systems positioned within the specified dimensions
pub fn generate_galaxy(
    map_width: i32,
    map_height: i32,
    map_length: i32,
    star_count: i32,
) -> Result<Vec<StarSystem>, String> {
    println!("Starting generate_galaxy with dimensions: {}x{}x{}, stars: {}", 
        map_width, map_height, map_length, star_count);
    
    let mut star_systems = Vec::with_capacity(star_count as usize);
    let mut unique_positions: HashSet<Position> = HashSet::with_capacity(star_count as usize);
    let mut attempts = 0;
    let max_attempts = star_count * 2;

    while star_systems.len() < star_count as usize && attempts < max_attempts {
        println!("Generating star system {}/{}", star_systems.len() + 1, star_count);
        let position = random_nonzero_position(map_width, map_height, map_length);
        
        if !unique_positions.contains(&position) {
            println!("Found unique position for star system: {:?}", position);
            unique_positions.insert(position.clone());
            let system = generate_star_system(map_width, map_height, map_length);
            println!("Successfully generated star system at position {:?}", position);
            star_systems.push(system);
        } else {
            println!("Position {:?} already taken, trying again", position);
        }
        attempts += 1;
    }

    if star_systems.len() < star_count as usize {
        println!("Warning: Could only generate {} out of {} requested star systems", 
            star_systems.len(), star_count);
    }

    println!("Galaxy generation completed with {} star systems", star_systems.len());
    Ok(star_systems)
}