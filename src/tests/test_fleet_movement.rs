use crate::models::fleet::{Fleet, MoveFleetData, MoveFleetResponse, save_fleet};
use crate::models::position::Position;
use crate::models::settings::GameSettings;
use crate::models::star_system::{StarSystem, generate_star_system_default, generate_star_system};
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine, ShipStatus, CombatState};
use crate::models::ship::weapon::Weapon;
use crate::models::ship::shield::Shield;
use crate::models::ship::armor::Armor;
use crate::models::resource::{ResourceType, Resource};
use crate::routes::move_fleet;
use rocket::serde::json::Json;
use std::sync::Once;
use std::fs;
use crate::models::game_state::{get_game_state, save_game_state, game_data_path, clear_caches};
use crate::GLOBAL_GAME_WORLD;
use serde_json;
use std::sync::atomic::{AtomicUsize, Ordering};

static INIT: Once = Once::new();

// Counter for generating unique fleet IDs in tests
static TEST_FLEET_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn verify_game_world() {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        assert!(!guard.is_empty(), "Game world should not be empty");
        println!("Game world contains {} systems", guard.len());
        for (i, system) in guard.iter().enumerate() {
            println!("System {} at position ({}, {}, {})", i, system.position.x, system.position.y, system.position.z);
        }
    } else {
        panic!("Failed to access game world");
    }
}

// Setup runs once, initializes shared environment *without* fleets
fn setup() {
    INIT.call_once(|| {
        println!("Initializing shared test environment (once)...");
        
        // Clear all caches first
        clear_caches();
        
        // Clean up any existing test data
        let test_dir = game_data_path("test_game", &[]);
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).expect("Failed to clean up test directory");
        }

        // Initialize test environment settings
        let settings = GameSettings {
            game_id: "test_game".to_string(),
            player_name: "test_player".to_string(),
            map_width: 100,
            map_height: 100,
            map_length: 100,
            ..Default::default()
        };

        println!("Creating test directories...");
        fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        fs::create_dir_all(test_dir.join("fleets")).expect("Failed to create fleets directory");
        fs::create_dir_all(test_dir.join("star_systems")).expect("Failed to create star_systems directory");

        let settings_path = test_dir.join("settings.json");
        fs::write(settings_path, serde_json::to_string(&settings).unwrap()).expect("Failed to save test settings");

        println!("Creating test star system...");
        let mut existing_names = std::collections::HashSet::new();
        let system_data = generate_star_system(100, 100, 100, &mut existing_names);
        let system = StarSystem {
            star: system_data.star,
            position: Position { x: 0, y: 0, z: 0 },
            planets: system_data.planets,
        };

        let system_path = test_dir.join("star_systems").join("Star_System_0.json");
        fs::write(system_path, serde_json::to_string(&system).unwrap()).expect("Failed to save test system");

        println!("Initializing game state...");
        let mut state = get_game_state().expect("Failed to get game state");
        state.current_game_id = Some("test_game".to_string());
        state.credits = 1000.0;
        save_game_state(state).expect("Failed to save game state");

        println!("Initializing global game world...");
        if let Ok(mut guard) = GLOBAL_GAME_WORLD.lock() {
            *guard = vec![system.clone()];
            println!("Game world initialized with {} systems", guard.len());
        } else {
            panic!("Failed to initialize game world");
        }
        
        // Removed fleet creation from the shared setup
        println!("Shared test environment base setup complete.");
    });
    // Verify world after setup (can stay outside call_once)
    verify_game_world();
}

