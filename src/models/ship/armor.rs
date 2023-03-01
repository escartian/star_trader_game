use serde::{Serialize, Deserialize};

use crate::constants::PRINT_DEBUG;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Armor {
    pub durability: i32,
}

impl Armor {
    pub fn new(durability: i32) -> Self {
        Self { durability }
    }

    pub fn calculate_damage(&mut self, damage: i32) -> i32 {
        if PRINT_DEBUG {
            println!(
                "Calculating damage with armor durability: {}",
                self.durability
            );
            println!("Incoming damage: {}", damage);
        }
        let remaining_durability = self.durability - damage;
        let damage_absorbed = if remaining_durability < 0 {
            self.durability
        } else {
            damage
        };
        if PRINT_DEBUG {
            println!("Armor absorbed {} damage", damage_absorbed);
        }
        self.durability = remaining_durability.max(0);
        if PRINT_DEBUG {
            println!("Remaining armor durability: {}", self.durability);
        }
        damage - damage_absorbed
    }
}
