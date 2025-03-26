use crate::models::ship::ship::Ship;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fleet {
    pub name: String,
    pub owner_id: String,
    pub owner_type: OwnerType,
    pub ships: Vec<Ship>,
    pub location: String, // Star system ID where the fleet is located
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OwnerType {
    Player,
    Planet,
    Faction,
}

impl fmt::Display for OwnerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnerType::Player => write!(f, "Player"),
            OwnerType::Planet => write!(f, "Planet"),
            OwnerType::Faction => write!(f, "Faction"),
        }
    }
}

impl Fleet {
    pub fn new(owner_id: String, owner_type: OwnerType, location: String) -> Self {
        Fleet {
            name: format!("Fleet_{}_{}", owner_type.to_string(), owner_id),
            owner_id,
            owner_type,
            ships: Vec::new(),
            location,
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
} 