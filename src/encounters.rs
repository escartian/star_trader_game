use crate::models::fleet::Fleet;
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine, ShipStatus, CombatState};
use crate::models::ship::armor::Armor;
use crate::models::ship::weapon::Weapon;
use crate::models::ship::shield::Shield;
use crate::models::position::Position;
use rand::Rng;
use crate::models::resource::{ResourceType, Resource};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncounterFleet {
    pub name: String,
    pub owner_id: String,
    pub ships: Vec<Ship>,
    pub position: Position,
}

/// Generates a random encounter fleet at the specified position.
/// The fleet type and number of ships are determined by weighted probabilities.
/// 
/// # Arguments
/// * `position` - The position where the encounter fleet should be generated
/// 
/// # Returns
/// An `EncounterFleet` containing randomly generated ships based on the fleet type
pub fn generate_encounter_fleet(position: Position) -> EncounterFleet {
    let mut rng = rand::thread_rng();
    
    // Generate a random fleet type with adjusted probabilities for combat encounters
    let fleet_types = vec![
        ("Pirate", 1, 3, 0.4),    // 40% chance, 1-3 ships
        ("Trader", 2, 4, 0.2),    // 20% chance, 2-4 ships
        ("Military", 3, 5, 0.3),  // 30% chance, 3-5 ships
        ("Mercenary", 2, 4, 0.1), // 10% chance, 2-4 ships
    ];
    
    // Select fleet type based on probability
    let roll = rand::random::<f64>();
    let mut cumulative = 0.0;
    let (fleet_type, min_ships, max_ships, _) = fleet_types.iter()
        .find(|(_, _, _, prob)| {
            cumulative += prob;
            roll <= cumulative
        })
        .unwrap_or(&fleet_types[0]);
    
    let ship_count = rng.gen_range(*min_ships..=*max_ships);
    
    let mut ships = Vec::new();
    
    // Generate ships based on fleet type
    for _ in 0..ship_count {
        let ship = match *fleet_type {
            "Pirate" => generate_pirate_ship(),
            "Trader" => generate_trader_ship(),
            "Military" => generate_military_ship(),
            "Mercenary" => generate_mercenary_ship(),
            _ => generate_trader_ship(),
        };
        ships.push(ship);
    }
    
    EncounterFleet {
        name: format!("Fleet_{}_{}", fleet_type, rng.gen_range(1000..9999)),
        owner_id: fleet_type.to_string(),
        ships,
        position,
    }
}

/// Calculates the base price for a ship based on its attributes
fn calculate_ship_price(ship: &Ship) -> f32 {
    let base_price = match ship.size {
        ShipSize::Tiny => 1000.0,
        ShipSize::Small => 2500.0,
        ShipSize::Medium => 5000.0,
        ShipSize::Large => 10000.0,
        ShipSize::Huge => 20000.0,
        ShipSize::Planetary => 50000.0,
    };

    let specialization_multiplier = match ship.specialization {
        ShipType::Fighter => 1.2,
        ShipType::Battleship => 2.0,
        ShipType::Freighter => 1.5,
        ShipType::Explorer => 1.8,
        ShipType::Shuttle => 0.8,
        ShipType::Capital => 3.0,
    };

    let engine_multiplier = match ship.engine {
        ShipEngine::Basic => 1.0,
        ShipEngine::Advanced => 1.5,
        ShipEngine::Experimental => 2.0,
    };

    base_price * specialization_multiplier * engine_multiplier
}

/// Generates a pirate ship with aggressive combat capabilities.
/// 
/// # Returns
/// A `Ship` configured for combat with high damage weapons and balanced defenses
fn generate_pirate_ship() -> Ship {
    let mut rng = rand::thread_rng();
    let mut ship = Ship {
        name: format!("Pirate_Ship_{}", rng.gen_range(1000..9999)),
        owner: format!("Pirate_{}", rng.gen_range(1000..9999)),
        position: Position { x: 0, y: 0, z: 0 },
        specialization: ShipType::Fighter,
        size: ShipSize::Medium,
        engine: ShipEngine::Advanced,
        status: ShipStatus::Stationary,
        hp: 100,
        combat_state: CombatState::Aggressive,
        cargo: vec![],
        shields: Shield::new(100),
        weapons: vec![
            Weapon::NeutronBeam { damage: 50 },
            Weapon::QuantumEntanglementTorpedo { damage: 30 },
        ],
        armor: Armor::new(75),
        price: None, // Pirate ships are not for sale
    };
    ship.price = Some(calculate_ship_price(&ship));
    ship
}

