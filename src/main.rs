mod constants;
mod models;
mod routes;
mod engine;
mod combat;
mod encounters;

use rocket::{get, routes, Request, Response};
use rocket_dyn_templates::{Template, tera::Tera, context};
use std::sync::atomic::Ordering;

//use serde ::{Deserialize, Serialize};
//use serde_json::{to_writer, Result};

use rocket::catchers;
use serde_json::to_writer;

use std::fs;
use std::env;

use std::fs::File;
use std::path::Path;
use std::string::String;

use crate::routes::*;

//use crate::combat::combat::{auto_resolve_ship_combat, CombatResult};
use crate::models::player::Player;
//use crate::models::ship::ship::Ship;
use crate::models::star_system::StarSystem;
use crate::models::trader::Trader;
use lazy_static::lazy_static;
use std::io::Read;
use std::sync::Mutex;

use crate::constants::HOST_PLAYER_NAME;
use crate::constants::GAME_ID;
use crate::constants::STAR_COUNT;
use crate::models::game_world;
use crate::models::game_world::create_game_world_file;
use crate::models::faction::{Faction, save_faction};
use crate::models::fleet::generate_and_save_fleet;
use crate::models::position::Position;
use crate::models::position::random_position;
use crate::constants::{MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH, GAME_GENERATED};

use rocket_cors::{AllowedOrigins, CorsOptions, AllowedHeaders};
use rocket::fs::FileServer;

lazy_static! {
    static ref GLOBAL_GAME_WORLD: Mutex<Vec<StarSystem>> = {
        println!("Loading Game World");
        let data_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("GameWorld.json");

        println!("{}", data_path.display());
        if data_path.exists() && data_path.is_file() {
            let file = File::open(data_path);
            let mut contents = String::new();
            file.expect("REASON").read_to_string(&mut contents);
            Mutex::new(serde_json::from_str(&contents).unwrap())
        } else {
            println!("Game world is empty");
            Mutex::new(create_game_world_file(GAME_ID, true))
        }
    };
}

pub(crate) fn get_global_game_world() -> Vec<StarSystem> {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
}

fn create_player_fleet() {
    if !GAME_GENERATED.load(Ordering::Relaxed) {
        // Check if player fleet already exists
        let fleets = crate::models::fleet::list_owner_fleets(HOST_PLAYER_NAME).unwrap_or_default();
        if fleets.is_empty() {
            if let Ok(fleet) = generate_and_save_fleet(
                HOST_PLAYER_NAME.to_string(),
                random_position(MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH),
                1, // Start with 1 ship
            ) {
                println!("Generated player fleet: {}", fleet.name);
            }
        } else {
            println!("Player fleet already exists");
        }
    }
}

fn create_faction_fleets(factions: &[(&str, &str)]) {
    if !GAME_GENERATED.load(Ordering::Relaxed) {
        for (name, desc) in factions {
            // Check if faction already exists
            if let Ok(Some(_)) = crate::models::faction::load_faction(name) {
                println!("Faction {} already exists", name);
                continue;
            }

            let faction = Faction::new(name.to_string(), desc.to_string());
            if let Ok(_) = save_faction(&faction) {
                println!("Created faction: {}", faction.name);
                
                // Generate 2-3 fleets for each faction in different positions
                let fleet_count = rand::random::<usize>() % 2 + 2; // Random number between 2-3
                for fleet_num in 0..fleet_count {
                    if let Ok(fleet) = generate_and_save_fleet(
                        name.to_string(), // Use faction name directly as owner ID
                        random_position(MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH),
                        rand::random::<usize>() % 5 + 1, // Random number of ships (1-5)
                    ) {
                        println!("Generated fleet for faction {}: {}", name, fleet.name);
                    }
                }
            }
        }
    }
}

