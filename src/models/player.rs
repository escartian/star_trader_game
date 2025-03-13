use serde::Serialize;
use serde::Deserialize;
use serde_json::to_writer;


use crate::constants::INITIAL_CREDIT_COUNT;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use crate::models::resource::generate_resources_no_trade;
use crate::Trader;
use crate::models::trade::TradeAction;
use crate::models::trade::TradeResult;
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
        self.resources.push(resource);
    }

    pub fn remove_resource(&mut self, resource: Resource, quantity: u32) {
        if quantity == 0 {
            return;
        }
        
        let resource_type = resource.resource_type;
        
        self.resources.retain_mut(|r| {
            if r.resource_type == resource_type {
                if let Some(q) = r.quantity.clone() {
                    if q >= quantity {
                        return false; // Don't remove this resource
                    }
                    r.quantity = Some(q.saturating_sub(quantity));
                }
            }
            true // Keep this resource
        });
    }

    pub fn trade_with_trader(&mut self, trader: &mut Trader, action: TradeAction) -> TradeResult {
    // Check if the trader reference is valid
        if trader.is_null() {
            return TradeResult::TraderNotFound;
        }
        else {    
            match action {
                TradeAction::Buy { resource_type, quantity } => {
                    // Check for invalid quantity
                    if quantity == 0 {
                        return TradeResult::InvalidResource;
                    }
                    // Attempt to buy resource and handle potential errors
                    match trader.buy_resource(resource_type, quantity, self) {
                        Ok(_) => TradeResult::Success,
                        Err(e) => {
                            eprintln!("Error during buying resource: {}", e);
                            TradeResult::TransactionFailed
                        }
                    }
                }
                TradeAction::Sell { resource_type, quantity } => {
                    // Check for invalid quantity
                    if quantity == 0 {
                        return TradeResult::InvalidResource;
                    }
                    // Attempt to sell resource and handle potential errors
                    match trader.sell_resource(resource_type, quantity, self) {
                        Ok(_) => TradeResult::Success,
                        Err(e) => {
                            eprintln!("Error during selling resource: {}", e);
                            TradeResult::TransactionFailed
                        }
                    }
                }
            }
        }    
    }

    /** Creates a new player with the specified name and saves their data to a JSON file 
    * within the game directory. If necessary, creates the required directories.
    * Returns the newly created Player object.
    **/
    pub fn create_player(game_id: &str, player_name: &str) -> Player {
        let data_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("players")
        .join(player_name)
        .with_extension("json");
        if std::fs::metadata(&data_path).is_ok() {
            let mut file = File::open(data_path);
            let mut contents = String::new();
            file.expect("REASON").read_to_string(&mut contents);

            let player = serde_json::from_str(&contents).expect("Failed to parse player data");
            return player;
        }

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
}