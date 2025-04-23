use crate::models::fleet::{Fleet, MoveFleetData, MoveFleetResponse, save_fleet};
use crate::models::position::Position;
use crate::models::settings::GameSettings;
use crate::models::star_system::{StarSystem, generate_star_system_default};
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine, ShipStatus, CombatState};
use crate::models::ship::weapon::Weapon;
use crate::models::ship::shield::Shield;
use crate::models::ship::armor::Armor;
use crate::models::resource::{ResourceType, Resource};
use crate::routes::move_fleet;
use rocket::serde::json::Json;
use std::sync::Once;
use std::fs;
use std::path::Path;
use crate::models::game_state::{get_game_state, save_game_state};
use crate::GLOBAL_GAME_WORLD;
use serde_json;

static INIT: Once = Once::new();

fn setup() {
    INIT.call_once(|| {
        // Clean up any existing test data
        let test_dir = Path::new("data").join("game").join("test_game");
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir).expect("Failed to clean up test directory");
        }

        // Initialize test environment
        let settings = GameSettings {
            game_id: "test_game".to_string(),
            player_name: "test_player".to_string(),
            map_width: 100,
            map_height: 100,
            map_length: 100,
            ..Default::default()
        };

        // Create necessary directories
        fs::create_dir_all(&test_dir).expect("Failed to create test directory");
        fs::create_dir_all(test_dir.join("fleets")).expect("Failed to create fleets directory");
        fs::create_dir_all(test_dir.join("star_systems")).expect("Failed to create star_systems directory");

        // Save test settings
        let settings_path = test_dir.join("settings.json");
        fs::write(settings_path, serde_json::to_string(&settings).unwrap()).expect("Failed to save test settings");

        // Create a test star system
        let system = generate_star_system_default();

        // Save the test system
        let system_path = test_dir.join("star_systems").join("Star_System_0.json");
        fs::write(system_path, serde_json::to_string(&system).unwrap()).expect("Failed to save test system");

        // Initialize game state
        let mut state = get_game_state().expect("Failed to get game state");
        state.current_game_id = Some("test_game".to_string());
        state.credits = 1000.0;
        save_game_state(state).expect("Failed to save game state");

        // Initialize global game world
        if let Ok(mut guard) = GLOBAL_GAME_WORLD.lock() {
            *guard = vec![system];
        }
    });
}

fn create_test_fleet() -> Fleet {
    let mut fleet = Fleet::new("test_player".to_string(), Position { x: 0, y: 0, z: 0 }, 1);
    fleet.current_system_id = Some(0);

    // Add a capital ship
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

#[test]
fn test_valid_move_within_system() {
    setup();
    
    let mut fleet = create_test_fleet();
    save_fleet(&fleet).expect("Failed to save test fleet");

    // Test moving within system bounds
    let move_data = MoveFleetData { x: 50, y: 50, z: 50 };
    let response = move_fleet("test_player".to_string(), 1, Json(move_data));
    
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "success");
    assert_eq!(data.current_position.x, 50);
    assert_eq!(data.current_position.y, 50);
    assert_eq!(data.current_position.z, 50);
    assert_eq!(data.current_system_id, Some(0));

    // Verify all ships moved with the fleet
    let updated_fleet = Fleet::load(&format!("Fleet_test_player_1")).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 50);
        assert_eq!(ship.position.y, 50);
        assert_eq!(ship.position.z, 50);
        // Verify ship state is preserved
        assert_eq!(ship.status, ShipStatus::Stationary);
        assert_eq!(ship.combat_state, CombatState::NotInCombat);
        assert!(ship.shields.current > 0);
        assert!(ship.armor.current > 0);
    }
}