// This function creates the fleet structure but doesn't save it.
// Name/position/system are set by the caller.
fn create_test_fleet_structure() -> Fleet {
    let mut fleet = Fleet::new("temp_owner".to_string(), Position { x: -999, y: -999, z: -999 }, 0); // Temporary details
    let mut capital_ship = Ship::new(ShipType::Capital, ShipSize::Tiny, ShipEngine::Experimental);
    capital_ship.name = "Gamma 285-type".to_string();
    capital_ship.owner = "test_player".to_string();
    capital_ship.position = Position { x: 0, y: 0, z: 0 };
    capital_ship.status = ShipStatus::Stationary;
    capital_ship.hp = 150;
    capital_ship.combat_state = CombatState::NotInCombat;
    
    // Add weapons
    capital_ship.weapons = vec![
        Weapon::PhotonSingularityBeam { damage: 10 },
        Weapon::QuantumEntanglementTorpedo { damage: 20 },
        Weapon::MagneticResonanceDisruptor { damage: 50 },
        Weapon::GravitonPulse { damage: 40 },
        Weapon::NeutronBeam { damage: 30 }
    ];
    
    // Add cargo
    capital_ship.cargo = vec![
        Resource::new(ResourceType::Fuel, 40),
        Resource::new(ResourceType::Metals, 30),
        Resource::new(ResourceType::Electronics, 17)
    ];
    
    // Add shields and armor
    capital_ship.shields = Shield::new(225);
    capital_ship.armor = Armor::new(300);
    
    fleet.add_ship(capital_ship);

    // Add a freighter
    let mut freighter = Ship::new(ShipType::Freighter, ShipSize::Small, ShipEngine::Basic);
    freighter.name = "Xi 451-mark".to_string();
    freighter.owner = "test_player".to_string();
    freighter.position = Position { x: 0, y: 0, z: 0 };
    freighter.status = ShipStatus::Stationary;
    freighter.hp = 75;
    freighter.combat_state = CombatState::NotInCombat;
    
    // Add weapons
    freighter.weapons = vec![
        Weapon::PhotonSingularityBeam { damage: 10 },
        Weapon::GravitonPulse { damage: 40 }
    ];
    
    // Add cargo
    freighter.cargo = vec![
        Resource::new(ResourceType::Fuel, 43),
        Resource::new(ResourceType::Minerals, 26),
        Resource::new(ResourceType::Food, 19),
        Resource::new(ResourceType::Electronics, 15),
        Resource::new(ResourceType::LuxuryGoods, 26)
    ];
    
    // Add shields and armor
    freighter.shields = Shield::new(112);
    freighter.armor = Armor::new(150);
    
    fleet.add_ship(freighter);

    // Add a shuttle
    let mut shuttle = Ship::new(ShipType::Shuttle, ShipSize::Medium, ShipEngine::Basic);
    shuttle.name = "Psi 224-type".to_string();
    shuttle.owner = "test_player".to_string();
    shuttle.position = Position { x: 0, y: 0, z: 0 };
    shuttle.status = ShipStatus::Stationary;
    shuttle.hp = 30;
    shuttle.combat_state = CombatState::NotInCombat;
    
    // Add weapons
    shuttle.weapons = vec![
        Weapon::PhotonSingularityBeam { damage: 10 },
        Weapon::QuantumEntanglementTorpedo { damage: 20 }
    ];
    
    // Add cargo
    shuttle.cargo = vec![
        Resource::new(ResourceType::Fuel, 3),
        Resource::new(ResourceType::Food, 14),
        Resource::new(ResourceType::Water, 5)
    ];
    
    // Add shields and armor
    shuttle.shields = Shield::new(45);
    shuttle.armor = Armor::new(60);
    
    fleet.add_ship(shuttle);

    fleet
}

// Helper function to create a unique fleet for a test run
fn create_and_save_unique_fleet() -> Fleet {
    let fleet_number = TEST_FLEET_COUNTER.fetch_add(1, Ordering::SeqCst);
    let owner_id = "test_player".to_string();
    let unique_fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    println!("Creating unique fleet: {}", unique_fleet_name);

    let mut fleet = create_test_fleet_structure();
    // Assign the unique name and initial state
    fleet.name = unique_fleet_name.clone();
    fleet.owner_id = owner_id.clone();
    fleet.current_system_id = Some(0); // Start in system 0
    fleet.position = Position { x: 0, y: 0, z: 0 }; // Start at center

    // Save this specific fleet
    save_fleet(&fleet).expect(&format!("Failed to save unique test fleet {}", fleet.name));
    println!("Saved unique fleet {}", fleet.name);

    // Quick verification it saved correctly
    let loaded = Fleet::load(&fleet.name).expect("Failed load verification after save");
    assert_eq!(loaded.name, fleet.name);
    assert_eq!(loaded.position.x, 0);
    assert_eq!(loaded.current_system_id, Some(0));

    fleet // Return the created and saved fleet
}

#[test]
fn test_valid_move_within_system() {
    setup(); // Ensure shared env is ready
    let fleet = create_and_save_unique_fleet(); // Create fleet for this test
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    let move_data = MoveFleetData { x: 50, y: 50, z: 50 };
    let response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(move_data));
    
    println!("Response: {:?}", response);
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "success");
    assert_eq!(data.current_position.x, 50);
    assert_eq!(data.current_position.y, 50);
    assert_eq!(data.current_position.z, 50);
    assert_eq!(data.current_system_id, Some(0));

    let updated_fleet = Fleet::load(&fleet.name).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 50);
        assert_eq!(ship.position.y, 50);
        assert_eq!(ship.position.z, 50);
        assert_eq!(ship.status, ShipStatus::Stationary);
        assert_eq!(ship.combat_state, CombatState::NotInCombat);
        assert!(ship.shields.current > 0);
        assert!(ship.armor.current > 0);
    }
}