/// Generates a trader ship with cargo capacity and basic defenses.
/// 
/// # Returns
/// A `Ship` configured for trading with cargo space and minimal combat capabilities
fn generate_trader_ship() -> Ship {
    let mut rng = rand::thread_rng();
    let mut cargo = Vec::new();
    
    // Generate 2-4 random resources for each trader ship
    let resource_count = rng.gen_range(2..=4);
    let resource_types = vec![
        ResourceType::Water,
        ResourceType::Food,
        ResourceType::Fuel,
        ResourceType::Minerals,
        ResourceType::Metals,
        ResourceType::Electronics,
        ResourceType::LuxuryGoods,
        ResourceType::Narcotics,
    ];
    
    // Randomly select resources
    let mut available_types = resource_types.clone();
    for _ in 0..resource_count {
        if available_types.is_empty() {
            break;
        }
        let index = rng.gen_range(0..available_types.len());
        let resource_type = available_types.remove(index);
        
        // Generate random quantity between 10 and 100
        let quantity = rng.gen_range(10..=100);
        
        cargo.push(Resource {
            resource_type,
            quantity: Some(quantity),
            buy: Some(rng.gen_range(10.0..=50.0)),
            sell: Some(rng.gen_range(50.0..=100.0)),
        });
    }

    let mut ship = Ship {
        name: format!("Trader_Ship_{}", rng.gen_range(1000..9999)),
        owner: format!("Trader_{}", rng.gen_range(1000..9999)),
        position: Position { x: 0, y: 0, z: 0 },
        specialization: ShipType::Freighter,
        size: ShipSize::Large,
        engine: ShipEngine::Basic,
        status: ShipStatus::Stationary,
        hp: 80,
        combat_state: CombatState::Passive,
        cargo,
        shields: Shield::new(50),
        weapons: vec![
            Weapon::GravitonPulse { damage: 20 },
        ],
        armor: Armor::new(50),
        price: None, // Will be set below
    };
    ship.price = Some(calculate_ship_price(&ship));
    ship
}

/// Generates a military ship with strong combat capabilities.
/// 
/// # Returns
/// A `Ship` configured for military operations with powerful weapons and heavy defenses
fn generate_military_ship() -> Ship {
    let mut rng = rand::thread_rng();
    let mut ship = Ship {
        name: format!("Military_Ship_{}", rng.gen_range(1000..9999)),
        owner: format!("Military_{}", rng.gen_range(1000..9999)),
        position: Position { x: 0, y: 0, z: 0 },
        specialization: ShipType::Battleship,
        size: ShipSize::Large,
        engine: ShipEngine::Advanced,
        status: ShipStatus::Stationary,
        hp: 200,
        combat_state: CombatState::Aggressive,
        cargo: vec![],
        shields: Shield::new(200),
        weapons: vec![
            Weapon::PhotonSingularityBeam { damage: 100 },
            Weapon::QuantumEntanglementTorpedo { damage: 80 },
            Weapon::MagneticResonanceDisruptor { damage: 150 },
        ],
        armor: Armor::new(150),
        price: None, // Will be set below
    };
    ship.price = Some(calculate_ship_price(&ship));
    ship
}

/// Generates a mercenary ship with balanced combat capabilities.
/// 
/// # Returns
/// A `Ship` configured for mercenary operations with versatile weapons and defenses
fn generate_mercenary_ship() -> Ship {
    let mut rng = rand::thread_rng();
    let mut ship = Ship {
        name: format!("Mercenary_Ship_{}", rng.gen_range(1000..9999)),
        owner: format!("Mercenary_{}", rng.gen_range(1000..9999)),
        position: Position { x: 0, y: 0, z: 0 },
        specialization: ShipType::Fighter,
        size: ShipSize::Medium,
        engine: ShipEngine::Advanced,
        status: ShipStatus::Stationary,
        hp: 150,
        combat_state: CombatState::Default,
        cargo: vec![],
        shields: Shield::new(75),
        weapons: vec![
            Weapon::NeutronBeam { damage: 75 },
            Weapon::QuantumEntanglementTorpedo { damage: 50 },
        ],
        armor: Armor::new(100),
        price: None, // Mercenary ships are not for sale
    };
    ship.price = Some(calculate_ship_price(&ship));
    ship
} 