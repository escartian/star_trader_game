use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::models::planet::Planet;
use crate::models::planet::generate_planets;

use crate::constants::{MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH};

use super::position::random_position;
use super::{star::{generate_star, Star}, position::Position};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StarSystem{
    star: Star,
    //this is used to maintain position of the StarSystem in the galaxy
    pub position: Position,
    pub planets: Vec<Planet>
}

pub fn generate_star_system_default() -> StarSystem{
    
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let star = generate_star();
    let planets = generate_planets(rng.gen_range(3..10), MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH);
    let position = random_position(MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH);

    let star_system = StarSystem { star, position, planets };
    
    star_system
}