mod constants;
mod models;
mod routes;
mod engine;

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
mod combat;
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
                for _ in 0..fleet_count {
                    if let Ok(fleet) = generate_and_save_fleet(
                        faction.name.clone(),
                        random_position(MAP_WIDTH, MAP_HEIGHT, MAP_LENGTH),
                        rand::random::<usize>() % 5 + 1, // Random number of ships (1-5)
                    ) {
                        println!("Generated fleet for faction {}: {}", faction.name, fleet.name);
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
    let game_generated = false;
    /***On game launch create the game. ***/
    let gameworld = get_global_game_world();
    
    // Check if player exists, if not create them
    let player_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("players")
        .join(format!("{}.json", HOST_PLAYER_NAME));
    
    if !player_path.exists() {
        println!("Creating new player: {}", HOST_PLAYER_NAME);
        let player = Player::create_player(GAME_ID, HOST_PLAYER_NAME);
        println!("Player created with {} credits", player.credits);
    } else {
        println!("Loading existing player: {}", HOST_PLAYER_NAME);
    }
    
    let player = get_player(HOST_PLAYER_NAME);

    // Generate some factions
    let factions = vec![
        ("The Galactic Empire", "A powerful military dictatorship"),
        ("The Rebel Alliance", "Freedom fighters against tyranny"),
        ("The Trade Federation", "Wealthy merchants and traders"),
    ];

    if !game_generated {
        create_player_fleet();
        create_faction_fleets(&factions);
    }
    println!("Current working directory: {:?}", env::current_dir().unwrap());
    let template_dir = Path::new("src").join("templates");
    println!("Template directory: {:?}", template_dir);
    rocket::build()
        .mount("/", routes![
            index, 
            get_player, 
            get_galaxy_map, 
            get_star_system, 
            get_fleet, 
            get_owner_fleets,
            get_planet_market,
            buy_from_planet,
            sell_to_planet
        ])
        .attach(Template::fairing())
        .register("/", catchers![internal_error])
        .launch()
        .await
        .unwrap();
}