fn create_special_fleets() {
    if !GAME_GENERATED.load(Ordering::Relaxed) {
        let special_types = vec![
            ("Pirate", 3),    // 3 pirate fleets
            ("Trader", 2),    // 2 trader fleets
            ("Military", 2),  // 2 military fleets
            ("Mercenary", 2), // 2 mercenary fleets
        ];

        for (fleet_type, count) in special_types {
            // Check if any fleets of this type already exist by looking for files starting with Fleet_Pirate_, Fleet_Trader_, etc.
            let fleets_dir = Path::new("data")
                .join("game")
                .join(GAME_ID)
                .join("fleets");

            let mut has_existing_fleets = false;
            if fleets_dir.exists() {
                if let Ok(entries) = fs::read_dir(fleets_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            if let Some(file_name) = entry.file_name().to_str() {
                                if file_name.starts_with(&format!("Fleet_{}_", fleet_type)) {
                                    has_existing_fleets = true;
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if has_existing_fleets {
                println!("{} fleets already exist, skipping generation", fleet_type);
                continue;
            }

            for fleet_num in 0..count {
                // Generate a captain name for the fleet
                let captain_name = crate::models::ship::ship::generate_owner_name();
                if let Ok(fleet) = generate_and_save_fleet(
                    fleet_type.to_string(), // Use fleet type for the fleet name
                    random_position(MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH),
                    rand::random::<usize>() % 5 + 1, // Random number of ships (1-5)
                ) {
                    // Update the fleet's owner_id to be the captain name
                    let mut fleet = fleet;
                    fleet.owner_id = captain_name.clone();
                    if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                        println!("Error saving fleet: {}", e);
                    } else {
                        println!("Generated {} fleet: {} (Captain: {})", fleet_type, fleet.name, captain_name);
                    }
                }
            }
        }
    }
}

/// The main entry point for the Rocket application.
///
/// This function is responsible for launching the Rocket server and mounting
/// the routes.
#[rocket::main]
async fn main() {
    /***On game launch create the game. ***/
    let gameworld = get_global_game_world();
    
    // Check if player exists, if not create them
    let player_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("players")
        .join(format!("{}.json", HOST_PLAYER_NAME));
    
    // Create players directory if it doesn't exist
    if let Some(parent) = player_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create players directory");
    }
    
    if !player_path.exists() {
        println!("Creating new player: {}", HOST_PLAYER_NAME);
        let player = Player::create_player(GAME_ID, HOST_PLAYER_NAME);
        println!("Player created with {} credits", player.credits);
        
        // Save the player file
        let file = File::create(&player_path).expect("Failed to create player file");
        serde_json::to_writer(file, &player).expect("Failed to write player data");
    } else {
        println!("Loading existing player: {}", HOST_PLAYER_NAME);
    }

    // Generate some factions
    let factions = vec![
        ("The Galactic Empire", "A powerful military dictatorship"),
        ("The Rebel Alliance", "Freedom fighters against tyranny"),
        ("The Trade Federation", "Wealthy merchants and traders"),
    ];

    // Only generate fleets if the game hasn't been generated yet
    if !GAME_GENERATED.load(Ordering::Relaxed) {
        println!("Generating initial game state...");
        create_player_fleet();
        create_faction_fleets(&factions);
        create_special_fleets();
        GAME_GENERATED.store(true, Ordering::Relaxed);
    } else {
        println!("Game already generated, skipping fleet generation");
    }

    // Configure CORS
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec!["Get", "Post", "Put", "Delete", "Options"]
                .into_iter()
                .map(|s| s.parse().unwrap())
                .collect(),
        )
        .allowed_headers(AllowedHeaders::all())
        .allow_credentials(true)
        .to_cors()
        .expect("Failed to create CORS fairing");

    println!("Current working directory: {:?}", env::current_dir().unwrap());
    
    rocket::build()
        .mount("/", FileServer::from("frontend/build"))
        .mount("/api", routes![
            get_player, 
            get_galaxy_map, 
            get_star_system, 
            get_fleet, 
            get_owner_fleets,
            get_planet_market,
            buy_from_planet,
            sell_to_planet,
            move_fleet,
            get_fleet_owners,
            initiate_combat,
            check_for_encounter,
            trade_with_trader
        ])
        .attach(cors)
        .register("/", catchers![internal_error])
        .configure(rocket::Config::figment()
            .merge(("address", "0.0.0.0"))
            .merge(("port", 8000)))
        .launch()
        .await
        .unwrap();
}
