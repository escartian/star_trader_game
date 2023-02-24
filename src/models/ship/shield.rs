use serde::{Deserialize, Serialize};
use crate::constants::PRINT_DEBUG;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shield {
    pub strength: i32,
}

impl Shield {
    pub fn new(strength: i32) -> Self {
        Self { strength }
    }

    pub fn calculate_damage(&mut self, damage: i32, shield_strength: i32) -> i32 {
        if PRINT_DEBUG {
            println!("Calculating damage with shield strength: {}", self.strength);
            println!("Incoming damage: {}", damage);
        }
        let reduction_percentage = 1.00;
        let reduction = (reduction_percentage / 100.0) * shield_strength as f32;
        let remaining_shield = (shield_strength as f32 - damage as f32 + reduction).max(0.0) as i32;
        let damage_absorbed = shield_strength - remaining_shield;
        if PRINT_DEBUG {
            println!("Shield absorbed {} damage", damage_absorbed);
        }
        self.strength = remaining_shield;
        if PRINT_DEBUG {
            println!("Remaining shield strength: {}", self.strength);
        }
        damage - damage_absorbed
    }
}
