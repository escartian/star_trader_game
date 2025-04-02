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
use crate::constants::GAME_ID;

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
        let mut contents = String::new();
        File::open(path)?.read_to_string(&mut contents)?;
        let fleet: Fleet = serde_json::from_str(&contents)?;
        Ok(fleet)
    }

    pub fn save(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }
}

pub fn get_next_fleet_number(owner_id: &str) -> std::io::Result<usize> {
    let fleets_dir = Path::new("data")
        .join("game")
        .join(GAME_ID)
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
    let fleets_dir = Path::new("data")
        .join("game")
        .join(GAME_ID)
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
                    if let Ok(Some(fleet)) = load_fleet(file_name.split('.').next().unwrap_or("")) {
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
    let fleet_number = get_next_fleet_number(&owner_id)?;
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    // Generate and save new fleet
    let fleet = generate_random_fleet(owner_id, position, ship_count, fleet_number);
    save_fleet(&fleet)?;
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

pub fn save_fleet(fleet: &Fleet) -> std::io::Result<()> {
    let data_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("fleets")
        .join(format!("{}.json", fleet.name));

    // Create the necessary directories if they don't exist
    if let Some(parent) = data_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write the fleet to a JSON file
    let file = File::create(data_path)?;
    serde_json::to_writer(file, fleet)?;
    
    Ok(())
}

pub fn load_fleet(fleet_name: &str) -> std::io::Result<Option<Fleet>> {
    let data_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("fleets")
        .join(format!("{}.json", fleet_name));

    println!("Attempting to load fleet from: {:?}", data_path);

    if !data_path.exists() {
        println!("Fleet file does not exist at: {:?}", data_path);
        return Ok(None);
    }

    let mut contents = String::new();
    File::open(data_path)?.read_to_string(&mut contents)?;
    
    let fleet: Fleet = serde_json::from_str(&contents)?;
    println!("Successfully loaded fleet: {}", fleet.name);
    Ok(Some(fleet))
} 