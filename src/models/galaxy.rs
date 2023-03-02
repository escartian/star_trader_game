use serde::{Deserialize, Serialize};

use crate::models::star_system::generate_star_system_default;

use super::star_system::StarSystem;
#[derive(Serialize, Deserialize, Debug)]
pub struct Galaxy{
    
    //this is used to maintain position of the StarSystems in the galaxy
    star_systems: Vec<StarSystem>,

    //planets: Vec<Planet>
}
pub fn generate_galaxy(stars: i32) -> Vec<StarSystem> {
    let mut systems = Vec::new();
    for _ in 0..stars {
            let star_system = generate_star_system_default();
            systems.push(star_system);
    }
    systems
}