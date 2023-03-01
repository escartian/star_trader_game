use rand::Rng;
use serde::{Serialize, Deserialize};
use crate::models::planet::Planet;
use crate::models::planet::generate_planets;

use crate::constants::{MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH};

use super::{star::{generate_star, Star}, position::Position};
//use super::(planet);
#[derive(Serialize, Deserialize, Debug)]
pub struct StarSystem{
    star: Star,
    //this is used to maintain position of the StarSystem in the galaxy
    position: Position,
    planets: Vec<Planet>
}

pub fn generate_star_system() -> StarSystem{
    
    // Create a random number generator
    let mut rng = rand::thread_rng();
    let star = generate_star();
    let planets = generate_planets(rng.gen_range(3..10), MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH);
    let position = Position { 
        x: rng.gen_range(-MAP_WIDTH..=MAP_WIDTH as i32),
         y: rng.gen_range(-MAP_HEIGHT..=MAP_HEIGHT as i32), 
         z: rng.gen_range(-MAP_LENGTH..=MAP_LENGTH as i32) 
        };

    let star_system = StarSystem { star, position, planets };
    
    star_system
}