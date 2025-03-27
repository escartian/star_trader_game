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
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<Fleet> {
    println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

    // First try to load by owner_id (for regular fleets)
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    
    println!("Looking for fleet with name: {}", fleet_name);
    
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(Some(fleet)) => Json(fleet),
        Ok(None) => {
            // If not found, try to load by fleet type (for special fleets)
            let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
            for fleet_type in special_types {
                let fleet_name = format!("Fleet_{}_{}", fleet_type, fleet_number);
                match crate::models::fleet::load_fleet(&fleet_name) {
                    Ok(Some(fleet)) => return Json(fleet),
                    Ok(None) => continue,
                    Err(e) => {
                        println!("Error loading special fleet: {}", e);
                        continue;
                    }
                }
            }
            println!("Fleet not found: {}", fleet_name);
            Json(Fleet {
                name: "Fleet not found".to_string(),
                owner_id: "".to_string(),
                ships: Vec::new(),
                position: Position { x: 0, y: 0, z: 0 },
                current_system_id: None,
                last_move_distance: None,
            })
        }
        Err(e) => {
            println!("Error loading fleet: {}", e);
            Json(Fleet {
                name: "Error loading fleet".to_string(),
                owner_id: "".to_string(),
                ships: Vec::new(),
                position: Position { x: 0, y: 0, z: 0 },
                current_system_id: None,
                last_move_distance: None,
            })
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
    println!("Starting fleet move operation:");
    println!("  Fleet: Fleet_{}_{}", owner_id, fleet_number);
    println!("  Target position: ({}, {}, {})", x, y, z);
    
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);

    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(Some(mut fleet)) => {
            println!("Successfully loaded fleet: {}", fleet.name);
            let new_position = Position { x, y, z };
            
            // Calculate movement distance and direction
            let dx = (new_position.x - fleet.position.x) as f64;
            let dy = (new_position.y - fleet.position.y) as f64;
            let dz = (new_position.z - fleet.position.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            println!("Movement distance: {}", distance);
            
            // Check for encounters along the path
            let mut encounters = Vec::new();
            
            // Calculate path steps
            let steps = (distance / 5.0).ceil() as i32; // Check every 5 units
            let mut current_position = fleet.position.clone();
            
            println!("Checking path for encounters...");
            for step in 0..steps {
                let t = step as f64 / steps as f64;
                let check_position = Position {
                    x: fleet.position.x + (dx * t) as i32,
                    y: fleet.position.y + (dy * t) as i32,
                    z: fleet.position.z + (dz * t) as i32,
                };
                
                // Update current position
                current_position = check_position.clone();
                
                // Check for random encounters
                if rand::random::<f64>() < 0.1 { // 10% chance per step
                    println!("Random encounter chance triggered at step {}", step);
                    let mut encounter_fleet = generate_encounter_fleet(check_position.clone());
                    if encounter_fleet.owner_id != owner_id {
                        println!("Generated encounter fleet: {}", encounter_fleet.name);
                        
                        // Save the encounter fleet to ensure it exists
                        let fleet = Fleet {
                            name: encounter_fleet.name.clone(),
                            owner_id: encounter_fleet.owner_id.clone(),
                            ships: encounter_fleet.ships.clone(),
                            position: encounter_fleet.position.clone(),
                            current_system_id: fleet.current_system_id,
                            last_move_distance: None,
                        };
                        if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                            println!("Error saving encounter fleet: {}", e);
                            continue;
                        }
                        
                        encounters.push(fleet);
                    }
                }
                
                // Check for star system encounters
                for (system_id, system) in get_global_game_world().iter().enumerate() {
                    let system_distance = ((check_position.x - system.position.x).pow(2) + 
                                        (check_position.y - system.position.y).pow(2) + 
                                        (check_position.z - system.position.z).pow(2)) as f64;
                    if system_distance <= 10.0 && fleet.current_system_id.is_none() {
                        println!("Entering star system: {}", system_id);
                        let system_fleet = Fleet {
                            name: format!("StarSystem_{}", system_id),
                            owner_id: "StarSystem".to_string(),
                            ships: Vec::new(),
                            position: system.position,
                            current_system_id: Some(system_id),
                            last_move_distance: None,
                        };
                        encounters.push(system_fleet);
                    }
                }
                
                // Check for planet encounters if in a system
                if let Some(system_id) = fleet.current_system_id {
                    if let Ok(Some(system)) = crate::models::game_world::load_star_system(GAME_ID, system_id) {
                        for planet in &system.planets {
                            let planet_pos = planet.position;
                            let distance = ((check_position.x - planet_pos.x).pow(2) + 
                                          (check_position.y - planet_pos.y).pow(2) + 
                                          (check_position.z - planet_pos.z).pow(2)) as f64;
                            if distance <= 10.0 {
                                println!("Planet encounter detected: {}", planet.name);
                                let planet_fleet = Fleet {
                                    name: format!("Planet_{}", planet.name),
                                    owner_id: "Planet".to_string(),
                                    ships: Vec::new(),
                                    position: planet_pos,
                                    current_system_id: Some(system_id),
                                    last_move_distance: None,
                                };
                                encounters.push(planet_fleet);
                            }
                        }
                    }
                }
                
                // If we have encounters, return them with current position
                if !encounters.is_empty() {
                    println!("Found {} encounters at position ({}, {}, {})", 
                        encounters.len(), current_position.x, current_position.y, current_position.z);
                    
                    // Update fleet position to current position
                    fleet.position = current_position.clone();
                    fleet.last_move_distance = Some(distance * t);
                    
                    // Update current_system_id if needed
                    for (system_id, system) in get_global_game_world().iter().enumerate() {
                        let system_distance = ((current_position.x - system.position.x).pow(2) + 
                                            (current_position.y - system.position.y).pow(2) + 
                                            (current_position.z - system.position.z).pow(2)) as f64;
                        if system_distance <= 10.0 {
                            fleet.current_system_id = Some(system_id);
                            break;
                        }
                    }
                    
                    // Save the updated fleet position
                    if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                        println!("Error saving fleet position: {}", e);
                    }
                    
                    let response = serde_json::json!({
                        "status": "encounter",
                        "message": "Encounter detected during movement",
                        "encounters": encounters,
                        "current_position": current_position,
                        "target_position": new_position,
                        "remaining_distance": distance * (1.0 - t)
                    });
                    return Json(response.to_string());
                }
            }
            
            println!("No encounters found, proceeding with move");
            // If no encounters, proceed with the move
            fleet.position = new_position.clone();
            fleet.last_move_distance = Some(distance);
            
            // Update current_system_id based on final position
            for (system_id, system) in get_global_game_world().iter().enumerate() {
                let system_distance = ((new_position.x - system.position.x).pow(2) + 
                                    (new_position.y - system.position.y).pow(2) + 
                                    (new_position.z - system.position.z).pow(2)) as f64;
                if system_distance <= 10.0 {
                    fleet.current_system_id = Some(system_id);
                    break;
                }
            }
            
            // Update all ships in the fleet to the new position
            for ship in &mut fleet.ships {
                ship.position = new_position.clone();
            }
            
            // Save the updated fleet
            if let Err(e) = crate::models::fleet::save_fleet(&fleet) {
                println!("Error saving fleet: {}", e);
                return Json("Error saving fleet".to_string());
            }
            
            println!("Fleet move completed successfully");
            Json("Fleet moved successfully".to_string())
        }
        Ok(None) => {
            println!("Fleet not found: {}", fleet_name);
            Json("Fleet not found".to_string())
        }
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
    println!("Starting combat initiation:");
    println!("  Attacker: Fleet_{}_{}", attacker_id, attacker_number);
    println!("  Defender: Fleet_{}_{}", defender_id, defender_number);

    // First try to load attacker fleet
    println!("Loading attacker fleet...");
    let attacker_name = format!("Fleet_{}_{}", attacker_id, attacker_number);
    let mut attacker_result = Err("Attacker fleet not found".to_string());
    
    // Try loading regular fleet first
    match crate::models::fleet::load_fleet(&attacker_name) {
        Ok(Some(fleet)) => {
            println!("Found attacker fleet directly: {}", fleet.name);
            attacker_result = Ok(fleet);
        }
        Ok(None) => {
            println!("Regular fleet not found, trying special types...");
            // Try special fleet types
            let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
            for fleet_type in special_types {
                let fleet_name = format!("Fleet_{}_{}", fleet_type, attacker_number);
                println!("Trying special fleet: {}", fleet_name);
                match crate::models::fleet::load_fleet(&fleet_name) {
                    Ok(Some(fleet)) => {
                        println!("Found attacker fleet: {}", fleet.name);
                        attacker_result = Ok(fleet);
                        break;
                    }
                    Ok(None) => continue,
                    Err(e) => {
                        println!("Error loading special fleet: {}", e);
                        continue;
                    }
                }
            }
        }
        Err(e) => {
            println!("Error loading attacker fleet: {}", e);
        }
    }

    // Now try to load defender fleet
    println!("Loading defender fleet...");
    let defender_name = format!("Fleet_{}_{}", defender_id, defender_number);
    let mut defender_result = Err("Defender fleet not found".to_string());
    
    // Try loading regular fleet first
    match crate::models::fleet::load_fleet(&defender_name) {
        Ok(Some(fleet)) => {
            println!("Found defender fleet directly: {}", fleet.name);
            defender_result = Ok(fleet);
        }
        Ok(None) => {
            println!("Regular fleet not found, trying special types...");
            // Try special fleet types
            let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
            for fleet_type in special_types {
                // Try both naming formats
                let fleet_names = vec![
                    format!("Fleet_{}_{}", fleet_type, defender_number),
                    format!("{}_Fleet_{}", fleet_type, defender_number)
                ];
                
                for fleet_name in fleet_names {
                    println!("Trying special fleet: {}", fleet_name);
                    match crate::models::fleet::load_fleet(&fleet_name) {
                        Ok(Some(fleet)) => {
                            println!("Found defender fleet: {}", fleet.name);
                            defender_result = Ok(fleet);
                            break;
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            println!("Error loading special fleet: {}", e);
                            continue;
                        }
                    }
                }
                
                if defender_result.is_ok() {
                    break;
                }
            }
        }
        Err(e) => {
            println!("Error loading defender fleet: {}", e);
        }
    }

    match (attacker_result, defender_result) {
        (Ok(mut attacker), Ok(mut defender)) => {
            println!("Both fleets loaded successfully:");
            println!("  Attacker: {} ({} ships)", attacker.name, attacker.ships.len());
            println!("  Defender: {} ({} ships)", defender.name, defender.ships.len());

            if !crate::combat::combat::can_engage_combat(&attacker, &defender) {
                println!("Fleets cannot engage in combat - not at same position");
                return Json("Fleets must be at the same position to engage in combat".to_string());
            }

            println!("Starting combat resolution...");
            let combat_result = crate::combat::combat::auto_resolve_ship_combat(&mut attacker, &mut defender);

            // Save the updated fleets
            println!("Saving updated fleets...");
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

            println!("Combat completed successfully");
            Json(result)
        }
        (Err(e), _) => {
            println!("Error loading attacker fleet: {}", e);
            Json(e)
        }
        (_, Err(e)) => {
            println!("Error loading defender fleet: {}", e);
            Json(e)
        }
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

#[get("/fleet/<owner_id>/<fleet_number>/trade/<resource_type>/<quantity>/<trade_type>")]
pub fn trade_with_trader(owner_id: String, fleet_number: usize, resource_type: ResourceType, quantity: u32, trade_type: String) -> Json<String> {
    println!("Starting trade operation:");
    println!("  Fleet: Fleet_{}_{}", owner_id, fleet_number);
    println!("  Resource: {}", resource_type);
    println!("  Quantity: {}", quantity);
    println!("  Trade Type: {}", trade_type);

    let mut player: Player = serde_json::from_str(&get_player(HOST_PLAYER_NAME).unwrap()).unwrap();
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    let trader_fleet_name = format!("Fleet_Trader_{}", fleet_number);

    // Load both fleets
    match (crate::models::fleet::load_fleet(&fleet_name), crate::models::fleet::load_fleet(&trader_fleet_name)) {
        (Ok(Some(mut player_fleet)), Ok(Some(mut trader_fleet))) => {
            // Find the resource in trader's cargo
            let mut trader_resource = None;
            let mut trader_ship_index = 0;
            let mut cargo_index = 0;

            for (ship_idx, ship) in trader_fleet.ships.iter().enumerate() {
                for (cargo_idx, cargo) in ship.cargo.iter().enumerate() {
                    if cargo.resource_type == resource_type {
                        trader_resource = Some(cargo.clone());
                        trader_ship_index = ship_idx;
                        cargo_index = cargo_idx;
                        break;
                    }
                }
                if trader_resource.is_some() {
                    break;
                }
            }

            match trader_resource {
                Some(resource) => {
                    match trade_type.as_str() {
                        "buy" => {
                            // Calculate total cost
                            let total_cost = (resource.buy.unwrap_or(0.0) * quantity as f32) as f32;
                            
                            // Check if player has enough credits
                            if player.credits < total_cost {
                                return Json("Insufficient credits".to_string());
                            }

                            // Check if trader has enough quantity
                            if resource.quantity.unwrap_or(0) < quantity {
                                return Json("Trader doesn't have enough resources".to_string());
                            }

                            // Update player's credits and cargo
                            player.credits -= total_cost;
                            
                            // Add cargo to player's fleet
                            let mut found = false;
                            for ship in &mut player_fleet.ships {
                                for cargo in &mut ship.cargo {
                                    if cargo.resource_type == resource_type {
                                        cargo.quantity = Some(cargo.quantity.unwrap_or(0) + quantity);
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }

                            if !found {
                                // Add new cargo item if not found
                                if let Some(ship) = player_fleet.ships.first_mut() {
                                    ship.cargo.push(Resource {
                                        resource_type: resource_type.clone(),
                                        quantity: Some(quantity),
                                        buy: None,
                                        sell: None
                                    });
                                }
                            }

                            // Update trader's cargo
                            if let Some(ship) = trader_fleet.ships.get_mut(trader_ship_index) {
                                if let Some(cargo) = ship.cargo.get_mut(cargo_index) {
                                    cargo.quantity = Some(cargo.quantity.unwrap_or(0) - quantity);
                                }
                            }
                        },
                        "sell" => {
                            // Find resource in player's fleet
                            let mut player_resource = None;
                            let mut player_ship_index = 0;
                            let mut player_cargo_index = 0;

                            for (ship_idx, ship) in player_fleet.ships.iter().enumerate() {
                                for (cargo_idx, cargo) in ship.cargo.iter().enumerate() {
                                    if cargo.resource_type == resource_type {
                                        player_resource = Some(cargo.clone());
                                        player_ship_index = ship_idx;
                                        player_cargo_index = cargo_idx;
                                        break;
                                    }
                                }
                                if player_resource.is_some() {
                                    break;
                                }
                            }

                            match player_resource {
                                Some(resource) => {
                                    // Check if player has enough quantity
                                    if resource.quantity.unwrap_or(0) < quantity {
                                        return Json("You don't have enough resources".to_string());
                                    }

                                    // Calculate total earnings
                                    let total_earnings = (resource.sell.unwrap_or(0.0) * quantity as f32) as f32;
                                    
                                    // Update player's credits and cargo
                                    player.credits += total_earnings;
                                    
                                    // Update player's cargo
                                    if let Some(ship) = player_fleet.ships.get_mut(player_ship_index) {
                                        if let Some(cargo) = ship.cargo.get_mut(player_cargo_index) {
                                            cargo.quantity = Some(cargo.quantity.unwrap_or(0) - quantity);
                                        }
                                    }

                                    // Add cargo to trader's fleet
                                    let mut found = false;
                                    for ship in &mut trader_fleet.ships {
                                        for cargo in &mut ship.cargo {
                                            if cargo.resource_type == resource_type {
                                                cargo.quantity = Some(cargo.quantity.unwrap_or(0) + quantity);
                                                found = true;
                                                break;
                                            }
                                        }
                                        if found {
                                            break;
                                        }
                                    }

                                    if !found {
                                        // Add new cargo item if not found
                                        if let Some(ship) = trader_fleet.ships.first_mut() {
                                            ship.cargo.push(Resource {
                                                resource_type: resource_type.clone(),
                                                quantity: Some(quantity),
                                                buy: resource.buy,
                                                sell: resource.sell
                                            });
                                        }
                                    }
                                },
                                None => {
                                    return Json("You don't have this resource".to_string());
                                }
                            }
                        },
                        _ => {
                            return Json("Invalid trade type".to_string());
                        }
                    }

                    // Save all changes
                    if let Err(e) = crate::models::fleet::save_fleet(&player_fleet) {
                        println!("Error saving player fleet: {}", e);
                        return Json("Error saving player fleet".to_string());
                    }

                    if let Err(e) = crate::models::fleet::save_fleet(&trader_fleet) {
                        println!("Error saving trader fleet: {}", e);
                        return Json("Error saving trader fleet".to_string());
                    }

                    let player_path = Path::new("data")
                        .join("game")
                        .join(GAME_ID)
                        .join("players")
                        .join(format!("{}.json", HOST_PLAYER_NAME));
                    
                    let player_file = File::create(player_path).unwrap();
                    serde_json::to_writer(player_file, &player).unwrap();

                    Json("Success".to_string())
                },
                None => {
                    Json("Resource not found in trader's cargo".to_string())
                }
            }
        },
        (Ok(Some(_)), Ok(None)) => {
            Json("Trader fleet not found".to_string())
        },
        (Ok(None), _) => {
            Json("Player fleet not found".to_string())
        },
        (Err(e), _) => {
            println!("Error loading player fleet: {}", e);
            Json("Error loading player fleet".to_string())
        },
        (_, Err(e)) => {
            println!("Error loading trader fleet: {}", e);
            Json("Error loading trader fleet".to_string())
        }
    }
} 