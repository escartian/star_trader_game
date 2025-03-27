use crate::constants::MAX_COMBAT_TIME;
use crate::models::fleet::Fleet;
use crate::models::ship::ship::{Ship, ShipType, ShipSize, ShipEngine};
use crate::models::position::Position;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct CombatResult {
    pub attacker_losses: Vec<Ship>,
    pub defender_losses: Vec<Ship>,
    pub attacker_victory: bool,
    pub combat_log: Vec<String>,
}

pub fn calculate_combat_power(ship: &Ship) -> f64 {
    let base_power = match ship.specialization {
        ShipType::Fighter => 10.0,
        ShipType::Battleship => 25.0,
        ShipType::Capital => 50.0,
        ShipType::Freighter => 5.0,
        ShipType::Explorer => 8.0,
        ShipType::Shuttle => 3.0,
    };

    let size_multiplier = match ship.size {
        ShipSize::Tiny => 0.5,
        ShipSize::Small => 0.75,
        ShipSize::Medium => 1.0,
        ShipSize::Large => 1.5,
        ShipSize::Huge => 2.0,
        ShipSize::Planetary => 3.0,
    };

    let engine_multiplier = match ship.engine {
        ShipEngine::Basic => 1.0,
        ShipEngine::Advanced => 1.2,
        ShipEngine::Experimental => 1.5,
    };

    base_power * size_multiplier * engine_multiplier
}

pub fn calculate_fleet_power(fleet: &Fleet) -> f64 {
    fleet.ships.iter()
        .map(|ship| calculate_combat_power(ship))
        .sum()
}

pub fn auto_resolve_ship_combat(attacker: &mut Fleet, defender: &mut Fleet) -> CombatResult {
    let mut combat_log = Vec::new();
    let mut attacker_losses = Vec::new();
    let mut defender_losses = Vec::new();

    // Calculate initial fleet powers
    let attacker_power = calculate_fleet_power(attacker);
    let defender_power = calculate_fleet_power(defender);
    
    combat_log.push(format!("Combat initiated between {} and {}", attacker.name, defender.name));
    combat_log.push(format!("Initial fleet powers - Attacker: {:.1}, Defender: {:.1}", attacker_power, defender_power));

    // Determine victory based on fleet power and some randomness
    let mut rng = rand::thread_rng();
    let random_factor = rng.gen_range(0.8..1.2); // 20% random variation
    let attacker_victory = attacker_power * random_factor > defender_power;

    if attacker_victory {
        // Attacker wins, defender takes more losses
        let defender_loss_ratio = rng.gen_range(0.6..0.9);
        let attacker_loss_ratio = rng.gen_range(0.2..0.4);

        // Apply losses
        let defender_loss_count = (defender.ships.len() as f64 * defender_loss_ratio) as usize;
        let attacker_loss_count = (attacker.ships.len() as f64 * attacker_loss_ratio) as usize;

        for _ in 0..defender_loss_count {
            if let Some(ship) = defender.ships.pop() {
                defender_losses.push(ship);
            }
        }

        for _ in 0..attacker_loss_count {
            if let Some(ship) = attacker.ships.pop() {
                attacker_losses.push(ship);
            }
        }

        combat_log.push(format!("Attacker victorious! Defender lost {} ships, Attacker lost {} ships", 
            defender_loss_count, attacker_loss_count));
    } else {
        // Defender wins, attacker takes more losses
        let attacker_loss_ratio = rng.gen_range(0.6..0.9);
        let defender_loss_ratio = rng.gen_range(0.2..0.4);

        // Apply losses
        let attacker_loss_count = (attacker.ships.len() as f64 * attacker_loss_ratio) as usize;
        let defender_loss_count = (defender.ships.len() as f64 * defender_loss_ratio) as usize;

        for _ in 0..attacker_loss_count {
            if let Some(ship) = attacker.ships.pop() {
                attacker_losses.push(ship);
            }
        }

        for _ in 0..defender_loss_count {
            if let Some(ship) = defender.ships.pop() {
                defender_losses.push(ship);
            }
        }

        combat_log.push(format!("Defender victorious! Attacker lost {} ships, Defender lost {} ships", 
            attacker_loss_count, defender_loss_count));
    }

    CombatResult {
        attacker_losses,
        defender_losses,
        attacker_victory,
        combat_log,
    }
}

pub fn can_engage_combat(attacker: &Fleet, defender: &Fleet) -> bool {
    // Check if fleets are at the same position
    attacker.position == defender.position
}
