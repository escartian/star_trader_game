use super::{resource::{Resource, generate_resources}};
use serde::{Deserialize, Serialize};
use serde_json::{to_writer};
use crate::constants::INITIAL_CREDIT_COUNT;
use std::path::Path;
use crate::models::resource::generate_resources_no_trade;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub resources: Vec<Resource>,
    pub credits: f32,
}

impl Player {
    /// Creates a new Player with the given name, and default resources and credits.
    pub fn new(player_name: &str) -> Self {
        Player {
            name: player_name.to_string(),
            resources: generate_resources_no_trade(),
            credits: INITIAL_CREDIT_COUNT
        }
    }
}

pub fn create_player(game_id: &str, player_name: &str) -> Player {
    let mut player = Player::new(player_name);

    // Create the path to the player file
    let data_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("players")
        .join(player_name)
        .with_extension("json");

    // Create the necessary directories if they don't exist
    if let Some(parent) = data_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create directories");
    }

    // Create the file and handle any errors
    let file = match std::fs::File::create(&data_path) {
        Ok(file) => file,
        Err(e) => panic!("Failed to create player file: {}", e),
    };

    // Write the player data to the file
    match to_writer(&file, &player) {
        Ok(_) => println!("Successfully wrote player data to file"),
        Err(e) => panic!("Failed to write player data to file: {}", e),
    }

    player
}