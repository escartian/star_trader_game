use serde::{Deserialize, Serialize};
use crate::constants::PRINT_DEBUG;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Shield {
    pub capacity: i32,
    pub current: i32,
    pub regen: i32,
}

impl Shield {
    pub fn new(capacity: i32) -> Self {
        Self {
            capacity,
            current: capacity,
            regen: capacity / 10, // 10% of capacity per tick
        }
    }

    pub fn calculate_damage(&mut self, damage: i32) -> i32 {
        if PRINT_DEBUG {
            println!("Calculating damage with shield current: {}", self.current);
            println!("Incoming damage: {}", damage);
        }
        
        let reduction_percentage = 1.00;
        let reduction = (reduction_percentage / 100.0) * self.current as f32;
        let reduced_damage = (damage as f32 - reduction).max(0.0) as i32;
        
        // Reduce shield current
        self.current = (self.current - damage).max(0);
        
        // Regenerate shields
        self.current = (self.current + self.regen).min(self.capacity);
        
        reduced_damage
    }
}
