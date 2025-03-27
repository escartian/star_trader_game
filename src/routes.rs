use crate::get_global_game_world;
use crate::GAME_ID;
use crate::GLOBAL_GAME_WORLD;
use std::fs::File;
use std::path::Path;
use rocket_dyn_templates::Template;
use crate::HOST_PLAYER_NAME;
use std::collections::HashMap;
use rocket::Request;
use rocket::get;
use std::io::Read;
use crate::models::star_system::StarSystem;
use rocket::catch;
use rocket::serde::json::Json;
use crate::models::fleet::Fleet;
use crate::models::resource::{Resource, ResourceType};
use crate::models::player::Player;
use rand;
use serde_json;
use std::sync::Mutex;
use std::sync::LockResult;
use std::sync::MutexGuard;
use crate::models::position::Position;
use std::fs;


#[catch(500)]
pub fn internal_error(_req: &Request) -> Template {
    Template::render("500", ())
}

/// Handles the root route (`/`) and renders the index page
#[get("/")]
pub fn index() -> Template {
    let mut context = HashMap::new();
    context.insert("player_name", HOST_PLAYER_NAME);
    Template::render("index", &context)
}

#[get("/player/<name>")]
pub fn get_player(name: &str) -> Result<String, rocket::http::Status> {
    let data_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("players")
        .join(format!("{}.json", name));

    match File::open(data_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            match file.read_to_string(&mut contents) {
                Ok(_) => Ok(contents),
                Err(e) => {
                    println!("Error reading player file: {}", e);
                    Err(rocket::http::Status::InternalServerError)
                }
            }
        }
        Err(_) => {
            // If the file doesn't exist, create a new player
            println!("Player file not found, creating new player: {}", name);
            let player = Player::create_player(GAME_ID, name);
            match serde_json::to_string(&player) {
                Ok(json) => Ok(json),
                Err(e) => {
                    println!("Error serializing new player: {}", e);
                    Err(rocket::http::Status::InternalServerError)
                }
            }
        }
    }
}

// Returns a serialized JSON string representation of the galaxy map
#[get("/galaxy_map")]
pub fn get_galaxy_map() -> String {
    serde_json::to_string(&get_global_game_world()).unwrap()
}

// Returns a star system with the given id from the galaxy map as a serialized JSON string
#[get("/star_system/<id>")]
pub fn get_star_system(id: usize) -> Option<String> {
    match crate::models::game_world::load_star_system(GAME_ID, id) {
        Ok(Some(system)) => Some(serde_json::to_string(&system).unwrap()),
        Ok(None) => None,
        Err(e) => {
            println!("Error loading star system: {}", e);
            None
        }
    }
}

#[get("/fleet/<owner_id>")]
pub fn get_owner_fleets(owner_id: String) -> Json<Vec<Fleet>> {
    println!("Getting fleets for owner: {}", owner_id);

    match crate::models::fleet::list_owner_fleets(&owner_id) {
        Ok(fleets) => {
            println!("Found {} fleets for owner", fleets.len());
            Json(fleets)
        }
        Err(e) => {
            println!("Error loading fleets: {}", e);
            Json(Vec::new())
        }
    }
}

#[get("/fleet/<owner_id>/<fleet_number>")]
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<Option<Fleet>> {
    println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

    // Try to load existing fleet
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    println!("Looking for fleet with name: {}", fleet_name);
    
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(fleet) => {
            println!("Fleet loaded successfully");
            Json(fleet)
        }
        Err(e) => {
            println!("Error loading fleet: {}", e);
            Json(None)
        }
    }
}

#[get("/planet/<system_id>/<planet_id>/market")]
pub fn get_planet_market(system_id: usize, planet_id: usize) -> Option<Json<Vec<Resource>>> {
    get_global_game_world()
        .get(system_id)
        .and_then(|system| system.planets.get(planet_id))
        .map(|planet| Json(planet.market.clone()))
}

