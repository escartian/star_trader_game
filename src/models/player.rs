use serde::Serialize;
use serde::Deserialize;
use serde_json::to_writer;
use crate::constants::INITIAL_CREDIT_COUNT;
use std::path::Path;
use crate::models::resource::generate_resources_no_trade;
use super::resource::Resource;

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
            credits: INITIAL_CREDIT_COUNT,
            //faction: Factom::new(),
            //fleets: vec![],
            //race: Race::new()
            //faction_standing : vec![]

        }
    }

    pub fn add_resource(&mut self, resource: Resource) {
        if let Some(existing_resource) = self.resources.iter_mut().find(|r| r.resource_type == resource.resource_type) {
            // If the resource exists, add to its quantity
            if let Some(existing_quantity) = existing_resource.quantity {
                if let Some(new_quantity) = resource.quantity {
                    existing_resource.quantity = Some(existing_quantity + new_quantity);
                }
            } else {
                existing_resource.quantity = resource.quantity;
            }
        } else {
            // If the resource doesn't exist, add it as new
            self.resources.push(resource);
        }
    }

    pub fn remove_resource(&mut self, resource: Resource, quantity: u32) {
        if quantity == 0 {
            return;
        }
        
        let resource_type = resource.resource_type;
        
        if let Some(existing_resource) = self.resources.iter_mut().find(|r| r.resource_type == resource_type) {
            if let Some(existing_quantity) = existing_resource.quantity {
                if existing_quantity >= quantity {
                    existing_resource.quantity = Some(existing_quantity - quantity);
                }
            }
        }
    }

    /** Creates a new player with the specified name and saves their data to a JSON file 
    * within the game directory. If necessary, creates the required directories.
    * Returns the newly created Player object.
    **/
    pub fn create_player(game_id: &str, player_name: &str) -> Player {
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

        // Create a new player
        let player = Player::new(player_name);

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
}