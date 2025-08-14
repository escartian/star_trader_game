use serde::Serialize;
use serde::Deserialize;
use serde_json::to_writer;
use crate::models::resource::{Resource, ResourceType, generate_resources_no_trade};
use crate::models::game_state::PLAYER_CACHE;
use crate::models::game_state::game_path;
use std::path::Path;
use crate::models::settings::load_settings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub resources: Vec<Resource>,
    pub credits: f64,
    pub fleets: Vec<String>, // Store fleet names
}

impl Player {
    /// Creates a new Player with the given name, and default resources and credits.
    /// 
    /// # Arguments
    /// * `player_name` - The name of the player
    /// * `starting_credits` - The amount of credits the player starts with
    /// 
    /// # Returns
    /// A new Player instance with the specified name and starting credits
    pub fn new(player_name: &str, starting_credits: f64) -> Self {
        Player {
            name: player_name.to_string(),
            resources: generate_resources_no_trade(),
            credits: starting_credits,
            fleets: vec![format!("Fleet_{}_{}", player_name, 1)], // Initialize with first fleet
        }
    }

    pub fn add_resource(&mut self, resource_type: ResourceType, quantity: u32) {
        if let Some(existing_resource) = self.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            // If the resource exists, add to its quantity
            existing_resource.quantity = Some(existing_resource.quantity.unwrap_or(0) + quantity);
        } else {
            // If the resource doesn't exist, add it as new
            self.resources.push(Resource::new(resource_type, quantity));
        }
    }

    pub fn remove_resource(&mut self, resource_type: ResourceType, quantity: u32) -> bool {
        if let Some(existing_resource) = self.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(existing_quantity) = existing_resource.quantity {
                if existing_quantity >= quantity {
                    existing_resource.quantity = Some(existing_quantity - quantity);
                    return true;
                }
            }
        }
        false
    }

    pub fn has_resource(&self, resource_type: ResourceType, quantity: u32) -> bool {
        if let Some(resource) = self.resources.iter().find(|r| r.resource_type == resource_type) {
            if let Some(existing_quantity) = resource.quantity {
                return existing_quantity >= quantity;
            }
        }
        false
    }

    /** Creates a new player with the specified name and saves their data to a JSON file 
    * within the game directory. If necessary, creates the required directories.
    * Returns the newly created Player object.
    **/
    pub fn create_player(game_id: &str, player_name: &str, starting_credits: f64) -> Player {
        // Create the path to the player file
        let data_path = Path::new("data")
            .join("game")
            .join(game_id)
            .join("players")
            .join(format!("{}.json", player_name));

        // Create a new player
        let player = Player::new(player_name, starting_credits);

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

    /// Saves the player's data to a JSON file.
    /// 
    /// # Returns
    /// A Result indicating whether the save was successful
    pub fn save(&self) -> Result<(), String> {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("players")
            .join(format!("{}.json", self.name));
        let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
        serde_json::to_writer(&file, self).map_err(|e| e.to_string())?;
        PLAYER_CACHE.set(self.name.clone(), self.clone());
        Ok(())
    }
}