#[get("/planet/<system_id>/<planet_id>/buy/<resource_type>/<quantity>")]
pub fn buy_from_planet(system_id: usize, planet_id: usize, resource_type: ResourceType, quantity: u32) -> Json<String> {
    let mut player = serde_json::from_str(&get_player(HOST_PLAYER_NAME).unwrap()).unwrap();

    if let Ok(Some(mut system)) = crate::models::game_world::load_star_system(GAME_ID, system_id) {
        if let Some(planet) = system.planets.get_mut(planet_id) {
            match planet.buy_resource(resource_type, quantity, &mut player) {
                Ok(_) => {
                    // Save the updated player data
                    let player_path = Path::new("data")
                        .join("game")
                        .join(GAME_ID)
                        .join("players")
                        .join(format!("{}.json", HOST_PLAYER_NAME));
                    
                    let player_file = File::create(player_path).unwrap();
                    serde_json::to_writer(player_file, &player).unwrap();

                    // Save the updated star system
                    if let Err(e) = crate::models::game_world::save_star_system(GAME_ID, system_id, &system) {
                        println!("Error saving star system: {}", e);
                        return Json("Error saving star system".to_string());
                    }
                    
                    // Update the global game world state
                    if let Ok(mut guard) = GLOBAL_GAME_WORLD.lock() {
                        if let Some(existing_system) = guard.get_mut(system_id) {
                            *existing_system = system;
                        }
                    }
                    
                    Json("Successfully bought resource".to_string())
                },
                Err(e) => Json(e),
            }
        } else {
            Json("Planet not found".to_string())
        }
    } else {
        Json("Star system not found".to_string())
    }
}

#[get("/planet/<system_id>/<planet_id>/sell/<resource_type>/<quantity>")]
pub fn sell_to_planet(system_id: usize, planet_id: usize, resource_type: ResourceType, quantity: u32) -> Json<String> {
    let mut player = serde_json::from_str(&get_player(HOST_PLAYER_NAME).unwrap()).unwrap();

    if let Ok(Some(mut system)) = crate::models::game_world::load_star_system(GAME_ID, system_id) {
        if let Some(planet) = system.planets.get_mut(planet_id) {
            match planet.sell_resource(resource_type, quantity, &mut player) {
                Ok(_) => {
                    // Save the updated player data
                    let player_path = Path::new("data")
                        .join("game")
                        .join(GAME_ID)
                        .join("players")
                        .join(format!("{}.json", HOST_PLAYER_NAME));
                    
                    let player_file = File::create(player_path).unwrap();
                    serde_json::to_writer(player_file, &player).unwrap();

                    // Save the updated star system
                    if let Err(e) = crate::models::game_world::save_star_system(GAME_ID, system_id, &system) {
                        println!("Error saving star system: {}", e);
                        return Json("Error saving star system".to_string());
                    }
                    
                    // Update the global game world state
                    if let Ok(mut guard) = GLOBAL_GAME_WORLD.lock() {
                        if let Some(existing_system) = guard.get_mut(system_id) {
                            *existing_system = system;
                        }
                    }
                    
                    Json("Successfully sold resource".to_string())
                },
                Err(e) => Json(e),
            }
        } else {
            Json("Planet not found".to_string())
        }
    } else {
        Json("Star system not found".to_string())
    }
}

#[get("/fleet/<owner_id>/<fleet_number>/move/<x>/<y>/<z>")]
pub fn move_fleet(owner_id: String, fleet_number: usize, x: i32, y: i32, z: i32) -> Json<String> {
    println!("Moving fleet {} for owner: {} to position ({}, {}, {})", fleet_number, owner_id, x, y, z);
    
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(Some(mut fleet)) => {
            let new_position = Position { x, y, z };
            fleet.position = new_position.clone();
            
            // Update all ships in the fleet to the new position
            for ship in &mut fleet.ships {
                ship.position = new_position.clone();
            }
            
            // Save the updated fleet
            if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                println!("Error saving fleet: {}", e);
                return Json("Error saving fleet".to_string());
            }
            
            Json("Fleet moved successfully".to_string())
        }
        Ok(None) => Json("Fleet not found".to_string()),
        Err(e) => {
            println!("Error loading fleet: {}", e);
            Json("Error loading fleet".to_string())
        }
    }
}

#[get("/fleet/owners")]
pub fn get_fleet_owners() -> Json<Vec<String>> {
    let fleets_dir = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("fleets");

    let mut owners = std::collections::HashSet::new();

    if fleets_dir.exists() {
        if let Ok(entries) = fs::read_dir(fleets_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        // Extract owner from fleet name (Fleet_owner_number)
                        if let Some(owner) = file_name.split('_').nth(1) {
                            owners.insert(owner.to_string());
                        }
                    }
                }
            }
        }
    }

    Json(owners.into_iter().collect())
} 