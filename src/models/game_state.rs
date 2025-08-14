use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::sync::{Arc, RwLock, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::io::Write;
use lazy_static::lazy_static;
use crate::models::settings::load_settings;
use crate::models::player::Player;
use crate::models::star_system::StarSystem;
use crate::models::fleet::Fleet;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::models::market::Market;

#[derive(Clone)]
pub struct GameState {
    pub current_game_id: Option<String>,
    pub credits: f64,
}

lazy_static! {
    pub static ref GAME_STATE: Mutex<GameState> = {
        println!("Initializing game state");
        Mutex::new(GameState {
            current_game_id: None,
            credits: 0.0,
        })
    };
}

pub fn get_game_state() -> Result<GameState, String> {
    if let Ok(guard) = GAME_STATE.lock() {
        Ok((*guard).clone())
    } else {
        Err("Failed to lock game state".to_string())
    }
}

pub fn save_game_state(state: GameState) -> Result<(), String> {
    if let Ok(mut guard) = GAME_STATE.lock() {
        *guard = state;
        Ok(())
    } else {
        Err("Failed to lock game state".to_string())
    }
}

// Cache structure
#[derive(Clone)]
pub struct Cache<T: Clone> {
    data: Arc<RwLock<HashMap<String, (T, Instant)>>>,
    ttl: Duration,
}

impl<T: Clone> Cache<T> {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    pub fn get(&self, key: &str) -> Option<T> {
        if let Ok(data) = self.data.read() {
            if let Some((value, timestamp)) = data.get(key) {
                if timestamp.elapsed() < self.ttl {
                    return Some(value.clone());
                }
            }
        }
        None
    }

    pub fn set(&self, key: String, value: T) {
        if let Ok(mut data) = self.data.write() {
            data.insert(key, (value, Instant::now()));
        }
    }

    pub fn remove(&self, key: &str) {
        if let Ok(mut data) = self.data.write() {
            data.remove(key);
        }
    }

    pub fn remove_all(&self) {
        if let Ok(mut data) = self.data.write() {
            data.clear();
        }
    }
}

// Global caches
lazy_static! {
    pub static ref PLAYER_CACHE: Cache<Player> = Cache::new(30); // 30 seconds TTL
    pub static ref SYSTEM_CACHE: Cache<StarSystem> = Cache::new(60); // 60 seconds TTL
    pub static ref FLEET_CACHE: Cache<Fleet> = Cache::new(30); // 30 seconds TTL
    pub static ref MARKET_CACHE: Cache<Market> = Cache::new(30); // 30 seconds TTL
}

pub fn game_path(components: &[&str]) -> PathBuf {
    let mut path = PathBuf::from("data").join("game");
    
    // Try to get game_id from game state, but don't fail if we can't
    if let Ok(state) = get_game_state() {
        if let Some(game_id) = state.current_game_id {
            path = path.join(game_id);
        }
    }
    
    for component in components {
        path = path.join(component);
    }
    path
}

pub fn game_path_with_id(game_id: &str, components: &[&str]) -> PathBuf {
    let mut path = PathBuf::from("data").join("game").join(game_id);
    for component in components {
        path = path.join(component);
    }
    path
}

pub fn game_data_path(game_id: &str, path: &[&str]) -> std::path::PathBuf {
    let mut full_path = std::path::PathBuf::from("data").join("game").join(game_id);
    for component in path {
        full_path = full_path.join(component);
    }
    full_path
}

pub fn ensure_parent_dirs(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    println!("Starting to save JSON to {}", path.display());
    ensure_parent_dirs(path)
        .map_err(|e| format!("Failed to create directories: {}", e))?;
    
    println!("Creating file at {}", path.display());
    let file = File::create(path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    
    println!("Serializing data to JSON");
    serde_json::to_writer(file, data)
        .map_err(|e| format!("Failed to write JSON: {}", e))
}

pub fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let file = File::open(path)
        .map_err(|e| format!("Failed to open file: {}", e))?;
    
    serde_json::from_reader(file)
        .map_err(|e| format!("Failed to parse JSON: {}", e))
}

pub fn save_player(player: &Player) -> Result<(), String> {
    let path = game_path(&["players", &format!("{}.json", player.name)]);
    save_json(&path, player)?;
    PLAYER_CACHE.set(player.name.clone(), player.clone());
    Ok(())
}

pub fn load_player(name: &str) -> Result<Player, String> {
    if let Some(player) = PLAYER_CACHE.get(name) {
        return Ok(player);
    }
    
    let path = game_path(&["players", &format!("{}.json", name)]);
    if !path.exists() {
        return Err(format!("Player file not found: {}", path.display()));
    }
    
    let player: Player = load_json(&path)?;
    PLAYER_CACHE.set(name.to_string(), player.clone());
    Ok(player)
}

pub fn save_star_system(system_id: usize, system: &StarSystem) -> Result<(), String> {
    let path = game_path(&["star_systems", &format!("system_{}.json", system_id)]);
    save_json(&path, system)?;
    SYSTEM_CACHE.set(system_id.to_string(), system.clone());
    Ok(())
}

pub fn load_star_system(system_id: usize) -> Result<StarSystem, String> {
    if let Some(system) = SYSTEM_CACHE.get(&system_id.to_string()) {
        return Ok(system);
    }
    
    let path = game_path(&["star_systems", &format!("system_{}.json", system_id)]);
    let system: StarSystem = load_json(&path)?;
    SYSTEM_CACHE.set(system_id.to_string(), system.clone());
    Ok(system)
}

pub fn save_fleet(fleet: &Fleet) -> Result<(), String> {
    let path = game_path(&["fleets", &format!("{}.json", fleet.name)]);
    save_json(&path, fleet)?;
    FLEET_CACHE.set(fleet.name.clone(), fleet.clone());
    Ok(())
}

pub fn load_fleet(name: &str) -> Result<Fleet, String> {
    if let Some(fleet) = FLEET_CACHE.get(name) {
        return Ok(fleet);
    }
    
    let path = game_path(&["fleets", &format!("{}.json", name)]);
    let fleet: Fleet = load_json(&path)?;
    FLEET_CACHE.set(name.to_string(), fleet.clone());
    Ok(fleet)
}

// Helper for trade operations
pub fn save_trade_state(player: &Player, system: &StarSystem, system_id: usize) -> Result<(), String> {
    // Save player state first
    let player_path = game_path(&["players", &format!("{}.json", player.name)]);
    save_json(&player_path, player).map_err(|e| format!("Failed to save player state: {}", e))?;
    
    // Save star system state
    let system_path = game_path(&["star_systems", &format!("system_{}.json", system_id)]);
    save_json(&system_path, system).map_err(|e| format!("Failed to save system state: {}", e))?;
    
    Ok(())
}

// Helper to save all star systems to individual files
pub fn save_star_systems(systems: &[StarSystem]) -> Result<(), String> {
    println!("Starting to save {} star systems", systems.len());
    let settings = load_settings().map_err(|e| format!("Failed to load settings: {}", e))?;
    println!("Loaded settings");
    let systems_dir = Path::new("data")
        .join("game")
        .join(&settings.game_id)
        .join("star_systems");
    println!("Creating systems directory at {}", systems_dir.display());
    if let Err(e) = fs::create_dir_all(&systems_dir) {
        return Err(format!("Failed to create systems directory: {}", e));
    }
    
    // Process systems in smaller batches
    const BATCH_SIZE: usize = 2; // Process 2 systems at a time
    for chunk in systems.chunks(BATCH_SIZE) {
        for (i, system) in chunk.iter().enumerate() {
            let system_index = i + (chunk.as_ptr() as usize - systems.as_ptr() as usize) / std::mem::size_of::<StarSystem>();
            println!("Processing system {}/{} with {} planets", 
                system_index + 1, systems.len(), system.planets.len());
            
            let system_path = systems_dir.join(format!("system_{}.json", system_index));
            
            // Use a buffered writer for better performance
            let file = File::create(&system_path)
                .map_err(|e| format!("Failed to create file for system {}: {}", system_index, e))?;
            let mut writer = std::io::BufWriter::new(file);
            
            serde_json::to_writer(&mut writer, system)
                .map_err(|e| format!("Failed to serialize system {}: {}", system_index, e))?;
            
            // Ensure the buffer is flushed
            writer.flush()
                .map_err(|e| format!("Failed to flush system {}: {}", system_index, e))?;
            
            println!("Successfully saved system {}", system_index);
            
            // Clear the cache after each system to free memory
            SYSTEM_CACHE.remove(&system_index.to_string());
        }
    }
    
    println!("Completed saving all star systems");
    Ok(())
}

pub fn clear_caches() {
    PLAYER_CACHE.remove_all();
    SYSTEM_CACHE.remove_all();
    FLEET_CACHE.remove_all();
    MARKET_CACHE.remove_all();
} 