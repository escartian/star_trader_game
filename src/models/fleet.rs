use crate::models::ship::ship::Ship;
use crate::models::star_system::StarSystem;
use crate::models::position::Position;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use rand::random;
use crate::models::settings::load_settings;
use crate::encounters::EncounterFleet;
use crate::models::settings::GameSettings;
use crate::models::game_state::game_data_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    pub name: String,
    pub owner_id: String,
    pub ships: Vec<Ship>,
    pub position: Position,
    pub current_system_id: Option<usize>, // Optional because fleet might be between systems
    pub last_move_distance: Option<f64>,
}

#[derive(Serialize, Debug)]
pub struct MoveFleetResponse {
    pub status: String,
    pub message: String,
    pub encounters: Vec<EncounterFleet>,
    pub current_position: Position,
    pub target_position: Position,
    pub remaining_distance: f64,
    pub current_system_id: Option<usize>,
}

#[derive(Deserialize)]
pub struct MoveFleetData {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Fleet {
    /// Construct a new `Fleet` instance with the given `owner_id`, `position`, and `fleet_number`.
    ///
    /// The `fleet_number` is used to generate a unique name for the fleet in the format
    /// "Fleet_<owner_id>_<fleet_number>".
    ///
    /// The `current_system_id` and `last_move_distance` fields are initialized to `None` and `0.0`
    /// respectively.
    pub fn new(owner_id: String, position: Position, fleet_number: usize) -> Self {
        Fleet {
            name: format!("Fleet_{}_{}", owner_id, fleet_number),
            owner_id,
            ships: Vec::new(),
            position,
            current_system_id: None,
            last_move_distance: None,
        }
    }

    /// Adds a new ship to the fleet's roster.
    ///
    /// This function appends the provided `Ship` to the fleet's list of ships,
    /// thereby making it a part of the fleet. This is typically used when a new
    /// ship is constructed, purchased, or otherwise acquired by the fleet.
    ///
    /// # Arguments
    ///
    /// * `ship` - The `Ship` instance to be added to the fleet.
    pub fn add_ship(&mut self, ship: Ship) {
        self.ships.push(ship);
    }

    /// Removes the ship with the given `ship_name` from the fleet and returns it.
    ///
    /// * If no ship with the given name is found in the fleet, the function returns `None`.
    /// * This is useful for when a ship is destroyed or removed from the fleet.
    pub fn remove_ship(&mut self, ship_name: &str) -> Option<Ship> {
        if let Some(index) = self.ships.iter().position(|s| s.name == ship_name) {
            Some(self.ships.remove(index))
        } else {
            None
        }
    }

    /// Updates the position of the fleet to the given `Position` and
    /// propagates the change to all ships in the fleet. This is useful
    /// for when the fleet moves to a new position, for example when the
    /// player issues a "move to" order to the fleet.
    pub fn update_position(&mut self, new_position: Position) {
        self.position = new_position.clone();
        // Update all ships' positions to match fleet position
        for ship in &mut self.ships {
            ship.position = new_position.clone();
        }
    }

    /// Loads a fleet from a JSON file at the specified path.
    ///
    /// The function returns an `std::io::Result` containing the loaded fleet.
    /// If the file does not exist or if there is an error while reading the file,
    /// the function returns an `std::io::Error`.
    pub fn load(path: &str) -> std::io::Result<Fleet> {
        let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let fleet_path = game_data_path(&settings.game_id, &["fleets", &format!("{}.json", path)]);

        let file = File::open(fleet_path)?;
        let fleet: Fleet = serde_json::from_reader(file)?;
        Ok(fleet)
    }

    /// Saves the fleet to a JSON file at the specified path.
    ///
    /// The function creates the necessary directories and writes the fleet
    /// data in JSON format to the file. If the operation is successful,
    /// it returns `Ok(())`. Otherwise, it returns an `std::io::Error`.
    ///
    /// # Arguments
    ///
    /// * `path` - A string slice that specifies the path where the fleet
    ///   should be saved. This path is relative to the game's data directory.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is an issue creating the
    /// directories, creating the file, or writing the fleet data to the file.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let fleet_path = game_data_path(&settings.game_id, &["fleets", &format!("{}.json", path)]);

