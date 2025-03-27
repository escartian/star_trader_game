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
use crate::encounters::generate_encounter_fleet;


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

    let fleets_dir = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("fleets");

    let mut fleets = Vec::new();

    if fleets_dir.exists() {
        if let Ok(entries) = fs::read_dir(fleets_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(file_name) = entry.file_name().to_str() {
                        // Load all fleets for the owner
                        if file_name.starts_with(&format!("Fleet_{}_", owner_id)) {
                            println!("Loading fleet: {}", file_name);
                            // Remove the .json extension if it exists
                            let fleet_name = file_name.trim_end_matches(".json");
                            if let Ok(Some(fleet)) = crate::models::fleet::load_fleet(fleet_name) {
                                println!("Successfully loaded fleet: {}", fleet.name);
                                fleets.push(fleet);
                            } else {
                                println!("Failed to load fleet: {}", fleet_name);
                            }
                        }
                    }
                }
            }
        }
    }

    println!("Found {} fleets for owner {}", fleets.len(), owner_id);
    Json(fleets)
}

#[get("/fleet/<owner_id>/<fleet_number>")]
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<Option<Fleet>> {
    println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

    // First try to load by owner_id (for regular fleets)
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    
    println!("Looking for fleet with name: {}", fleet_name);
    
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(Some(fleet)) => {
            println!("Fleet loaded successfully: {}", fleet.name);
            Json(Some(fleet))
        }
        Ok(None) => {
            // If not found, try to load by fleet type (for special fleets)
            let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
            for fleet_type in special_types {
                let fleet_name = format!("Fleet_{}_{}", fleet_type, fleet_number);
                match crate::models::fleet::load_fleet(&fleet_name) {
                    Ok(Some(fleet)) => {
                        println!("Special fleet loaded successfully: {}", fleet.name);
                        return Json(Some(fleet));
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        println!("Error loading special fleet: {}", e);
                        continue;
                    }
                }
            }
            println!("Fleet not found: {}", fleet_name);
            Json(None)
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
            
            // Calculate movement distance
            let dx = (fleet.position.x - new_position.x) as f64;
            let dy = (fleet.position.y - new_position.y) as f64;
            let dz = (fleet.position.z - new_position.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            
            fleet.position = new_position.clone();
            fleet.last_move_distance = Some(distance);
            
            // Update all ships in the fleet to the new position
            for ship in &mut fleet.ships {
                ship.position = new_position.clone();
            }
            
            // Save the updated fleet
            if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                println!("Error saving fleet: {}", e);
                return Json("Error saving fleet".to_string());
            }

            // Check for encounters immediately after movement
            let mut encounters = Vec::new();
            
            // Check for star system encounters first
            if let Some(system_id) = fleet.current_system_id {
                if let Ok(Some(system)) = crate::models::game_world::load_star_system(GAME_ID, system_id) {
                    // Check if fleet is near any planets
                    for planet in &system.planets {
                        let planet_pos = planet.position;
                        let distance = ((fleet.position.x - planet_pos.x).pow(2) + 
                                      (fleet.position.y - planet_pos.y).pow(2) + 
                                      (fleet.position.z - planet_pos.z).pow(2)) as f64;
                        if distance <= 10.0 {
                            // Create a fleet representing the planet's defenses
                            let planet_fleet = Fleet {
                                name: format!("Planet_{}", planet.name),
                                owner_id: "Planet".to_string(),
                                ships: Vec::new(), // Planet encounters don't have ships
                                position: planet_pos,
                                current_system_id: Some(system_id),
                                last_move_distance: None,
                            };
                            encounters.push(planet_fleet);
                        }
                    }
                }
            }
            
            // Generate random encounters based on distance
            let max_encounters = (distance / 10.0).min(3.0) as i32;
            
            for _ in 0..max_encounters {
                if rand::random::<f64>() < 0.3 { // 30% chance for each potential encounter
                    let encounter_fleet = generate_encounter_fleet(fleet.position.clone());
                    
                    // Only add the encounter if it's not the same owner as the player's fleet
                    if encounter_fleet.owner_id != owner_id {
                        let fleet = Fleet {
                            name: encounter_fleet.name,
                            owner_id: encounter_fleet.owner_id,
                            ships: encounter_fleet.ships,
                            position: encounter_fleet.position,
                            current_system_id: fleet.current_system_id,
                            last_move_distance: None,
                        };
                        encounters.push(fleet);
                    }
                }
            }
            
            // If we have encounters, return them along with the movement success message
            if !encounters.is_empty() {
                let response = serde_json::json!({
                    "status": "success",
                    "message": "Fleet moved successfully",
                    "encounters": encounters
                });
                return Json(response.to_string());
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
                        // Handle fleet naming scheme
                        if file_name.starts_with("Fleet_") {
                            // Regular fleet (Fleet_owner_number)
                            if let Some(owner) = file_name.split('_').nth(1) {
                                println!("Found fleet owner: {}", owner);
                                owners.insert(owner.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    let owners_vec: Vec<String> = owners.into_iter().collect();
    println!("Found {} fleet owners: {:?}", owners_vec.len(), owners_vec);
    Json(owners_vec)
}

#[get("/fleet/<attacker_id>/<attacker_number>/attack/<defender_id>/<defender_number>")]
pub fn initiate_combat(attacker_id: String, attacker_number: usize, defender_id: String, defender_number: usize) -> Json<String> {
    println!("Initiating combat between Fleet_{}_{} and Fleet_{}_{}", 
        attacker_id, attacker_number, defender_id, defender_number);

    let attacker_name = format!("Fleet_{}_{}", attacker_id, attacker_number);
    let defender_name = format!("Fleet_{}_{}", defender_id, defender_number);

    match (crate::models::fleet::load_fleet(&attacker_name), crate::models::fleet::load_fleet(&defender_name)) {
        (Ok(Some(mut attacker)), Ok(Some(mut defender))) => {
            if !crate::combat::combat::can_engage_combat(&attacker, &defender) {
                return Json("Fleets must be at the same position to engage in combat".to_string());
            }

            let combat_result = crate::combat::combat::auto_resolve_ship_combat(&mut attacker, &mut defender);

            // Save the updated fleets
            if let Err(e) = crate::models::fleet::save_fleet(&attacker) {
                println!("Error saving attacker fleet: {}", e);
                return Json("Error saving attacker fleet".to_string());
            }

            if let Err(e) = crate::models::fleet::save_fleet(&defender) {
                println!("Error saving defender fleet: {}", e);
                return Json("Error saving defender fleet".to_string());
            }

            // Format combat result
            let mut result = String::new();
            for log in combat_result.combat_log {
                result.push_str(&format!("{}\n", log));
            }
            result.push_str(&format!("\nFinal fleet sizes:\nAttacker: {} ships\nDefender: {} ships", 
                attacker.ships.len(), defender.ships.len()));

            Json(result)
        }
        (Ok(None), _) => Json("Attacker fleet not found".to_string()),
        (_, Ok(None)) => Json("Defender fleet not found".to_string()),
        (Err(e), _) => Json(format!("Error loading attacker fleet: {}", e)),
        (_, Err(e)) => Json(format!("Error loading defender fleet: {}", e)),
    }
}

#[get("/fleet/<owner_id>/<fleet_number>/encounter")]
pub fn check_for_encounter(owner_id: String, fleet_number: usize) -> Json<Vec<Fleet>> {
    println!("Checking for encounters for Fleet_{}_{}", owner_id, fleet_number);
    
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(Some(fleet)) => {
            let mut encounters = Vec::new();
            
            // Check for star system encounters first
            if let Some(system_id) = fleet.current_system_id {
                if let Ok(Some(system)) = crate::models::game_world::load_star_system(GAME_ID, system_id) {
                    // Check if fleet is near any planets
                    for planet in &system.planets {
                        let planet_pos = planet.position;
                        let distance = ((fleet.position.x - planet_pos.x).pow(2) + 
                                      (fleet.position.y - planet_pos.y).pow(2) + 
                                      (fleet.position.z - planet_pos.z).pow(2)) as f64;
                        if distance <= 10.0 {
                            // Create a fleet representing the planet's defenses
                            let planet_fleet = Fleet {
                                name: format!("Planet_{}", planet.name),
                                owner_id: "Planet".to_string(),
                                ships: Vec::new(), // Planet encounters don't have ships
                                position: planet_pos,
                                current_system_id: Some(system_id),
                                last_move_distance: None,
                            };
                            encounters.push(planet_fleet);
                        }
                    }
                }
            }
            
            // Generate random encounters based on distance
            let distance = fleet.last_move_distance.unwrap_or(0.0);
            let max_encounters = (distance / 10.0).min(3.0) as i32;
            
            for _ in 0..max_encounters {
                if rand::random::<f64>() < 0.3 { // 30% chance for each potential encounter
                    let encounter_fleet = generate_encounter_fleet(fleet.position.clone());
                    
                    // Only add the encounter if it's not the same owner as the player's fleet
                    if encounter_fleet.owner_id != owner_id {
                        let fleet = Fleet {
                            name: encounter_fleet.name,
                            owner_id: encounter_fleet.owner_id,
                            ships: encounter_fleet.ships,
                            position: encounter_fleet.position,
                            current_system_id: fleet.current_system_id,
                            last_move_distance: None,
                        };
                        encounters.push(fleet);
                    }
                }
            }
            
            // If we have encounters, save the current fleet's last move distance
            if !encounters.is_empty() {
                if let Ok(Some(mut current_fleet)) = crate::models::fleet::load_fleet(&fleet_name) {
                    current_fleet.last_move_distance = Some(distance);
                    if let Err(e) = crate::models::fleet::save_fleet(&current_fleet) {
                        println!("Error saving fleet's last move distance: {}", e);
                    }
                }
            }
            
            Json(encounters)
        }
        Ok(None) => Json(Vec::new()),
        Err(e) => {
            println!("Error loading fleet: {}", e);
            Json(Vec::new())
        }
    }
} 