#[test]
fn test_system_exit() {
    setup();
    let fleet = create_and_save_unique_fleet();
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    let move_data = MoveFleetData { x: 101, y: 0, z: 0 };
    let response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(move_data));
    
    println!("Response: {:?}", response);
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "transition_exit");
    assert_eq!(data.current_system_id, None);
    assert_eq!(data.current_position.x, 101);
    assert_eq!(data.current_position.y, 0);
    assert_eq!(data.current_position.z, 0);

    let updated_fleet = Fleet::load(&fleet.name).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 101);
        assert_eq!(ship.position.y, 0);
        assert_eq!(ship.position.z, 0);
    }
}

#[test]
fn test_system_entry() {
    setup();
    let fleet = create_and_save_unique_fleet();
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    // First move fleet to deep space
    let exit_data = MoveFleetData { x: 100, y: 0, z: 0 };
    let exit_response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(exit_data));
    println!("Exit response: {:?}", exit_response);
    assert!(exit_response.success);
    
    // Now try to enter the system by moving to its coordinates
    let entry_data = MoveFleetData { x: 0, y: 0, z: 0 };
    let entry_response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(entry_data));
    
    println!("Entry response: {:?}", entry_response);
    assert!(entry_response.success);
    let data = entry_response.into_inner().data.unwrap();
    assert_eq!(data.status, "transition_entry");
    assert_eq!(data.current_system_id, Some(0));
    
    // Verify entry point based on the approach from (100, 0, 0)
    assert_eq!(data.current_position.x, -100); 
    assert_eq!(data.current_position.y, 0);
    assert_eq!(data.current_position.z, 0);

    let updated_fleet = Fleet::load(&fleet.name).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, data.current_position.x);
        assert_eq!(ship.position.y, data.current_position.y);
        assert_eq!(ship.position.z, data.current_position.z);
    }
}

#[test]
fn test_deep_space_movement() {
    setup();
    let fleet = create_and_save_unique_fleet();
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    // First move fleet to deep space 
    let exit_data = MoveFleetData { x: 100, y: 0, z: 0 };
    let exit_response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(exit_data));
    println!("Exit response: {:?}", exit_response);
    assert!(exit_response.success);
    
    // Now move in deep space
    let move_data = MoveFleetData { x: 50, y: 50, z: 50 };
    let response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(move_data));
    
    println!("Response: {:?}", response);
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "success");
    assert_eq!(data.current_system_id, None);
    assert_eq!(data.current_position.x, 50);
    assert_eq!(data.current_position.y, 50);
    assert_eq!(data.current_position.z, 50);

    let updated_fleet = Fleet::load(&fleet.name).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 50);
        assert_eq!(ship.position.y, 50);
        assert_eq!(ship.position.z, 50);
    }
}

#[test]
fn test_invalid_move_outside_bounds() {
    setup();
    let fleet = create_and_save_unique_fleet();
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    // Test moving outside galaxy bounds
    let move_data = MoveFleetData { x: 101, y: 101, z: 101 };
    let response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(move_data));
    
    assert!(!response.success);
    assert!(response.into_inner().message.contains("outside galaxy bounds"));
}

#[test]
fn test_move_nonexistent_fleet() {
    setup(); // Ensure shared env is ready
    // Don't create a fleet for this test

    // Test moving a fleet that doesn't exist 
    let move_data = MoveFleetData { x: 0, y: 0, z: 0 };
    let response = move_fleet("nonexistent_player".to_string(), 999, Json(move_data));
    
    assert!(!response.success);
    assert!(response.into_inner().message.contains("Fleet 'Fleet_nonexistent_player_999' not found"));
}

#[test]
fn test_move_with_cargo() {
    setup();
    let fleet = create_and_save_unique_fleet();
    let fleet_number = fleet.name.split('_').last().unwrap().parse::<usize>().unwrap();

    let move_data = MoveFleetData { x: 50, y: 50, z: 50 };
    let response = move_fleet(fleet.owner_id.clone(), fleet_number, Json(move_data));
    
    println!("Response: {:?}", response);
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "success");
    
    let updated_fleet = Fleet::load(&fleet.name).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 50);
        assert_eq!(ship.position.y, 50);
        assert_eq!(ship.position.z, 50);
        assert!(!ship.cargo.is_empty());
        for resource in ship.cargo {
            assert!(resource.quantity.unwrap_or(0) > 0);
        }
    }
} 