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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    pub name: String,
    pub owner_id: String,
    pub ships: Vec<Ship>,
    pub position: Position,
    pub current_system_id: Option<usize>, // Optional because fleet might be between systems
    pub last_move_distance: Option<f64>,
}

impl Fleet {
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

    pub fn add_ship(&mut self, ship: Ship) {
        self.ships.push(ship);
    }

    pub fn remove_ship(&mut self, ship_name: &str) -> Option<Ship> {
        if let Some(index) = self.ships.iter().position(|s| s.name == ship_name) {
            Some(self.ships.remove(index))
        } else {
            None
        }
    }

    pub fn update_position(&mut self, new_position: Position) {
        self.position = new_position.clone();
        // Update all ships' positions to match fleet position
        for ship in &mut self.ships {
            ship.position = new_position.clone();
        }
    }

    pub fn load(path: &str) -> std::io::Result<Fleet> {
        let file = File::open(path)?;
        let fleet: Fleet = serde_json::from_reader(file)?;
        Ok(fleet)
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    // Reverted to radius-based check
    pub fn check_star_system_transition(&self, position: &Position, game_world: &[StarSystem]) -> (Option<usize>, bool) {
        println!("--- Checking Radius Transition for Fleet '{}' at Pos({},{},{}) --- Current Fleet System ID: {:?}", 
                 self.name, position.x, position.y, position.z, self.current_system_id);

        // Check if we're already in a system
        if let Some(system_id) = self.current_system_id {
            // If we're in a system, check if we're leaving it
            if system_id < game_world.len() {
                let system = &game_world[system_id];
                println!("  Fleet is in system {}. Checking exit.", system_id);
                println!("  System {} Center: ({},{},{}), Radius: {}", system_id, system.position.x, system.position.y, system.position.z, system.radius);
                let dx = (position.x - system.position.x) as f64;
                let dy = (position.y - system.position.y) as f64;
                let dz = (position.z - system.position.z) as f64;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();
                
                println!("  Calculated distance from current system center: {:.4}, System Radius: {:.4}", distance, system.radius);
                
                // If we're moving outside the radius
                if distance > system.radius {
                    println!("  >>> Result: Leaving System {} <<< Return: (None, true)", system_id);
                    return (None, true); // Left the system
                } else {
                    // If we didn't leave, we are still inside the current system. No transition.
                    println!("  >>> Result: Still inside System {} <<< Return: (Some({}), false)", system_id, system_id);
                    return (Some(system_id), false); // Still inside the same system, no transition
                }
            } else {
                println!("  Warning: Fleet system ID {} is out of bounds for game_world (len {}). Treating as None.", system_id, game_world.len());
                // Fall through to check for entry into other systems below
            }
        }

        // If we're not in a system (current_system_id is None), check if we're entering one
        println!("  Fleet is in Deep Space (System ID None). Checking for entry into any system.");
        for (index, system) in game_world.iter().enumerate() {
            println!("  Checking against System {}: Center({},{},{}), Radius: {}", 
                    index, system.position.x, system.position.y, system.position.z, system.radius);
            let dx = (position.x - system.position.x) as f64;
            let dy = (position.y - system.position.y) as f64;
            let dz = (position.z - system.position.z) as f64;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();
            
            println!("  Calculated distance from system {} center: {:.4}", index, distance);
            
            // If we enter the radius
            if distance <= system.radius {
                println!("  >>> Result: Entering System {} <<< Return: (Some({}), true)", index, index);
                return (Some(index), true); // Entered a system
            }
        }

        // Did not enter any system and wasn't already in one.
        println!("  >>> Result: Still in Deep Space <<< Return: (None, false)");
        (None, false) // Still in deep space, no transition
    }

    // This function is intended for movement *after* a fleet is already confirmed to be inside a system.
    pub fn move_within_system(&mut self, target_x: i32, target_y: i32, target_z: i32, system: &StarSystem) -> Result<(), String> {
        // Only allow movement if we're in a system
        if self.current_system_id.is_none() {
            return Err("Cannot move within system: fleet is not in a star system".to_string());
        }

        // Calculate the distance from the system center
        let dx = (target_x - system.position.x) as f64;
        let dy = (target_y - system.position.y) as f64;
        let dz = (target_z - system.position.z) as f64;
        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

        println!("Fleet {} attempting to move within system. Distance from center: {}, System radius: {}", 
            self.name, distance, system.radius);

        if distance > system.radius {
            return Err(format!("Target position is outside the star system's bounds (max distance: {})", system.radius));
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

pub fn load_fleet(fleet_name: &str) -> Result<Option<Fleet>, String> {
    let settings = load_settings().map_err(|e| e.to_string())?;
    let fleet_path = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("fleets")
        .join(format!("{}.json", fleet_name));

    if !fleet_path.exists() {
        return Ok(None);
    }

    let file = std::fs::File::open(fleet_path)
        .map_err(|e| format!("Failed to open fleet file: {}", e))?;
    
    let fleet: Fleet = serde_json::from_reader(file)
        .map_err(|e| format!("Failed to parse fleet data: {}", e))?;
    
    Ok(Some(fleet))
}

pub fn save_fleet(fleet: &Fleet) -> Result<(), String> {
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

#[derive(Serialize)]
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

pub fn is_within_local_bounds(position: &Position, settings: &GameSettings) -> bool {
    let max_coord = settings.map_width as i32;
    let min_coord = -max_coord;
    
    position.x >= min_coord && position.x <= max_coord &&
    position.y >= min_coord && position.y <= max_coord &&
    position.z >= min_coord && position.z <= max_coord
} 