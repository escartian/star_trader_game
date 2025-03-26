use crate::models::planet::Planet;
use crate::models::ship::ship::Ship;
use crate::models::resource::Resource;
use crate::models::galaxy::generate_galaxy;
use crate::constants::STAR_COUNT;
use std::fs;
use std::path::Path;
use std::fs::File;
use serde_json;
use serde::{Deserialize, Serialize};
use std::io::Read;
use serde_json::to_writer;
use crate::models::star_system::StarSystem;

pub struct GameWorld {
    planets: Vec<Planet>,
    ships: Vec<Ship>,
}

pub struct PlayerState {
    player_name: String,
    ship: Vec<Ship>,
    inventory: Vec<Resource>,
    credits: u32,
}

impl GameWorld {
    pub fn new() -> GameWorld {
        GameWorld {
            planets: Vec::new(),
            ships: Vec::new(),
        }
    }

    pub fn update(&mut self, player_state: &PlayerState) {
        /* ... */
    }
}

/// Creates a new game world file in the specified game directory and writes the generated
/// galaxy map to it. The file is stored in the "data/game/<game_id>/GameWorld.json" directory.
/// If the necessary directories do not exist, this function will create them.
///
/// # Arguments
/// - `game_id` - A string slice that holds the identifier for the game instance.
///
/// # Errors
/// This function will panic if it is unable to create the file or write the galaxy map to it.
pub fn create_game_world_file(game_id: &str, empty_world: bool) -> Vec<StarSystem> {
    // WORLD GENERATION
    // Generate the galaxy map
    println!("Creating Game World File");
    let galactic_map;
    if empty_world {   
        galactic_map = generate_galaxy(STAR_COUNT);
        let data_path = Path::new("data")
            .join("game")
            .join(game_id)
            .join("GameWorld.json");

        // Create the necessary directories if they don't exist
        if let Some(parent) = data_path.parent() {
            println!("{}", parent.display());
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
        
        // Create the file and handle any errors
        let file = match File::create(&data_path) {
            Ok(file) => file,
            Err(e) => panic!("Failed to create file: {}", e),
        };

        println!("Game world file created at: {:?}", data_path);

        // Write the galaxy map to the file
        match to_writer(&file, &galactic_map) {
            Ok(_) => println!("Successfully wrote galaxy map to file"),
            Err(e) => panic!("Failed to write galaxy map to file: {}", e),
        }
        return galactic_map;
    } else {
        println!("Game World Already Exists");
        let data_path = Path::new("data")
            .join("game")
            .join(game_id)
            .join("GameWorld.json");
        let file = File::open(data_path);
        let mut contents = String::new();
        file.expect("REASON").read_to_string(&mut contents);
        galactic_map = serde_json::from_str(&contents).unwrap();

        return galactic_map;
    }
    panic!("World failed to generate!");
    return generate_galaxy(1);
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            player_name: String::new(),
            ship: Vec::new(),
            inventory: Vec::new(),
            credits: 0,
        }
    }
}

