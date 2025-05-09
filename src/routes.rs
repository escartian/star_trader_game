use crate::models::game_world::get_global_game_world;
use crate::models::game_state::GAME_STATE;
use std::fs::File;
use std::path::Path;
use rocket::Request;
use rocket::{get, delete};
use std::io::Read;
use crate::models::star_system::StarSystem;
use rocket::catch;
use rocket::serde::json::Json;
use crate::models::fleet::{Fleet, generate_and_save_fleet, list_owner_fleets, save_fleet, MoveFleetResponse, MoveFleetData};
use crate::models::resource::{Resource, ResourceType};
use crate::models::player::Player;
use rand::{Rng, thread_rng};
use serde_json;
use crate::models::position::{Position, random_position};
use std::fs;
use crate::encounters::generate_encounter_fleet;
use rocket::post;
use serde::Deserialize;
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine};
use crate::models::market::{Market, ShipMarket, regenerate_system_markets, calculate_ship_price};
use crate::models::response::ApiResponse;
use crate::models::game_state::{load_player, load_star_system, save_trade_state, game_path, ensure_parent_dirs, save_json, load_json, load_fleet};
use crate::models::trade::{ResourceTradeData, ShipTradeData, ShipTradeInData, trade_with_fleet};
use crate::models::settings::{GameSettings, SavedGame, load_settings};
use chrono::Utc;
use std::collections::HashMap;
use crate::models::faction::{Faction, save_faction};
use crate::models::planet::PlanetSpecialization;
use crate::models::economy::Economy;
use std::error::Error;
use strum::IntoEnumIterator;
use std::sync::{Arc, Mutex};

use crate::models::planet::{load_planet_market, load_planet_ship_market};


#[catch(500)]
pub fn internal_error(_req: &Request) -> Json<ApiResponse<String>> {
    ApiResponse::error("An internal server error occurred. Please try again later.".to_string())
}

// Possible future features::
// TODO: 
// Implement multiplayer support
// Implement player authentication
// TODO: Implement game events system

/// Handles the root route (`/`) and renders the index page
#[get("/")]
pub fn index() -> Json<ApiResponse<String>> {
    ApiResponse::success("Welcome to Star Trader Game API".to_string(), "Success".to_string())
}