        if let Some(parent) = fleet_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = File::create(fleet_path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    
    /// Move the fleet within the given star system to the target position.
    ///
    /// # Arguments
    /// * `target_x` - Target x coordinate
    /// * `target_y` - Target y coordinate
    /// * `target_z` - Target z coordinate
    /// * `system` - The star system to move within
    ///
    /// # Errors
    /// If the target position is outside the star system's bounds, or if the fleet is not currently in a system, an error is returned.
    ///
    /// # Notes
    /// This function is intended for movement *after* a fleet is already confirmed to be inside a system.
    pub fn move_within_system(&mut self, target_x: i32, target_y: i32, target_z: i32, system: &StarSystem) -> Result<(), String> {
        // Only allow movement if we're in a system
        if self.current_system_id.is_none() {
            return Err("Cannot move within system: fleet is not in a star system".to_string());
        }

        // Check if target is within system boundaries
        let settings = load_settings().map_err(|e| e.to_string())?;
        let max_coord = settings.map_width as i32;
        let min_coord = -max_coord;
        
        if target_x < min_coord || target_x > max_coord ||
           target_y < min_coord || target_y > max_coord ||
           target_z < min_coord || target_z > max_coord {
            return Err(format!("Target position is outside the star system's bounds (must be between {} and {})", 
                             min_coord, max_coord));
        }

        // Update position
        self.position = Position { x: target_x, y: target_y, z: target_z };
        
        // Update all ships' positions
        for ship in &mut self.ships {
            ship.position = self.position.clone();
        }

        Ok(())
    }
}

/// Finds the next available fleet number for the given owner ID. This function
/// will return the lowest available number that doesn't conflict with any existing
/// fleet file name. If no fleet files exist, the function will return 1.
///
/// # Errors
///
/// If there is an IO error while reading the directory, an error will be returned.
pub fn get_next_fleet_number(owner_id: &str) -> std::io::Result<usize> {
    let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let fleets_dir = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("fleets");

    if !fleets_dir.exists() {
        return Ok(1);
    }

    let mut max_number = 0;
    for entry in fs::read_dir(fleets_dir)? {
        if let Ok(entry) = entry {
            if let Some(file_name) = entry.file_name().to_str() {
                let prefix = format!("Fleet_{}", owner_id);
                if file_name.starts_with(&prefix) {
                    if let Some(number) = file_name.split('_').last().and_then(|n| n.split('.').next()) {
                        if let Ok(num) = number.parse::<usize>() {
                            max_number = max_number.max(num);
                        }
                    }
                }
            }
        }
    }
    Ok(max_number + 1)
}

/// Lists all fleets for a given owner ID. This function will return a vector of all
/// `Fleet` objects that have a file name that starts with "Fleet_<owner_id>_".
///
/// # Errors
///
/// If there is an IO error while reading the directory, an error will be returned.
/// If any of the fleet files are not valid JSON, they will be skipped and not included
/// in the returned vector.
pub fn list_owner_fleets(owner_id: &str) -> std::io::Result<Vec<Fleet>> {
    let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let fleets_dir = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("fleets");

    if !fleets_dir.exists() {
        return Ok(Vec::new());
    }

    let mut fleets = Vec::new();
    let prefix = format!("Fleet_{}_", owner_id);
    
    for entry in fs::read_dir(fleets_dir)? {
        if let Ok(entry) = entry {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.starts_with(&prefix) {
                    let fleet_name = file_name.trim_end_matches(".json");
                    if let Ok(Some(fleet)) = load_fleet(fleet_name) {
                        fleets.push(fleet);
                    }
                }
            }
        }
    }
    println!("Fleets for owner {}: {:?}", owner_id, fleets);
    Ok(fleets)
}

/// Generates a new random fleet for the given `owner_id` and `ship_count`, saves it to a file, and returns the generated `Fleet` object.
///
/// The file name is in the format "Fleet_<owner_id>_<fleet_number>.json", where `<fleet_number>` is a number starting from 1 and incrementing for each new fleet for the given `owner_id`.
///
/// # Errors
///
/// If there is an IO error while creating the file or writing to it, an error will be returned. If there is an error while generating the fleet, an error will be returned.
pub fn generate_and_save_fleet(owner_id: String, position: Position, ship_count: usize) -> std::io::Result<Fleet> {
    let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let fleet_number = get_next_fleet_number(&owner_id)?;
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    
    // Generate new fleet
    let fleet = generate_random_fleet(owner_id, position, ship_count, fleet_number);
    
    // Save the fleet
    let fleet_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("fleets")
        .join(format!("{}.json", fleet_name));

    if let Some(parent) = fleet_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(fleet_path)?;
    serde_json::to_writer(file, &fleet)?;
    
    Ok(fleet)
}

/// Generates a new random `Fleet` with the given `owner_id` and `ship_count` at the given `position` and `fleet_number`.
///
/// The generated `Fleet` will have the given `owner_id` and `fleet_number`, and will be positioned at the given `position`.
///
/// The generated `Fleet` will have `ship_count` number of ships, each randomly generated.
///
/// # Returns
///
/// The generated `Fleet` object.
pub fn generate_random_fleet(owner_id: String, position: Position, ship_count: usize, fleet_number: usize) -> Fleet {
    let mut fleet = Fleet::new(owner_id, position.clone(), fleet_number);
    
    for _ in 0..ship_count {
        let mut ship = random::<Ship>();
        ship.owner = fleet.owner_id.clone();
        ship.position = position.clone();
        fleet.add_ship(ship);
    }
    
    fleet
}

/// Loads a fleet from a JSON file given its name.
///
/// This function attempts to load a fleet with the specified `fleet_name` from the 
/// game's data directory. The fleet data is expected to be in JSON format. If the 
/// fleet file does not exist, it returns `Ok(None)`. If the fleet is successfully 
/// loaded, it returns `Ok(Some(Fleet))`. Any errors encountered during loading, 
/// such as file access or JSON parsing errors, are returned as a `String`.
///
/// # Arguments
///
/// * `fleet_name` - The name of the fleet to be loaded.
///
/// # Returns
///
/// A `Result` which is:
/// * `Ok(Some(Fleet))` if the fleet is successfully loaded.
/// * `Ok(None)` if the fleet file does not exist.
/// * `Err(String)` if there is an error accessing the file or parsing the JSON.
pub fn load_fleet(fleet_name: &str) -> Result<Option<Fleet>, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let fleet_path = game_data_path(&settings.game_id, &["fleets", &format!("{}.json", fleet_name)]);

