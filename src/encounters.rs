use crate::models::fleet::Fleet;
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine, ShipStatus, CombatState};
use crate::models::ship::armor::Armor;
use crate::models::ship::weapon::Weapon;
use crate::models::ship::shield::Shield;
use crate::models::position::Position;
use rand::Rng;

pub struct EncounterFleet {
    pub name: String,
    pub owner_id: String,
    pub ships: Vec<Ship>,
    pub position: Position,
}

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
        name: format!("{}_Fleet_{}", fleet_type, rng.gen_range(1000..9999)),
        owner_id: fleet_type.to_string(),
        ships,
        position,
    }
}

fn generate_pirate_ship() -> Ship {
    let mut rng = rand::thread_rng();
    Ship {
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
    }
}

fn generate_trader_ship() -> Ship {
    let mut rng = rand::thread_rng();
    Ship {
        name: format!("Trader_Ship_{}", rng.gen_range(1000..9999)),
        owner: format!("Trader_{}", rng.gen_range(1000..9999)),
        position: Position { x: 0, y: 0, z: 0 },
        specialization: ShipType::Freighter,
        size: ShipSize::Large,
        engine: ShipEngine::Basic,
        status: ShipStatus::Stationary,
        hp: 80,
        combat_state: CombatState::Passive,
        cargo: vec![],
        shields: Shield::new(50),
        weapons: vec![
            Weapon::GravitonPulse { damage: 20 },
        ],
        armor: Armor::new(50),
    }
}

fn generate_military_ship() -> Ship {
    let mut rng = rand::thread_rng();
    Ship {
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
    }
}

fn generate_mercenary_ship() -> Ship {
    let mut rng = rand::thread_rng();
    Ship {
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
    }
} 