#[get("/player/<name>")]
pub fn get_player(name: &str) -> Json<ApiResponse<Player>> {
    let result: Result<Player, String> = (|| {
        let player = load_player(name)?;
        Ok(player)
    })();

    match result {
        Ok(player) => ApiResponse::success(player, "Successfully retrieved player".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

// Returns a serialized JSON string representation of the galaxy map
#[get("/galaxy_map")]
pub fn get_galaxy_map() -> Json<ApiResponse<Vec<StarSystem>>> {
    let _settings = load_settings().expect("Failed to load settings");
    let game_world_path = game_path(&["GameWorld.json"]);

    // Try to load from GameWorld.json
    if let Ok(world) = load_json::<Vec<StarSystem>>(&game_world_path) {
        return ApiResponse::success(world, "Successfully retrieved galaxy map".to_string());
    }

    // If GameWorld.json doesn't exist or can't be loaded, return empty vector
    ApiResponse::success(Vec::new(), "No galaxy map found".to_string())
}

// Returns a star system with the given id from the galaxy map as a serialized JSON string
#[get("/star_system/<system_id>")]
pub fn get_star_system(system_id: usize) -> Json<ApiResponse<StarSystem>> {
    let settings = load_settings().expect("Failed to load settings");
    let system_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("star_systems")
        .join(format!("Star_System_{}.json", system_id));

    match load_json::<StarSystem>(&system_path) {
        Ok(system) => ApiResponse::success(system, "Successfully retrieved star system".to_string()),
        Err(e) => {
            println!("Error loading star system: {}", e);
            // Try loading from the game world file
            let game_world_path = game_path(&["GameWorld.json"]);
            match load_json::<Vec<StarSystem>>(&game_world_path) {
                Ok(systems) => {
                    if system_id < systems.len() {
                        ApiResponse::success(systems[system_id].clone(), "Successfully retrieved star system from game world".to_string())
                    } else {
                        ApiResponse::error(format!("System ID {} not found in game world", system_id))
                    }
                },
                Err(e) => ApiResponse::error(format!("Failed to load star system: {}", e))
            }
        }
    }
}

#[get("/fleet/<owner_id>")]
pub fn get_owner_fleets(owner_id: String) -> Json<ApiResponse<Vec<Fleet>>> {
    println!("Getting fleets for owner: {}", owner_id);

    let result: Result<Vec<Fleet>, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let fleets_dir = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("fleets");

        println!("Looking for fleets in directory: {}", fleets_dir.display());
        let mut fleets = Vec::new();

        if fleets_dir.exists() {
            if let Ok(entries) = fs::read_dir(fleets_dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if let Some(file_name) = entry.file_name().to_str() {
                            println!("Found fleet file: {}", file_name);
                            // Handle fleet naming scheme
                            if file_name.starts_with("Fleet_") {
                                let parts: Vec<&str> = file_name.split('_').collect();
                                if parts.len() >= 3 {
                                    let fleet_owner = parts[1];
                                    if fleet_owner == owner_id {
                                        let fleet_name = file_name.trim_end_matches(".json");
                                        println!("Loading fleet: {}", fleet_name);
                                        match crate::models::fleet::load_fleet(fleet_name) {
                                            Ok(Some(fleet)) => {
                                                println!("Successfully loaded fleet: {} with {} ships", fleet.name, fleet.ships.len());
                                                fleets.push(fleet);
                                            },
                                            Ok(None) => println!("Fleet not found: {}", fleet_name),
                                            Err(e) => println!("Error loading fleet {}: {}", fleet_name, e),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                println!("Error reading fleets directory");
                return Err("Failed to read fleets directory".to_string());
            }
        } else {
            println!("Fleets directory does not exist");
            return Err("Fleets directory not found".to_string());
        }

        println!("Found {} fleets for owner {}", fleets.len(), owner_id);
        Ok(fleets)
    })();

    match result {
        Ok(fleets) => ApiResponse::success(fleets, "Successfully retrieved fleets".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[get("/fleet/<owner_id>/<fleet_number>")]
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<ApiResponse<Fleet>> {
    println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

    let result: Result<Fleet, String> = (|| {
        // First try to load by owner_id (for regular fleets)
        let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
        
        println!("Looking for fleet with name: {}", fleet_name);
        
        match crate::models::fleet::load_fleet(&fleet_name) {
            Ok(Some(fleet)) => Ok(fleet),
            Ok(None) => {
                // If not found, try to load by fleet type (for special fleets)
                let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
                for fleet_type in special_types {
                    let fleet_name = format!("Fleet_{}_{}", fleet_type, fleet_number);
                    match crate::models::fleet::load_fleet(&fleet_name) {
                        Ok(Some(fleet)) => return Ok(fleet),
                        Ok(None) => continue,
                        Err(e) => {
                            println!("Error loading special fleet: {}", e);
                            continue;
                        }
                    }
                }
                Err("Fleet not found".to_string())
            }
            Err(e) => {
                println!("Error loading fleet: {}", e);
                Err(e.to_string())
            }
        }
    })();

    match result {
        Ok(fleet) => ApiResponse::success(fleet, "Successfully retrieved fleet".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[get("/planet/<system_id>/<planet_id>/market")]
pub fn get_planet_market(system_id: usize, planet_id: usize) -> Json<ApiResponse<Market>> {
    match load_planet_market(system_id, planet_id) {
        Ok(market) => ApiResponse::success(market, "Successfully retrieved market".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[get("/planet/<system_id>/<planet_id>/ships")]
pub fn get_planet_ship_market(system_id: usize, planet_id: usize) -> Json<ApiResponse<ShipMarket>> {
    match load_planet_ship_market(system_id, planet_id) {
        Ok(market) => ApiResponse::success(market, "Successfully retrieved ship market".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/planet/<system_id>/<planet_id>/buy", format = "json", data = "<data>")]
pub fn buy_from_planet(system_id: usize, planet_id: usize, data: Json<ResourceTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name)?;
        let mut system = load_star_system(system_id)?;
        let mut market = Market::load(system_id, planet_id).map_err(|e| e.to_string())?;
        
        let planet = system.planets.get_mut(planet_id)
            .ok_or_else(|| "Planet not found".to_string())?;
        
        // Calculate total cost and update market quantities
        let total_cost = market.buy_resource(data.resource_type, data.quantity, system_id, planet_id)
            .map_err(|e| e.to_string())?;
        
        // Check if player has enough credits
        if player.credits < total_cost as f32 {
            return Err("Insufficient credits".to_string());
        }
        
        // Update player's inventory and credits
        player.credits -= total_cost as f32;
        player.add_resource(data.resource_type, data.quantity);
        
        // Save both player and market state
        player.save().map_err(|e| e.to_string())?;
        market.save(system_id, planet_id).map_err(|e| e.to_string())?;
        
        Ok("Successfully bought resource".to_string())
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Trade completed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/planet/<system_id>/<planet_id>/sell", format = "json", data = "<data>")]
pub fn sell_to_planet(system_id: usize, planet_id: usize, data: Json<ResourceTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name)?;
        let mut system = load_star_system(system_id)?;
        let mut market = Market::load(system_id, planet_id).map_err(|e| e.to_string())?;
        
        let planet = system.planets.get_mut(planet_id)
            .ok_or_else(|| "Planet not found".to_string())?;
        
        // Check if player has enough resources
        if !player.has_resource(data.resource_type, data.quantity) {
            return Err("Not enough resources in inventory".to_string());
        }
        
        // Calculate total value and update market quantities
        let total_value = market.sell_resource(data.resource_type, data.quantity, system_id, planet_id)
            .map_err(|e| e.to_string())?;
        
        // Update player's inventory and credits
        player.credits += total_value as f32;
        player.remove_resource(data.resource_type, data.quantity);
        
        // Save both player and market state
        player.save().map_err(|e| e.to_string())?;
        market.save(system_id, planet_id).map_err(|e| e.to_string())?;
        
        Ok("Successfully sold resource".to_string())
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Trade completed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

/// Validates the target position against galaxy bounds
/// 
/// # Arguments
/// * `target_pos` - The target position to validate
/// * `settings` - Game settings containing map dimensions
/// 
/// # Returns
/// * `Ok(())` if position is valid
/// * `Err(String)` with descriptive error message if invalid
fn validate_galaxy_bounds(target_pos: &Position, settings: &GameSettings) -> Result<(), String> {
    let galaxy_max = settings.map_width as i32;
    let galaxy_min = -galaxy_max;

    // Determine which axes are out of bounds
    let is_out_x = target_pos.x < galaxy_min || target_pos.x > galaxy_max;
    let is_out_y = target_pos.y < galaxy_min || target_pos.y > galaxy_max;
    let is_out_z = target_pos.z < galaxy_min || target_pos.z > galaxy_max;

    if is_out_x || is_out_y || is_out_z {
        // Allow axis-aligned system exit: exactly one axis outside by one unit
        let mut outside_count = 0;
        if is_out_x {
            if target_pos.x != galaxy_max + 1 && target_pos.x != galaxy_min - 1 {
                return Err(format!("Target position ({}, {}, {}) is outside galaxy bounds [{} to {}]. Please choose coordinates within the galaxy boundaries.",
                    target_pos.x, target_pos.y, target_pos.z, galaxy_min, galaxy_max));
            }
            outside_count += 1;
        }
        if is_out_y {
            if target_pos.y != galaxy_max + 1 && target_pos.y != galaxy_min - 1 {
                return Err(format!("Target position ({}, {}, {}) is outside galaxy bounds [{} to {}]. Please choose coordinates within the galaxy boundaries.",
                    target_pos.x, target_pos.y, target_pos.z, galaxy_min, galaxy_max));
            }
            outside_count += 1;
        }
        if is_out_z {
            if target_pos.z != galaxy_max + 1 && target_pos.z != galaxy_min - 1 {
                return Err(format!("Target position ({}, {}, {}) is outside galaxy bounds [{} to {}]. Please choose coordinates within the galaxy boundaries.",
                    target_pos.x, target_pos.y, target_pos.z, galaxy_min, galaxy_max));
            }
            outside_count += 1;
        }
        if outside_count > 1 {
            return Err(format!("Target position ({}, {}, {}) is outside galaxy bounds [{} to {}]. Please choose coordinates within the galaxy boundaries.",
                target_pos.x, target_pos.y, target_pos.z, galaxy_min, galaxy_max));
        }
    }

    Ok(())
}

/// Handles fleet movement within a star system
/// 
/// # Arguments
/// * `fleet` - The fleet to move
/// * `target_pos` - Target position within the system
/// * `system` - The current star system
/// * `system_id` - ID of the current system
/// 
/// # Returns
/// * `Ok((MoveFleetResponse, Fleet))` if movement successful
/// * `Err(String)` with descriptive error message if movement fails
fn handle_system_movement(
    mut fleet: Fleet,
    target_pos: Position,
    system: &StarSystem,
    system_id: usize
) -> Result<(MoveFleetResponse, Fleet), String> {
    println!("Handling system movement:");
    println!("  Fleet position: ({}, {}, {})", fleet.position.x, fleet.position.y, fleet.position.z);
    println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
    println!("  System position: ({}, {}, {})", system.position.x, system.position.y, system.position.z);
    
    let system_max = 100; // Fixed system size for testing
    let system_min = -system_max;
    
    // Check if target is outside or ON the system bounds (exclusive of system_min/max themselves)
    if target_pos.x <= system_min || target_pos.x >= system_max ||
       target_pos.y <= system_min || target_pos.y >= system_max ||
       target_pos.z <= system_min || target_pos.z >= system_max {
        
        println!("Target is on or outside system bounds. Triggering system exit.");
        return handle_system_exit(fleet, &target_pos, system, system_max);
    }

    // Local move within system
    println!("Performing local move within system {}", system_id);
    
    let local_dx = (target_pos.x - fleet.position.x) as f64;
    let local_dy = (target_pos.y - fleet.position.y) as f64;
    let local_dz = (target_pos.z - fleet.position.z) as f64;
    let local_distance = (local_dx*local_dx + local_dy*local_dy + local_dz*local_dz).sqrt();
    
    fleet.position = target_pos.clone();
    fleet.last_move_distance = Some(local_distance);
    fleet.current_system_id = Some(system_id);

    for ship in &mut fleet.ships {
        ship.position = target_pos.clone();
    }
    
    let response = MoveFleetResponse {
        status: "success".to_string(),
        message: "Fleet moved successfully within system".to_string(),
        encounters: Vec::new(),
        current_position: target_pos.clone(),
        target_position: target_pos,
        remaining_distance: 0.0,
        current_system_id: Some(system_id),
    };

    Ok((response, fleet))
}

/// Handles fleet exit from a star system
/// 
/// # Arguments
/// * `fleet` - The fleet to move
/// * `target_pos` - Target position outside the system
/// * `system` - The current star system
/// * `system_max` - Maximum system boundary coordinate
/// 
/// # Returns
/// * `Ok((MoveFleetResponse, Fleet))` if exit successful
/// * `Err(String)` with descriptive error message if exit fails
fn handle_system_exit(
    mut fleet: Fleet,
    target_pos: &Position,
    system: &StarSystem,
    system_max: i32
) -> Result<(MoveFleetResponse, Fleet), String> {
    println!("Handling system exit:");
    println!("  Fleet position: ({}, {}, {})", fleet.position.x, fleet.position.y, fleet.position.z);
    println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
    println!("  System position: ({}, {}, {})", system.position.x, system.position.y, system.position.z);
    
    let start_pos = fleet.position.clone();
    
    // Calculate movement direction
    let dx = target_pos.x - start_pos.x;
    let dy = target_pos.y - start_pos.y;
    let dz = target_pos.z - start_pos.z;
    
    // Get the sign of each direction component (-1, 0, or 1)
    let dir_x = if dx != 0 { dx.signum() } else { 0 };
    let dir_y = if dy != 0 { dy.signum() } else { 0 };
    let dir_z = if dz != 0 { dz.signum() } else { 0 };
    
    // Calculate exit point just outside the system boundary
    // Use system absolute position + offset relative to system center
    let exit_x = system.position.x + dir_x * (system_max + 1);
    let exit_y = system.position.y + dir_y * (system_max + 1);
    let exit_z = system.position.z + dir_z * (system_max + 1);
    
    println!("Calculated exit position: ({}, {}, {})", exit_x, exit_y, exit_z);
    
    let exit_position = Position { x: exit_x, y: exit_y, z: exit_z };
    fleet.position = exit_position.clone();
    fleet.current_system_id = None;
    fleet.last_move_distance = Some(1.0); // Simplified distance for exit
    
    for ship in &mut fleet.ships {
        ship.position = exit_position.clone();
    }
    
    let response = MoveFleetResponse {
        status: "transition_exit".to_string(),
        message: format!("Fleet exited the star system at coordinates ({}, {}, {})", exit_x, exit_y, exit_z),
        encounters: Vec::new(),
        current_position: exit_position.clone(),
        target_position: exit_position, // Target is the exit point now
        remaining_distance: 0.0,
        current_system_id: None,
    };

    Ok((response, fleet))
}

/// Handles fleet movement in deep space
/// 
/// # Arguments
/// * `fleet` - The fleet to move
/// * `start_pos` - Starting position
/// * `target_pos` - Target position
/// * `game_world` - Reference to the game world containing all star systems
/// 
/// # Returns
/// * `Ok((MoveFleetResponse, Fleet))` if movement successful
/// * `Err(String)` with descriptive error message if movement fails
fn handle_deep_space_movement(
    mut fleet: Fleet,
    start_pos: Position,
    target_pos: Position,
    game_world: &[StarSystem]
) -> Result<(MoveFleetResponse, Fleet), String> {
    println!("Handling deep space movement:");
    println!("  Start position: ({}, {}, {})", start_pos.x, start_pos.y, start_pos.z);
    println!("  Target position: ({}, {}, {})", target_pos.x, target_pos.y, target_pos.z);
    println!("  Number of systems in game world: {}", game_world.len());
    
    let dx = (target_pos.x - start_pos.x) as f64;
    let dy = (target_pos.y - start_pos.y) as f64;
    let dz = (target_pos.z - start_pos.z) as f64;
    let distance = (dx*dx + dy*dy + dz*dz).sqrt();

    // Define system bounds for entry calculation
    let system_max = 100; // Fixed system size for testing

    // Check if target position matches any system's coordinates
    for (index, system) in game_world.iter().enumerate() {
        println!("Checking system {} at position ({}, {}, {})", 
                index, system.position.x, system.position.y, system.position.z);
        
        if target_pos.x == system.position.x && 
           target_pos.y == system.position.y && 
           target_pos.z == system.position.z {
            println!("Fleet entering System {} at coordinates ({}, {}, {})", 
                    index, target_pos.x, target_pos.y, target_pos.z);
            
            // Calculate entry point based on approach vector
            let approach_dx = (target_pos.x - start_pos.x) as f64;
            let approach_dy = (target_pos.y - start_pos.y) as f64;
            let approach_dz = (target_pos.z - start_pos.z) as f64;
            let approach_mag = (approach_dx * approach_dx + approach_dy * approach_dy + approach_dz * approach_dz).sqrt();
            
            let entry_point = if approach_mag > 1e-6 {
                let norm_dx = approach_dx / approach_mag;
                let norm_dy = approach_dy / approach_mag;
                let norm_dz = approach_dz / approach_mag;
                // Entry point is just inside the system boundary, relative to system center
                Position {
                    x: (system.position.x as f64 + norm_dx * system_max as f64).round() as i32,
                    y: (system.position.y as f64 + norm_dy * system_max as f64).round() as i32,
                    z: (system.position.z as f64 + norm_dz * system_max as f64).round() as i32,
                }
            } else {
                // Default entry if magnitude is zero (e.g., starting at system center)
                Position { x: system.position.x + 1, y: system.position.y, z: system.position.z }
            };

            println!("Calculated entry point: ({}, {}, {})", entry_point.x, entry_point.y, entry_point.z);

            fleet.position = entry_point.clone();
            fleet.current_system_id = Some(index);
            fleet.last_move_distance = Some(distance);
            
            for ship in &mut fleet.ships {
                ship.position = entry_point.clone();
            }

            let response = MoveFleetResponse {
                status: "transition_entry".to_string(),
                message: format!("Fleet entered System {} at coordinates ({}, {}, {})", 
                    index, entry_point.x, entry_point.y, entry_point.z),
                encounters: vec![],
                current_position: entry_point,
                target_position: target_pos,
                remaining_distance: 0.0,
                current_system_id: Some(index),
            };
            return Ok((response, fleet));
        }
    }

    // No system entry, complete the move in deep space
    println!("No system entry detected, completing deep space movement");
    fleet.position = target_pos.clone();
    fleet.current_system_id = None;
    fleet.last_move_distance = Some(distance);

    for ship in &mut fleet.ships {
        ship.position = target_pos.clone();
    }

    let response = MoveFleetResponse {
        status: "success".to_string(),
        message: "Fleet completed deep space movement successfully".to_string(),
        encounters: vec![],
        current_position: target_pos.clone(),
        target_position: target_pos,
        remaining_distance: 0.0,
        current_system_id: None,
    };
    Ok((response, fleet))
}

#[get("/fleet/owners")]
pub fn get_fleet_owners() -> Json<ApiResponse<Vec<String>>> {
    let result: Result<Vec<String>, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let fleets_dir = Path::new("data")
            .join("game")
            .join(&settings.game_id)
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
        Ok(owners_vec)
    })();

    match result {
        Ok(owners) => ApiResponse::success(owners, "Successfully retrieved fleet owners".to_string()),
        Err(e) => ApiResponse::error(e)
    }
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
                let settings = load_settings().expect("Failed to load settings");
                if let Ok(Some(system)) = crate::models::game_world::load_star_system(&settings.game_id, system_id) {
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

    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name)?;
        let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
        let trader_fleet_name = format!("Fleet_Trader_{}", fleet_number);

        // Load both fleets
        match (crate::models::fleet::load_fleet(&fleet_name), crate::models::fleet::load_fleet(&trader_fleet_name)) {
            (Ok(Some(mut player_fleet)), Ok(Some(mut trader_fleet))) => {
                match trade_with_fleet(&mut player_fleet, &mut trader_fleet, resource_type, quantity, &trade_type, &mut player) {
                    Ok(_) => {
                        // Save all changes
                        if let Err(e) = crate::models::fleet::save_fleet(&player_fleet) {
                            println!("Error saving player fleet: {}", e);
                            return Err("Error saving player fleet".to_string());
                        }

                        if let Err(e) = crate::models::fleet::save_fleet(&trader_fleet) {
                            println!("Error saving trader fleet: {}", e);
                            return Err("Error saving trader fleet".to_string());
                        }

                        let player_path = Path::new("data")
                            .join("game")
                            .join(&settings.game_id)
                            .join("players")
                            .join(format!("{}.json", settings.player_name));
                        if let Err(e) = save_json(&player_path, &player) {
                            println!("Error saving player: {}", e);
                            return Err("Error saving player".to_string());
                        }

                        Ok("Success".to_string())
                    },
                    Err(e) => Err(e)
                }
            },
            (Ok(Some(_)), Ok(None)) => Err("Trader fleet not found".to_string()),
            (Ok(None), _) => Err("Player fleet not found".to_string()),
            (Err(e), _) => {
                println!("Error loading player fleet: {}", e);
                Err("Error loading player fleet".to_string())
            },
            (_, Err(e)) => {
                println!("Error loading trader fleet: {}", e);
                Err("Error loading trader fleet".to_string())
            }
        }
    })();

    match result {
        Ok(message) => Json(message),
        Err(e) => Json(e)
    }
}

#[get("/games")]
pub fn list_games() -> Json<ApiResponse<Vec<SavedGame>>> {
    match SavedGame::list_saved_games() {
        Ok(games) => ApiResponse::success(games, "Successfully retrieved saved games".to_string()),
        Err(e) => ApiResponse::error(format!("Failed to load saved games: {}", e))
    }
}

#[post("/games/new", data = "<settings>")]
pub fn create_new_game(settings: Json<GameSettings>) -> Json<ApiResponse<String>> {
    println!("Starting create_new_game with settings: {:?}", settings);
    let mut settings = settings.into_inner();
    let game_id = settings.game_id.clone();
    let display_name = settings.display_name.clone();
    
    // Add required fields
    let now = Utc::now().to_rfc3339();
    settings.created_at = now.clone();
    settings.last_played = now;
    
    println!("Creating game directories for game_id: {}", game_id);
    // Create necessary directories
    let game_dir = Path::new("data").join("game").join(&game_id);
    let markets_dir = game_dir.join("markets");
    let fleets_dir = game_dir.join("fleets");
    let players_dir = game_dir.join("players");
    let factions_dir = game_dir.join("factions");
    let star_systems_dir = game_dir.join("star_systems");

    // Create all required directories
    for dir in [&game_dir, &markets_dir, &fleets_dir, &players_dir, &factions_dir, &star_systems_dir] {
        if let Err(e) = fs::create_dir_all(dir) {
            println!("Error creating directory {}: {}", dir.display(), e);
            return ApiResponse::error(format!("Failed to create game directories: {}", e));
        }
    }

    println!("Creating saved game entry");
    // Create a new saved game entry first
    let saved_game = SavedGame {
        game_id: game_id.clone(),
        display_name: display_name.clone(),
        created_at: settings.created_at.clone(),
        last_played: settings.last_played.clone(),
        settings: settings.clone(),
    };

    if let Err(e) = saved_game.save_game() {
        println!("Error saving game: {}", e);
        return ApiResponse::error("Failed to save game".to_string());
    }

    println!("Saving settings to game directory");
    // Save the settings in the game directory
    let settings_path = game_dir.join("settings.json");
    if let Err(e) = save_json(&settings_path, &settings) {
        println!("Error saving settings: {}", e);
        return ApiResponse::error("Failed to save settings".to_string());
    }

    // Update game state with current game ID
    let mut state = match crate::models::game_state::get_game_state() {
        Ok(state) => state,
        Err(e) => return ApiResponse::error(format!("Failed to get game state: {}", e)),
    };
    state.current_game_id = Some(game_id.clone());
    state.credits = settings.starting_credits;
    if let Err(e) = crate::models::game_state::save_game_state(state) {
        println!("Error updating game state: {}", e);
        return ApiResponse::error("Failed to update game state".to_string());
    }

    println!("Creating game world");
    // Create the game world with force_regenerate=true to ensure we create a new one
    let game_world = match crate::models::game_world::create_game_world_file(&settings, true) {
        Ok(world) => world,
        Err(e) => return ApiResponse::error(format!("Failed to create game world: {}", e)),
    };

    println!("Saving game world");
    if let Err(e) = crate::models::game_state::save_star_systems(&game_world) {
        println!("Error saving game world: {}", e);
        return ApiResponse::error("Failed to create game world".to_string());
    }

    println!("Updating global game world");
    // Update the global game world
    if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() {
        println!("Acquired GLOBAL_GAME_WORLD lock");
        *guard = game_world.clone();
        println!("Updated GLOBAL_GAME_WORLD with {} systems", guard.len());
    } else {
        println!("Failed to acquire GLOBAL_GAME_WORLD lock");
        return ApiResponse::error("Failed to update game world".to_string());
    }

    // Ensure the game world is properly initialized in memory
    let game_world = if let Ok(guard) = crate::GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        return ApiResponse::error("Failed to access game world".to_string());
    };

    println!("Starting market generation for {} systems", game_world.len());
    // Now that everything is set up, generate markets for all star systems
    for (system_id, _) in game_world.iter().enumerate() {
        println!("Generating markets for system {} at position {:?}", system_id, game_world[system_id].position);
        // Ensure the game world is loaded in memory
        if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() {
            *guard = game_world.clone();
        }
        if let Err(e) = regenerate_system_markets(system_id) {
            println!("Error generating markets for system {}: {}", system_id, e);
            return ApiResponse::error(format!("Failed to generate markets for system {}: {}", system_id, e));
        }
        println!("Completed market generation for system {}", system_id);
    }
    println!("Market generation completed for all systems");

    println!("Creating and saving factions");
    // Create and save factions from settings
    for faction_settings in &settings.factions {
        println!("Creating faction: {}", faction_settings.name);
        let faction = Faction::new(
            faction_settings.name.clone(),
            format!("The {} Empire", faction_settings.name) // Generate a basic description
        );
        if let Err(e) = save_faction(&faction) {
            println!("Error saving faction {}: {}", faction_settings.name, e);
            return ApiResponse::error(format!("Failed to save faction {}: {}", faction_settings.name, e));
        }

        // Create faction fleets - number of fleets scales with influence
        let fleet_count = 2 + (faction_settings.influence as usize / 30); // 2-5 fleets based on influence
        for fleet_num in 0..fleet_count {
            println!("Generating fleet {} for faction {}", fleet_num + 1, faction_settings.name);
            // Number of ships also scales with influence
            let ship_count = 1 + (faction_settings.influence as usize / 20); // 1-5 ships based on influence
            if let Ok(fleet) = generate_and_save_fleet(
                faction_settings.name.clone(),
                random_position(
                    settings.map_width as i32,
                    settings.map_height as i32,
                    settings.map_length as i32
                ),
                ship_count
            ) {
                println!("Generated fleet {} for faction {}: {}", fleet_num + 1, faction_settings.name, fleet.name);
            }
        }
    }

    println!("Creating special fleets");
    // Create special fleets (pirates, merchants, etc.)
    let special_fleets = vec![
        ("Pirate", 3),    // 3 pirate fleets
        ("Merchant", 3),  // 3 merchant fleets
        ("Military", 2),  // 2 military fleets
        ("Mercenary", 2), // 2 mercenary fleets
    ];

    for (fleet_type, count) in special_fleets {
        for fleet_num in 0..count {
            println!("Generating {} fleet {}", fleet_type, fleet_num + 1);
            if let Ok(fleet) = generate_and_save_fleet(
                format!("{}_{}", fleet_type, fleet_num + 1),
                random_position(
                    settings.map_width as i32,
                    settings.map_height as i32,
                    settings.map_length as i32
                ),
                rand::random::<usize>() % 3 + 1, // 1-3 ships
            ) {
                println!("Generated {} fleet {}: {}", fleet_type, fleet_num + 1, fleet.name);
            }
        }
    }

    println!("Creating player");
    // Create a new player with starting credits
    let player = Player::new(&settings.player_name, settings.starting_credits);
    
    println!("Saving player");
    // Save the player
    let player_path = game_dir.join("players").join(format!("{}.json", settings.player_name));
    
    if let Err(e) = save_json(&player_path, &player) {
        println!("Error saving player: {}", e);
        return ApiResponse::error(format!("Failed to save player: {}", e));
    }

    println!("Creating starting fleet");
    // Create a starting fleet for the player
    let mut player_fleet = Fleet::new(
        settings.player_name.clone(),
        random_position(
            settings.map_width as i32,
            settings.map_height as i32,
            settings.map_length as i32
        ),
        1
    );
    
    println!("Creating starting ship");
    // Create a starting ship for the player
    let mut starting_ship = Ship::new(
        ShipType::Freighter,
        ShipSize::Small,
        ShipEngine::Basic
    );
    starting_ship.position = player_fleet.position.clone();
    starting_ship.hp = 100; // Ensure full health
    starting_ship.name = format!("{}'s First Ship", settings.player_name);
    player_fleet.ships.push(starting_ship);
    
    println!("Saving player fleet");
    let fleet_path = game_dir.join("fleets").join(format!("Fleet_{}_{}.json", settings.player_name, 1));

    if let Err(e) = save_json(&fleet_path, &player_fleet) {
        println!("Error saving player fleet: {}", e);
        return ApiResponse::error(format!("Failed to save player fleet: {}", e));
    }

    println!("Game creation completed successfully");
    ApiResponse::success("Game created successfully".to_string(), "Success".to_string())
}

#[get("/games/<game_id>/load")]
pub fn load_game(game_id: String) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        // Load the saved game
        let saved_game = SavedGame::load_game(&game_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Game not found".to_string())?;
        
        // Load the game world
        let game_world = match crate::models::game_world::load_game_world(&game_id) {
            Ok(world) => world,
            Err(e) => return Err(format!("Failed to load game world: {}", e)),
        };

        // Update the global game world
        if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() {
            *guard = game_world;
        } else {
            return Err("Failed to update game world".to_string());
        }

        // Load the player data to get the current credits
        let player_path = Path::new("data")
            .join("game")
            .join(&game_id)
            .join("players")
            .join(format!("{}.json", saved_game.settings.player_name));

        let player_credits = if player_path.exists() {
            let file = std::fs::File::open(&player_path)
                .map_err(|e| format!("Failed to open player file: {}", e))?;
            let player: Player = serde_json::from_reader(file)
                .map_err(|e| format!("Failed to parse player data: {}", e))?;
            player.credits
        } else {
            saved_game.settings.starting_credits
        };

        // Update the game state with the current game ID and credits
        let mut state = crate::models::game_state::get_game_state()
            .map_err(|e| format!("Failed to get game state: {}", e))?;
        state.current_game_id = Some(game_id.clone());
        state.credits = player_credits;
        crate::models::game_state::save_game_state(state)
            .map_err(|e| format!("Failed to save game state: {}", e))?;

        Ok("Game loaded successfully".to_string())
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Success".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[get("/settings")]
pub fn get_settings() -> Json<ApiResponse<GameSettings>> {
    // First try to load settings from the game directory
    match load_settings() {
        Ok(settings) => ApiResponse::success(settings, "Successfully retrieved settings".to_string()),
        Err(_) => ApiResponse::error("No active game found".to_string())
    }
}

#[post("/settings", data = "<settings>")]
pub fn update_settings(settings: Json<GameSettings>) -> Json<ApiResponse<String>> {
    let settings = settings.into_inner();
    
    // Load the specific saved game using the game_id from the settings
    match SavedGame::load_game(&settings.game_id) {
        Ok(Some(mut saved_game)) => {
            saved_game.settings = settings;
            match saved_game.save_game() {
                Ok(_) => ApiResponse::success("Settings updated successfully".to_string(), "Success".to_string()),
                Err(e) => ApiResponse::error(format!("Failed to save settings: {}", e))
            }
        }
        Ok(None) => ApiResponse::error("Game not found".to_string()),
        Err(e) => ApiResponse::error(format!("Failed to load game: {}", e))
    }
}

#[delete("/games/<game_id>")]
pub fn delete_game(game_id: String) -> Json<ApiResponse<String>> {
    // Delete the game directory and all its contents
    let game_dir = Path::new("data").join("game").join(&game_id);
    let save_file = Path::new("data").join("saves").join(format!("{}.json", game_id));

    // Remove the save file
    if save_file.exists() {
        if let Err(e) = fs::remove_file(&save_file) {
            return ApiResponse::error(format!("Failed to delete save file: {}", e));
        }
    }

    // Remove the game directory and all its contents
    if game_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&game_dir) {
            return ApiResponse::error(format!("Failed to delete game directory: {}", e));
        }
    }

    ApiResponse::success("Game deleted successfully".to_string(), "Success".to_string())
}

#[post("/planet/<system_id>/<planet_id>/buy_ship", format = "json", data = "<data>")]
pub fn buy_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("ships_{}_{}.json", system_id, planet_id));

        println!("Buy ship request: system={}, planet={}, ship_index={}, fleet_name={:?}", 
                 system_id, planet_id, data.ship_index, data.fleet_name);
        println!("Player credits before purchase: {}", player.credits);

        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        if data.ship_index >= ship_market.ships.len() {
            return Err(format!("Invalid ship index: {} (market has {} ships)", 
                              data.ship_index, ship_market.ships.len()));
        }

        let ship = &ship_market.ships[data.ship_index];
        if let Some(price) = ship.price {
            if player.credits >= price {
                player.credits -= price;
                println!("Player credits after purchase: {}", player.credits);
                
                let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
                let mut fleet = crate::models::fleet::load_fleet(&fleet_name)
                    .map_err(|e| e.to_string())?
                    .ok_or_else(|| "Fleet not found".to_string())?;
                
                println!("Adding ship {} to fleet {}", ship.name, fleet.name);
                fleet.ships.push(ship.clone());
                
                // Remove the ship from the market
                ship_market.ships.remove(data.ship_index);
                
                // Save all changes
                save_json(&market_path, &ship_market).map_err(|e| e.to_string())?;
                if let Err(e) = player.save() {
                    println!("Error saving player data: {}", e);
                    return Err("Failed to save player data".to_string());
                }
                crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
                
                Ok(format!("Successfully bought ship for {} credits", price))
            } else {
                Err(format!("Not enough credits: need {} but have {}", price, player.credits))
            }
        } else {
            Err("Ship is not for sale".to_string())
        }
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Ship purchase completed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/planet/<system_id>/<planet_id>/sell_ship", format = "json", data = "<data>")]
pub fn sell_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("ships_{}_{}.json", system_id, planet_id));

        println!("Sell ship request: system={}, planet={}, ship_index={}, fleet_name={:?}", 
                 system_id, planet_id, data.ship_index, data.fleet_name);
        println!("Player credits before sale: {}", player.credits);

        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
        let mut fleet = crate::models::fleet::load_fleet(&fleet_name)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Fleet not found: {}", fleet_name))?;
        
        if data.ship_index >= fleet.ships.len() {
            return Err(format!("Invalid ship index: {} (fleet has {} ships)", 
                              data.ship_index, fleet.ships.len()));
        }

        let ship = fleet.ships[data.ship_index].clone();
        let price = calculate_ship_price(&ship) * 0.7; // Sell at 70% of purchase price
        
        println!("Selling ship: {} for {} credits", ship.name, price);
        player.credits += price;
        println!("Player credits after sale: {}", player.credits);
        
        // Add ship to market
        let mut market_ship = ship.clone();
        market_ship.price = Some(price);
        ship_market.ships.push(market_ship);
        
        // Remove ship from fleet
        fleet.ships.remove(data.ship_index);
        
        // Save all changes
        save_json(&market_path, &ship_market).map_err(|e| e.to_string())?;
        if let Err(e) = player.save() {
            println!("Error saving player data: {}", e);
            return Err("Failed to save player data".to_string());
        }
        crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
        
        Ok(format!("Successfully sold ship for {} credits", price))
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Ship sale completed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/planet/<system_id>/<planet_id>/trade_in_ship", format = "json", data = "<data>")]
pub fn trade_in_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeInData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data")
            .join("game")
            .join(&settings.game_id)
            .join("markets")
            .join(format!("ships_{}_{}.json", system_id, planet_id));

        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        if data.ship_index >= ship_market.ships.len() {
            return Err(format!("Invalid ship index: {} (market has {} ships)", 
                              data.ship_index, ship_market.ships.len()));
        }

        // Clone the ship before using it, to avoid borrowing issues
        let new_ship = ship_market.ships[data.ship_index].clone();
        
        // Check if the ship has a price
        if let Some(price) = new_ship.price {
            let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
            let mut fleet = crate::models::fleet::load_fleet(&fleet_name)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Fleet not found".to_string())?;

            // Calculate trade-in value if a ship is being traded in
            let mut trade_in_value = 0.0;
            if let Some(trade_in_index) = data.trade_in_ship_index {
                if trade_in_index >= fleet.ships.len() {
                    return Err(format!("Invalid trade-in ship index: {} (fleet has {} ships)", 
                                      trade_in_index, fleet.ships.len()));
                }
                let trade_in_ship = fleet.ships[trade_in_index].clone(); // Clone here to keep a copy
                trade_in_value = calculate_ship_price(&trade_in_ship) * 0.7; // 70% of value for trade-in
                
                // Add the traded-in ship to the market
                let mut market_ship = trade_in_ship.clone();
                market_ship.price = Some(calculate_ship_price(&trade_in_ship) * 0.8); // Set market price at 80% of full value
                ship_market.ships.push(market_ship);
                
                println!("Added trade-in ship to market: {}", trade_in_ship.name);
                
                // Remove the traded-in ship from the fleet
                fleet.ships.remove(trade_in_index);
            }

            // Calculate final price after trade-in
            let final_price = price - trade_in_value;
            println!("Trade calculation: Market price {} - Trade-in value {} = Final price {}", 
                    price, trade_in_value, final_price);
            
            if player.credits >= final_price {
                player.credits -= final_price;
                println!("Player credits after trade: {}", player.credits);
                
                // Add the new ship to the fleet
                fleet.ships.push(new_ship);
                
                // Remove the ship from the market
                ship_market.ships.remove(data.ship_index);
                
                // Save all changes
                save_json(&market_path, &ship_market).map_err(|e| e.to_string())?;
                if let Err(e) = player.save() {
                    println!("Error saving player data: {}", e);
                    return Err("Failed to save player data".to_string());
                }
                crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
                
                Ok(format!("Successfully traded in ship for {} credits", final_price))
            } else {
                Err(format!("Not enough credits: need {} but have {}", final_price, player.credits))
            }
        } else {
            Err("Ship is not for sale".to_string())
        }
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Ship trade-in completed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[get("/fleets")]
pub fn get_player_fleets() -> Json<ApiResponse<Vec<Fleet>>> {
    let settings = match load_settings() {
        Ok(settings) => settings,
        Err(e) => {
            println!("Error loading settings: {}", e);
            return ApiResponse::error("Failed to load game settings".to_string());
        }
    };

    println!("Loading fleets for player: {}", settings.player_name);
    
    // First load the player to get their fleet list
    let player = match load_player(&settings.player_name) {
        Ok(player) => player,
        Err(e) => {
            println!("Error loading player: {}", e);
            return ApiResponse::error(format!("Failed to load player: {}", e));
        }
    };

    // Load each fleet from the player's fleet list
    let mut fleets = Vec::new();
    for fleet_name in &player.fleets {
        match load_fleet(fleet_name) {
            Ok(fleet) => {
                println!("Loaded fleet: {} with {} ships", fleet.name, fleet.ships.len());
                fleets.push(fleet);
            },
            Err(e) => {
                println!("Error loading fleet {}: {}", fleet_name, e);
                // Continue loading other fleets even if one fails
                continue;
            }
        }
    }

    if fleets.is_empty() {
        println!("No fleets found for player");
        return ApiResponse::error("No fleets found".to_string());
    }

    ApiResponse::success(fleets, "Successfully loaded fleets".to_string())
}

#[post("/player/<name>/add_credits", format = "json", data = "<amount>")]
pub fn add_credits(name: &str, amount: Json<f32>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let mut player = load_player(name).map_err(|e| e.to_string())?;
        player.credits += *amount;
        if let Err(e) = player.save() {
            println!("Error saving player data: {}", e);
            return Err("Failed to save player data".to_string());
        }
        Ok(format!("Successfully added {} credits", *amount))
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Credits added successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/player/<name>/remove_credits", format = "json", data = "<amount>")]
pub fn remove_credits(name: &str, amount: Json<f32>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let mut player = load_player(name).map_err(|e| e.to_string())?;
        if player.credits < *amount {
            return Err(format!("Not enough credits: need {} but have {}", *amount, player.credits));
        }
        player.credits -= *amount;
        if let Err(e) = player.save() {
            println!("Error saving player data: {}", e);
            return Err("Failed to save player data".to_string());
        }
        Ok(format!("Successfully removed {} credits", *amount))
    })();

    match result {
        Ok(message) => ApiResponse::success(message, "Credits removed successfully".to_string()),
        Err(e) => ApiResponse::error(e)
    }
}

#[post("/clear-caches")]
pub fn clear_caches() -> Json<ApiResponse<String>> {
    // Clear all relevant caches
    crate::models::game_state::SYSTEM_CACHE.remove_all();
    crate::models::game_state::FLEET_CACHE.remove_all();
    crate::models::game_state::MARKET_CACHE.remove_all();
    ApiResponse::success("Caches cleared successfully".to_string(), "Success".to_string())
}

fn save_fleet_model(fleet: &Fleet) -> Result<(), String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let fleet_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("fleets")
        .join(format!("{}.json", fleet.name));

    if let Some(parent) = fleet_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create fleet directory: {}", e))?;
    }

    let file = std::fs::File::create(fleet_path)
        .map_err(|e| format!("Failed to create fleet file: {}", e))?;
    
    serde_json::to_writer(file, fleet)
        .map_err(|e| format!("Failed to write fleet data: {}", e))?;
    
    Ok(())
}

/// Handles fleet movement requests, managing both system and deep space movement
/// 
/// # Arguments
/// * `owner_id` - ID of the fleet owner
/// * `fleet_number` - Number of the fleet to move
/// * `data` - Movement data containing target coordinates
/// 
/// # Returns
/// * JSON response with movement result or error message
#[post("/fleet/<owner_id>/<fleet_number>/move", format = "json", data = "<data>")]
pub fn move_fleet(owner_id: String, fleet_number: usize, data: Json<MoveFleetData>) -> Json<ApiResponse<MoveFleetResponse>> {
    println!("--- Starting Fleet Movement Operation ---");
    println!("  Fleet: Fleet_{}_{}", owner_id, fleet_number);
    println!("  Target Position: ({}, {}, {})", data.x, data.y, data.z);
    
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    
    // Wrap the core logic in a block to handle potential errors and save the fleet at the end
    let result: Result<MoveFleetResponse, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let game_world = if settings.game_id == "test_game" {
            println!("  Running in test mode, loading test game world...");
            let system_path = crate::models::game_state::game_data_path(&settings.game_id, &["star_systems", "Star_System_0.json"]);
            match crate::models::game_state::load_json::<StarSystem>(&system_path) {
                Ok(system) => vec![system],
                Err(e) => {
                    println!("  Error loading test system: {}. Using empty world.", e);
                    Vec::new()
                }
            }
        } else {
            get_global_game_world()
        };
        println!("  Game world contains {} systems", game_world.len());

        let initial_fleet = match crate::models::fleet::load_fleet(&fleet_name) {
            Ok(Some(fleet)) => fleet,
            Ok(None) => return Err(format!("Fleet '{}' not found", fleet_name)),
            Err(e) => return Err(format!("Failed to load fleet: {}", e)),
        };
        println!("  Loaded fleet at position ({}, {}, {})", initial_fleet.position.x, initial_fleet.position.y, initial_fleet.position.z);

        let target_pos = Position { x: data.x, y: data.y, z: data.z };
        let start_pos = initial_fleet.position.clone();
        validate_galaxy_bounds(&target_pos, &settings)?;

        // Perform movement logic, getting back the response and the potentially updated fleet
        let (response, updated_fleet) = if let Some(system_id) = initial_fleet.current_system_id {
            println!("  Fleet is in system {}", system_id);
            if system_id >= game_world.len() {
                return Err(format!("Invalid system ID: {} (Game world len: {})", system_id, game_world.len()));
            }
            let system = &game_world[system_id];
            // Pass ownership of initial_fleet to the handler
            handle_system_movement(initial_fleet, target_pos, system, system_id)? 
        } else {
            println!("  Fleet is in deep space");
            // Pass ownership of initial_fleet to the handler
            handle_deep_space_movement(initial_fleet, start_pos, target_pos, &game_world)?
        };

        // Save the final state of the fleet *after* successful movement
        println!("Saving final fleet state for {}", updated_fleet.name);
        save_fleet(&updated_fleet)?;
        println!("Fleet saved successfully.");

        // Return the response part of the result
        Ok(response)
    })();

    // Handle the final result (Ok or Err)
    match result {
        Ok(response) => {
            let message = response.message.clone();
            println!("  Movement successful: {}", message);
            ApiResponse::success(response, message)
        },
        Err(e) => {
            println!("  Movement failed: {}", e);
            ApiResponse::error(e)
        }
    }
} 