    if !fleet_path.exists() {
        return Ok(None);
    }

    let file = std::fs::File::open(fleet_path)
        .map_err(|e| format!("Failed to open fleet file: {}", e))?;
    
    let fleet: Fleet = serde_json::from_reader(file)
        .map_err(|e| format!("Failed to parse fleet data: {}", e))?;
    
    Ok(Some(fleet))
}

/// Saves a `Fleet` object to a JSON file at the specified path.
///
/// The file name is in the format "<fleet_name>.json", where `<fleet_name>` is the name of the fleet.
///
/// # Errors
///
/// If there is an IO error while creating the file or writing to it, an error will be returned. If there is an error while generating the fleet, an error will be returned.
pub fn save_fleet(fleet: &Fleet) -> Result<(), String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let fleet_path = game_data_path(&settings.game_id, &["fleets", &format!("{}.json", fleet.name)]);

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

/// Checks if a position is within the local bounds of a star system.
/// 
/// The local bounds of a star system are considered to be the cube of size
/// `map_width` centered at the origin (0,0,0). Positions within this cube are
/// considered to be within the local bounds of the star system.
///
/// # Arguments
/// * `position` - The position to check
/// * `settings` - Game settings containing the map width
/// 
/// # Returns
/// `true` if the position is within local bounds, `false` otherwise.
pub fn is_within_local_bounds(position: &Position, settings: &GameSettings) -> bool {
    let max_coord = settings.map_width as i32;
    let min_coord = -max_coord;
    
    position.x >= min_coord && position.x <= max_coord &&
    position.y >= min_coord && position.y <= max_coord &&
    position.z >= min_coord && position.z <= max_coord
} 