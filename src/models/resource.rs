use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use std::slice::SliceIndex;

// Enumeration of the different types of resources that can be traded in the game.
#[derive(Serialize, Deserialize, Debug, Clone, EnumIter, Eq, Hash, PartialEq)]
pub enum ResourceType {
    Water,
    Food,
    Fuel,
    Metals,
    Electronics,
    LuxuryGoods,
    Narcotics,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub buy: Option<f32>,
    pub sell: Option<f32>,
    pub quantity: Option<u32>,
}
impl Resource {
    pub fn buy_price(&self) -> Option<f32> {
        self.buy
    }

    pub fn sell_price(&self) -> Option<f32> {
        self.sell
    }
}

// Generate a vector of random resources for the Traders
pub fn generate_resources() -> Vec<Resource> {
    let mut inventory = vec![]; // Create an empty vector to hold the resources

    let resource_types: Vec<ResourceType> = ResourceType::iter().collect(); // Define a vector of all possible resource types

    let mut rng = thread_rng(); // Create a new random number generator

    // Loop through each resource type and generate a random Resource object for it
    for resource_type in resource_types {
        let buy_price: Option<f32> = if rng.gen_bool(0.7) {
            // Generate a random buy price with a 70% chance
            Some(rng.gen_range(0.5..8.0))
        } else {
            None // 30% chance of no buy price
        };
        let sell_price: Option<f32> = if rng.gen_bool(0.7) {
            // Generate a random sell price with a 70% chance
            Some(rng.gen_range(1.0..10.0))
        } else {
            None // 30% chance of no sell price
        };
        let quantity: Option<u32> = if rng.gen_bool(0.5) {
            // Generate a random quantity with a 50% chance
            Some(rng.gen_range(10..100))
        } else {
            None // 50% chance of no quantity
        };

        let resource = Resource {
            // Create a new Resource object with the generated values
            resource_type,
            buy: buy_price,
            sell: sell_price,
            quantity,
        };
        inventory.push(resource); // Add the new Resource object to the inventory vector
    }

    inventory // Return the completed inventory vector
}
