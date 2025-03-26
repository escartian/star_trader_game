use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Read;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub name: String,
    pub description: String,
    pub influence: f32,  // 0.0 to 1.0
    pub relations: std::collections::HashMap<String, f32>,  // Relations with other factions
}

impl fmt::Display for Faction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Faction {
    pub fn new(name: String, description: String) -> Self {
        Faction {
            name,
            description,
            influence: 0.01,  // Starting influence
            relations: std::collections::HashMap::new(),
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
    let data_path = std::path::Path::new("data")
        .join("game")
        .join(crate::constants::GAME_ID)
        .join("factions")
        .join(format!("{}.json", faction.name));

    if let Some(parent) = data_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = std::fs::File::create(data_path)?;
    serde_json::to_writer(file, faction)?;
    Ok(())
}

pub fn load_faction(faction_name: &str) -> std::io::Result<Option<Faction>> {
    let data_path = std::path::Path::new("data")
        .join("game")
        .join(crate::constants::GAME_ID)
        .join("factions")
        .join(format!("{}.json", faction_name));

    if !data_path.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    std::fs::File::open(data_path)?.read_to_string(&mut contents)?;
    let faction: Faction = serde_json::from_str(&contents)?;
    Ok(Some(faction))
} 