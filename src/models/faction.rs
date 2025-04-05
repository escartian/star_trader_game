use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Read;
use std::path::Path;
use std::fs;
use std::fs::File;
use std::collections::HashMap;
use crate::models::settings::load_settings;
use crate::models::game_state::game_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub name: String,
    pub description: String,
    pub reputation: f32,
    pub credits: f32,
    pub fleets: Vec<String>,
    pub relations: HashMap<String, f32>,
}

impl fmt::Display for Faction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Faction {
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            reputation: 0.0,
            credits: 1000.0,
            fleets: Vec::new(),
            relations: HashMap::new(),
        }
    }

    pub fn add_relation(&mut self, faction_name: String, relation: f32) {
        self.relations.insert(faction_name, relation);
    }

    pub fn get_relation(&self, faction_name: &str) -> Option<f32> {
        self.relations.get(faction_name).copied()
    }
}

pub fn save_faction(faction: &Faction) -> std::io::Result<()> {
    let faction_path = game_path(&["factions", &format!("{}.json", faction.name)]);
    let file = File::create(faction_path)?;
    serde_json::to_writer(file, faction)?;
    Ok(())
}

pub fn load_faction(faction_name: &str) -> std::io::Result<Option<Faction>> {
    let settings = load_settings().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let faction_path = game_path(&["factions", &format!("{}.json", faction_name)]);

    if !faction_path.exists() {
        return Ok(None);
    }

    let file = File::open(faction_path)?;
    let faction: Faction = serde_json::from_reader(file)?;
    Ok(Some(faction))
}

pub fn update_relations(faction1: &mut Faction, faction2: &mut Faction, change: f32) {
    faction1.reputation += change;
    faction2.reputation += change;
} 