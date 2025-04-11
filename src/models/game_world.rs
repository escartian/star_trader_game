use crate::models::planet::Planet;
use crate::models::ship::ship::Ship;
use crate::models::resource::Resource;
use crate::models::galaxy::generate_galaxy;
use crate::models::settings::{GameSettings, load_settings};
use std::fs;
use std::fs::File;
use serde_json;
use std::io::Read;
use serde_json::to_writer;
use crate::models::star_system::StarSystem;
use crate::models::game_state::game_path;
use std::path::Path;
use rand::thread_rng;
use rand::seq::SliceRandom;
use rand::Rng;
use crate::models::planet::PlanetSpecialization;
use strum_macros::EnumIter;
use strum::IntoEnumIterator;
use crate::models::economy::Economy;
use crate::models::position::random_position;
use crate::models::star_system::generate_star_system;
use serde::ser::{Serialize, Serializer, SerializeSeq};
use serde::{Deserialize};
use std::sync::Mutex;
use lazy_static::lazy_static;
use std::error::Error;

pub struct GameWorld {
    planets: Vec<Planet>,
    ships: Vec<Ship>,
}

pub struct PlayerState {
    player_name: String,
    ship: Vec<Ship>,
    inventory: Vec<Resource>,
    credits: u32,
}

impl GameWorld {
    pub fn new() -> GameWorld {
        GameWorld {
            planets: Vec::new(),
            ships: Vec::new(),
        }
    }

    pub fn update(&mut self, _player_state: &PlayerState) {
        /* ... */
    }
}

/// Creates a new game world file in the specified game directory and writes the generated
/// galaxy map to it. The file is stored in the "data/game/<game_id>/GameWorld.json" directory.
/// If the necessary directories do not exist, this function will create them.
///
/// # Arguments
/// - `settings` - Game settings containing map size, star count, etc.
///
/// # Returns
/// A Result containing either the loaded star systems or an error.
pub fn create_game_world_file(settings: &GameSettings, force_regenerate: bool) -> Result<Vec<StarSystem>, String> {
    println!("Starting game world creation");
    let game_path = Path::new("data").join("game").join(&settings.game_id);
    let game_world_path = game_path.join("GameWorld.json");
    
    // Check if we need to regenerate
    if !force_regenerate && game_world_path.exists() {
        println!("Game world file exists, loading from disk");
        let file = File::open(&game_world_path)
            .map_err(|e| format!("Failed to open game world file: {}", e))?;
        let world: Vec<StarSystem> = serde_json::from_reader(file)
            .map_err(|e| format!("Failed to deserialize game world: {}", e))?;
        println!("Successfully loaded game world with {} systems", world.len());
        return Ok(world);
    }
    
    println!("Generating new game world");
    let mut rng = thread_rng();
    
    // Create the game directory if it doesn't exist
    if let Some(parent) = game_world_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create game directory: {}", e))?;
    }
    
    // Create a temporary file for writing
    let temp_path = game_world_path.with_extension("json.tmp");
    let file = File::create(&temp_path)
        .map_err(|e| format!("Failed to create temporary game world file: {}", e))?;
    
    // Start writing the array
    let mut writer = serde_json::Serializer::new(file);
    let mut ser = writer.serialize_seq(Some(settings.star_count as usize))
        .map_err(|e| format!("Failed to start serialization: {}", e))?;
    
    // Generate and save star systems one at a time
    let mut existing_names = std::collections::HashSet::new();
    for i in 0..settings.star_count {
        println!("Generating star system {}/{}", i + 1, settings.star_count);
        let position = random_position(
            settings.map_width as i32,
            settings.map_height as i32,
            settings.map_length as i32
        );
        
        let system = generate_star_system(
            settings.map_width as i32,
            settings.map_height as i32,
            settings.map_length as i32,
            &mut existing_names
        );
        
        // Add the star name to our tracking set
        existing_names.insert(system.star.name.clone());
        
        // Serialize this system directly to the file
        ser.serialize_element(&system)
            .map_err(|e| format!("Failed to serialize system {}: {}", i, e))?;
        
        println!("Successfully generated star system at position {:?}", position);
    }
    
    // End the sequence
    ser.end()
        .map_err(|e| format!("Failed to end serialization: {}", e))?;
    
    // Rename the temporary file to the final file
    fs::rename(&temp_path, &game_world_path)
        .map_err(|e| format!("Failed to rename game world file: {}", e))?;
    
    // Now load the file back to return the systems
    let file = File::open(&game_world_path)
        .map_err(|e| format!("Failed to open game world file for reading: {}", e))?;
    let systems: Vec<StarSystem> = serde_json::from_reader(file)
        .map_err(|e| format!("Failed to deserialize game world: {}", e))?;
    
    println!("Successfully generated galaxy with {} star systems", systems.len());
    Ok(systems)
}

