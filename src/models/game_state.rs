use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::{Duration, Instant};
#[macro_use]
use lazy_static::lazy_static;
use crate::GAME_ID;
use crate::models::player::Player;
use crate::models::star_system::StarSystem;
use crate::models::fleet::Fleet;
use serde::de::DeserializeOwned;
use serde::Serialize;

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
}

// Global caches
lazy_static! {
    pub static ref PLAYER_CACHE: Cache<Player> = Cache::new(30); // 30 seconds TTL
    pub static ref SYSTEM_CACHE: Cache<StarSystem> = Cache::new(60); // 60 seconds TTL
    pub static ref FLEET_CACHE: Cache<Fleet> = Cache::new(30); // 30 seconds TTL
}

pub fn game_path(components: &[&str]) -> PathBuf {
    let mut path = PathBuf::from("data").join("game").join(GAME_ID);
    for component in components {
        path = path.join(component);
    }
    path
}

pub fn ensure_parent_dirs(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn save_json<T: Serialize>(path: &Path, data: &T) -> Result<(), String> {
    ensure_parent_dirs(path)
        .map_err(|e| format!("Failed to create directories: {}", e))?;
    
    let file = File::create(path)
        .map_err(|e| format!("Failed to create file: {}", e))?;
    
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
    save_player(player)?;
    save_star_system(system_id, system)?;
    Ok(())
} 