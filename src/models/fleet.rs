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