/// Loads a game world from the specified game directory.
///
/// # Arguments
/// - `game_id` - A string slice that holds the identifier for the game instance.
///
/// # Returns
/// A Result containing either the loaded star systems or an error.
pub fn load_game_world(game_id: &str) -> std::io::Result<Vec<StarSystem>> {
    let world_file = Path::new("data")
        .join("game")
        .join(game_id)
        .join("GameWorld.json");
        
    if !world_file.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Game world file not found",
        ));
    }

    // Open the file for reading
    let file = File::open(&world_file)?;
    
    let mut systems = Vec::new();
    systems = serde_json::from_reader(file)?;
    Ok(systems)
}

impl PlayerState {
    pub fn new() -> PlayerState {
        PlayerState {
            player_name: String::new(),
            ship: Vec::new(),
            inventory: Vec::new(),
            credits: 0,
        }
    }
}

/// Saves a single star system to its individual file
pub fn save_star_system(_game_id: &str, system_id: usize, system: &StarSystem) -> std::io::Result<()> {
    let system_path = game_path(&["star_systems", &format!("system_{}.json", system_id)]);

    let file = File::create(system_path)?;
    to_writer(file, system)?;
    Ok(())
}

/// Loads a single star system from its individual file
pub fn load_star_system(_game_id: &str, system_id: usize) -> std::io::Result<Option<StarSystem>> {
    let system_path = game_path(&["star_systems", &format!("system_{}.json", system_id)]);

    if !system_path.exists() {
        return Ok(None);
    }

    let file = File::open(system_path)?;
    let system: StarSystem = serde_json::from_reader(file)?;
    Ok(Some(system))
}

/// Saves a game world to the specified game directory.
///
/// # Arguments
/// - `_game_id` - A string slice that holds the identifier for the game instance.
/// - `star_systems` - A vector of star systems to save.
///
/// # Returns
/// A Result indicating success or failure.
pub fn save_game_world(_game_id: &str, star_systems: &[StarSystem]) -> std::io::Result<()> {
    let world_file = game_path(&["GameWorld.json"]);
    
    // Create the game directory if it doesn't exist
    if let Some(parent) = world_file.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = File::create(&world_file)?;
    to_writer(file, star_systems)?;
    Ok(())
}

lazy_static! {
    pub static ref GLOBAL_GAME_WORLD: Mutex<Vec<StarSystem>> = {
        println!("Initializing empty game world");
        Mutex::new(Vec::new())
    };
}

pub fn get_global_game_world() -> Vec<StarSystem> {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
}

pub fn split_game_world_into_systems() -> Result<(), Box<dyn Error>> {
    let settings = load_settings()?;
    let game_path = Path::new("data").join("game").join(&settings.game_id);
    let star_systems_path = game_path.join("star_systems");

    // Create star_systems directory if it doesn't exist
    if !star_systems_path.exists() {
        fs::create_dir_all(&star_systems_path)?;
    }

    // Load the game world
    let game_world = get_global_game_world();

    // Save each system to its own file
    for (system_id, system) in game_world.iter().enumerate() {
        let system_path = star_systems_path.join(format!("Star_System_{}.json", system_id));
        let file = File::create(system_path)?;
        serde_json::to_writer(file, system)?;
    }

    Ok(())
}

