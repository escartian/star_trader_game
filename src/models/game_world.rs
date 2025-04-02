use crate::models::planet::Planet;
use crate::models::ship::ship::Ship;
use crate::models::resource::Resource;
use crate::models::galaxy::generate_galaxy;
use crate::constants::STAR_COUNT;
use std::fs;
use std::path::Path;
use std::fs::File;
use serde_json;
use serde::{Deserialize, Serialize};
use std::io::Read;
use serde_json::to_writer;
use crate::models::star_system::StarSystem;
use crate::models::market::Market;

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

    pub fn update(&mut self, player_state: &PlayerState) {
        /* ... */
    }
}

/// Creates a new game world file in the specified game directory and writes the generated
/// galaxy map to it. The file is stored in the "data/game/<game_id>/GameWorld.json" directory.
/// If the necessary directories do not exist, this function will create them.
///
/// # Arguments
/// - `game_id` - A string slice that holds the identifier for the game instance.
///
/// # Errors
/// This function will panic if it is unable to create the file or write the galaxy map to it.
pub fn create_game_world_file(game_id: &str, empty_world: bool) -> Vec<StarSystem> {
    // WORLD GENERATION
    // Generate the galaxy map
    println!("Creating Game World File");
    let galactic_map;
    if empty_world {   
        galactic_map = generate_galaxy(STAR_COUNT);
        
        // Create the star_systems directory
        let systems_dir = Path::new("data")
            .join("game")
            .join(game_id)
            .join("star_systems");
        
        if let Some(parent) = systems_dir.parent() {
            println!("{}", parent.display());
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
        fs::create_dir_all(&systems_dir).expect("Failed to create star_systems directory");

        // Save each star system individually and initialize markets
        for (index, system) in galactic_map.iter().enumerate() {
            let system_path = systems_dir.join(format!("system_{}.json", index));
            let file = File::create(system_path).expect("Failed to create system file");
            to_writer(file, system).expect("Failed to write system data");

            // Initialize markets for each planet in the system
            for (planet_id, planet) in system.planets.iter().enumerate() {
                // Create resource market
                let market = Market::new(
                    planet.name.clone(),
                    index,
                    planet_id,
                    planet.specialization.clone(),
                    planet.economy.clone()
                );
                market.save().expect("Failed to save planet market");

                // Generate and save ship market
                let ships = planet.generate_ship_market();
                let market_path = Path::new("data")
                    .join("game")
                    .join(game_id)
                    .join("markets")
                    .join(format!("Star_System_{}_Planet_{}_ships.json", index, planet_id));

                if let Some(parent) = market_path.parent() {
                    fs::create_dir_all(parent).expect("Failed to create markets directory");
                }

                let market_json = serde_json::to_string(&ships).expect("Failed to serialize ship market");
                fs::write(&market_path, market_json).expect("Failed to save ship market");
            }
        }

        // Save the full game world for save state
        let world_path = Path::new("data")
            .join("game")
            .join(game_id)
            .join("GameWorld.json");
        
        let file = File::create(world_path).expect("Failed to create game world file");
        to_writer(file, &galactic_map).expect("Failed to write game world data");
        
        println!("Game world files created successfully");
        return galactic_map;
    } else {
        println!("Loading existing game world");
        let systems_dir = Path::new("data")
            .join("game")
            .join(game_id)
            .join("star_systems");

        if systems_dir.exists() {
            // Load from individual system files
            let mut systems = Vec::new();
            for i in 0..STAR_COUNT {
                let system_path = systems_dir.join(format!("system_{}.json", i));
                if system_path.exists() {
                    let mut contents = String::new();
                    File::open(system_path).expect("Failed to open system file").read_to_string(&mut contents).expect("Failed to read system file");
                    let mut system: StarSystem = serde_json::from_str(&contents).expect("Failed to parse system data");
                    
                    // Load markets for each planet
                    for (planet_id, planet) in system.planets.iter_mut().enumerate() {
                        let mut market = Market::load(i as usize, planet_id).expect("Failed to load planet market");
                        // Update market if needed based on planet's current state
                        if market.needs_update(&planet.specialization, &planet.economy) {
                            market.update(&planet.specialization, &planet.economy).expect("Failed to update market");
                        }

                        // Check if ship market exists, if not generate it
                        let ship_market_path = Path::new("data")
                            .join("game")
                            .join(game_id)
                            .join("markets")
                            .join(format!("Star_System_{}_Planet_{}_ships.json", i, planet_id));

                        if !ship_market_path.exists() {
                            let ships = planet.generate_ship_market();
                            if let Some(parent) = ship_market_path.parent() {
                                fs::create_dir_all(parent).expect("Failed to create markets directory");
                            }
                            let market_json = serde_json::to_string(&ships).expect("Failed to serialize ship market");
                            fs::write(&ship_market_path, market_json).expect("Failed to save ship market");
                        }
                    }
                    
                    systems.push(system);
                }
            }
            return systems;
        } else {
            // Fallback to loading from the full game world file
            let world_path = Path::new("data")
                .join("game")
                .join(game_id)
                .join("GameWorld.json");
            let mut contents = String::new();
            File::open(world_path).expect("Failed to open game world file").read_to_string(&mut contents).expect("Failed to read game world file");
            let mut systems: Vec<StarSystem> = serde_json::from_str(&contents).expect("Failed to parse game world data");
            
            // Load markets for each planet in each system
            for (system_id, system) in systems.iter_mut().enumerate() {
                for (planet_id, planet) in system.planets.iter_mut().enumerate() {
                    let mut market = Market::load(system_id, planet_id).expect("Failed to load planet market");
                    // Update market if needed based on planet's current state
                    if market.needs_update(&planet.specialization, &planet.economy) {
                        market.update(&planet.specialization, &planet.economy).expect("Failed to update market");
                    }

                    // Check if ship market exists, if not generate it
                    let ship_market_path = Path::new("data")
                        .join("game")
                        .join(game_id)
                        .join("markets")
                        .join(format!("Star_System_{}_Planet_{}_ships.json", system_id, planet_id));

                    if !ship_market_path.exists() {
                        let ships = planet.generate_ship_market();
                        if let Some(parent) = ship_market_path.parent() {
                            fs::create_dir_all(parent).expect("Failed to create markets directory");
                        }
                        let market_json = serde_json::to_string(&ships).expect("Failed to serialize ship market");
                        fs::write(&ship_market_path, market_json).expect("Failed to save ship market");
                    }
                }
            }
            
            return systems;
        }
    }
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
pub fn save_star_system(game_id: &str, system_id: usize, system: &StarSystem) -> std::io::Result<()> {
    let system_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("star_systems")
        .join(format!("system_{}.json", system_id));

    let file = File::create(system_path)?;
    to_writer(file, system)?;
    Ok(())
}

/// Loads a single star system from its individual file
pub fn load_star_system(game_id: &str, system_id: usize) -> std::io::Result<Option<StarSystem>> {
    let system_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("star_systems")
        .join(format!("system_{}.json", system_id));

    if !system_path.exists() {
        return Ok(None);
    }

    let mut contents = String::new();
    File::open(system_path)?.read_to_string(&mut contents)?;
    let system: StarSystem = serde_json::from_str(&contents)?;
    Ok(Some(system))
}

