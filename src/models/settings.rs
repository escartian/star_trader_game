use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use chrono::Utc;
use rocket::form::FromForm;
use crate::models::game_state::game_path;
use std::path::Path;
use std::fs::File;
use std::io::Read;
use uuid;

#[derive(Debug, Serialize, Deserialize, Clone, FromForm)]
pub struct GameSettings {
    pub game_id: String,
    #[serde(default = "default_display_name")]
    pub display_name: String,
    pub player_name: String,
    pub map_width: u32,
    pub map_height: u32,
    pub map_length: u32,
    pub star_count: u32,
    pub starting_credits: f32,
    pub created_at: String,
    pub last_played: String,
    pub factions: Vec<FactionSettings>,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromForm)]
pub struct FactionSettings {
    pub name: String,
    pub influence: u32,
}

fn default_display_name() -> String {
    "New Game".to_string()
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            game_id: "default_game".to_string(),
            display_name: default_display_name(),
            player_name: "Player".to_string(),
            map_width: 1000,
            map_height: 1000,
            map_length: 1000,
            star_count: 10,
            starting_credits: 1000.0,
            created_at: Utc::now().to_rfc3339(),
            last_played: Utc::now().to_rfc3339(),
            factions: vec![
                FactionSettings { name: "Federation".to_string(), influence: 50 },
                FactionSettings { name: "Empire".to_string(), influence: 50 },
                FactionSettings { name: "Republic".to_string(), influence: 50 },
                FactionSettings { name: "Alliance".to_string(), influence: 50 },
            ],
        }
    }
}

impl GameSettings {
    pub fn new(display_name: String, player_name: String, map_width: u32, map_height: u32, map_length: u32, star_count: u32, starting_credits: f32) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            game_id: uuid::Uuid::new_v4().to_string(),
            display_name,
            player_name,
            map_width,
            map_height,
            map_length,
            star_count,
            starting_credits,
            created_at: now.clone(),
            last_played: now,
            factions: vec![
                FactionSettings { name: "Federation".to_string(), influence: 50 },
                FactionSettings { name: "Empire".to_string(), influence: 50 },
                FactionSettings { name: "Republic".to_string(), influence: 50 },
                FactionSettings { name: "Alliance".to_string(), influence: 50 },
            ],
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let settings_path = game_path(&["settings.json"]);
        let file = fs::File::create(settings_path)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    pub fn load() -> io::Result<Self> {
        let settings_path = game_path(&["settings.json"]);
        if !settings_path.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "No settings file found"));
        }

        let file = fs::File::open(settings_path)?;
        let settings: GameSettings = serde_json::from_reader(file)?;
        Ok(settings)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavedGame {
    pub game_id: String,
    pub display_name: String,
    pub created_at: String,
    pub last_played: String,
    pub settings: GameSettings,
}

impl SavedGame {
    pub fn save_game(&self) -> std::io::Result<()> {
        let saves_dir = Path::new("data").join("saves");
        fs::create_dir_all(&saves_dir)?;
        
        let save_file = saves_dir.join(format!("{}.json", self.game_id));
        let file = fs::File::create(save_file)?;
        serde_json::to_writer(file, self)?;
        Ok(())
    }

    pub fn load_game(game_id: &str) -> std::io::Result<Option<Self>> {
        let save_file = Path::new("data").join("saves").join(format!("{}.json", game_id));
        if !save_file.exists() {
            return Ok(None);
        }

        let file = fs::File::open(save_file)?;
        let game: SavedGame = serde_json::from_reader(file)?;
        Ok(Some(game))
    }

    pub fn list_saved_games() -> std::io::Result<Vec<SavedGame>> {
        let saves_dir = Path::new("data").join("saves");
        if !saves_dir.exists() {
            return Ok(Vec::new());
        }

        let mut games = Vec::new();
        for entry in fs::read_dir(saves_dir)? {
            if let Ok(entry) = entry {
                if let Ok(file) = fs::File::open(entry.path()) {
                    if let Ok(game) = serde_json::from_reader(file) {
                        games.push(game);
                    }
                }
            }
        }
        Ok(games)
    }

    pub fn load_current_game() -> std::io::Result<Option<Self>> {
        let saves_dir = Path::new("data").join("saves");
        if !saves_dir.exists() {
            return Ok(None);
        }

        // Get the most recent save file
        let mut entries: Vec<_> = fs::read_dir(saves_dir)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by(|a, b| {
            b.metadata().unwrap().modified().unwrap()
                .cmp(&a.metadata().unwrap().modified().unwrap())
        });

        if let Some(entry) = entries.first() {
            let file = fs::File::open(entry.path())?;
            let game: SavedGame = serde_json::from_reader(file)?;
            Ok(Some(game))
        } else {
            Ok(None)
        }
    }
}

/// Loads game settings from the settings.json file.
/// 
/// # Returns
/// A Result containing either the loaded settings or an error
pub fn load_settings() -> Result<GameSettings, std::io::Error> {
    let state = crate::models::game_state::get_game_state()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    
    let game_id = state.current_game_id.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "No active game. Please create or load a game first."
        )
    })?;
    
    let settings_path = Path::new("data").join("game").join(&game_id).join("settings.json");
    
    if !settings_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No settings found for game {}", game_id)
        ));
    }

    let mut file = File::open(settings_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let settings: GameSettings = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    println!("Successfully loaded settings for game {}", game_id);
    Ok(settings)
}

/// Loads game settings for a specific game ID.
/// 
/// # Arguments
/// * `game_id` - The ID of the game to load settings for
/// 
/// # Returns
/// A Result containing either the loaded settings or an error
pub fn load_settings_for_game(game_id: &str) -> Result<GameSettings, std::io::Error> {
    let settings_path = Path::new("data").join("game").join(game_id).join("settings.json");
    
    if !settings_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No settings found for game {}", game_id)
        ));
    }

    let mut file = File::open(settings_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    
    let settings: GameSettings = serde_json::from_str(&contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    println!("Successfully loaded settings for game {}", game_id);
    Ok(settings)
}

/// Saves the current game settings to the settings.json file.
/// 
/// # Arguments
/// * `settings` - The settings to save
/// 
/// # Returns
/// A Result indicating success or failure
pub fn save_settings(settings: &GameSettings) -> io::Result<()> {
    settings.save()
} 