#[test]
fn test_invalid_move_outside_bounds() {
    setup();
    
    let mut fleet = create_test_fleet();
    save_fleet(&fleet).expect("Failed to save test fleet");

    // Test moving outside system bounds
    let move_data = MoveFleetData { x: 150, y: 150, z: 150 };
    let response = move_fleet("test_player".to_string(), 1, Json(move_data));
    
    assert!(!response.success);
    assert!(response.message.contains("outside galaxy bounds"));

    // Verify fleet and ships haven't moved
    let updated_fleet = Fleet::load(&format!("Fleet_test_player_1")).expect("Failed to load updated fleet");
    assert_eq!(updated_fleet.position.x, 0);
    assert_eq!(updated_fleet.position.y, 0);
    assert_eq!(updated_fleet.position.z, 0);
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 0);
        assert_eq!(ship.position.y, 0);
        assert_eq!(ship.position.z, 0);
    }
}

#[test]
fn test_system_exit() {
    setup();
    
    let mut fleet = create_test_fleet();
    save_fleet(&fleet).expect("Failed to save test fleet");

    // Test moving to exit the system
    let move_data = MoveFleetData { x: 101, y: 0, z: 0 };
    let response = move_fleet("test_player".to_string(), 1, Json(move_data));
    
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "transition_exit");
    assert_eq!(data.current_system_id, None);

    // Verify all ships moved with the fleet
    let updated_fleet = Fleet::load(&format!("Fleet_test_player_1")).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 101);
        assert_eq!(ship.position.y, 0);
        assert_eq!(ship.position.z, 0);
        // Verify ship state is preserved
        assert_eq!(ship.status, ShipStatus::Stationary);
        assert_eq!(ship.combat_state, CombatState::NotInCombat);
        assert!(ship.shields.current > 0);
        assert!(ship.armor.current > 0);
    }
}

#[test]
fn test_system_entry() {
    setup();
    
    let mut fleet = create_test_fleet();
    fleet.current_system_id = None;
    save_fleet(&fleet).expect("Failed to save test fleet");

    // Test moving to enter a system
    let move_data = MoveFleetData { x: 0, y: 0, z: 0 };
    let response = move_fleet("test_player".to_string(), 1, Json(move_data));
    
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "transition_entry");
    assert!(data.current_system_id.is_some());

    // Verify all ships moved with the fleet
    let updated_fleet = Fleet::load(&format!("Fleet_test_player_1")).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 0);
        assert_eq!(ship.position.y, 0);
        assert_eq!(ship.position.z, 0);
        // Verify ship state is preserved
        assert_eq!(ship.status, ShipStatus::Stationary);
        assert_eq!(ship.combat_state, CombatState::NotInCombat);
        assert!(ship.shields.current > 0);
        assert!(ship.armor.current > 0);
    }
}

#[test]
fn test_move_nonexistent_fleet() {
    setup();
    
    // Test moving a fleet that doesn't exist
    let move_data = MoveFleetData { x: 0, y: 0, z: 0 };
    let response = move_fleet("nonexistent_player".to_string(), 999, Json(move_data));
    
    assert!(!response.success);
    assert!(response.message.contains("Fleet not found"));
}

#[test]
fn test_move_with_cargo() {
    setup();
    
    let mut fleet = create_test_fleet();
    save_fleet(&fleet).expect("Failed to save test fleet");

    // Test moving the fleet with cargo
    let move_data = MoveFleetData { x: 50, y: 50, z: 50 };
    let response = move_fleet("test_player".to_string(), 1, Json(move_data));
    
    assert!(response.success);
    let data = response.into_inner().data.unwrap();
    assert_eq!(data.status, "success");
    
    // Verify all ships moved with their cargo
    let updated_fleet = Fleet::load(&format!("Fleet_test_player_1")).expect("Failed to load updated fleet");
    for ship in updated_fleet.ships {
        assert_eq!(ship.position.x, 50);
        assert_eq!(ship.position.y, 50);
        assert_eq!(ship.position.z, 50);
        // Verify cargo is preserved
        assert!(!ship.cargo.is_empty());
        for resource in ship.cargo {
            assert!(resource.quantity.unwrap_or(0) > 0);
        }
    }
} 