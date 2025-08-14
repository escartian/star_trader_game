use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use rocket::request::FromParam;
use rocket::http::RawStr;
use std::fmt;

// Enumeration of the different types of resources that can be traded in the game.
#[derive(Serialize, Deserialize, Debug, Clone, EnumIter, Eq, Hash, PartialEq, Copy)]
pub enum ResourceType {
    Water,
    Food,
    Fuel,
    Minerals,
    Metals,
    Electronics,
    LuxuryGoods,
    Narcotics,

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub buy: Option<f64>,
    pub sell: Option<f64>,
    pub quantity: Option<u32>,
}
impl Resource {
    pub fn buy_price(&self) -> Option<f64> {
        self.buy
    }

    pub fn sell_price(&self) -> Option<f64> {
        self.sell
    }

    pub fn set_quantity(&mut self, quantity: u32) {
        self.quantity = Some(quantity);
    }

    pub fn new(resource_type: ResourceType, quantity: u32) -> Self {
        Resource {
            resource_type,
            quantity: Some(quantity),
            buy: None,
            sell: None,
        }
    }
}

// Generate a vector of random resources for the Traders
pub fn generate_resources() -> Vec<Resource> {
    let mut inventory = vec![]; // Create an empty vector to hold the resources

    let resource_types: Vec<ResourceType> = ResourceType::iter().collect(); // Define a vector of all possible resource types

    let mut rng = thread_rng(); // Create a new random number generator

    // Loop through each resource type and generate a random Resource object for it
    for resource_type in resource_types {
        let buy_price: Option<f64> = if rng.gen_bool(0.7) {
            // Generate a random buy price with a 70% chance
            Some(rng.gen_range(0.5..8.0) as f64)
        } else {
            None // 30% chance of no buy price
        };
        let sell_price: Option<f64> = if rng.gen_bool(0.7) {
            // Generate a random sell price with a 70% chance
            Some(rng.gen_range(1.0..10.0) as f64)
        } else {
            None // 30% chance of no sell price
        };
        let quantity: Option<u32> = if rng.gen_bool(0.5) {
            // Generate a random quantity with a 50% chance
            Some(rng.gen_range(10..100))
        } else {
            if sell_price.is_some() {
                Some(rng.gen_range(10..100))
            }
            else{
                None // 50% chance of no quantity
            }
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


/** 
* Generates a vector of Resource objects with random quantities, but without any buy or sell prices. This is used to generate the resources for the player at the start of the game.
*
* # Returns
* A vector of Resource objects with random quantities, but without any buy or sell prices.
**/
pub fn generate_resources_no_trade() -> Vec<Resource> {
    let mut inventory = vec![]; // Create an empty vector to hold the resources

    let resource_types: Vec<ResourceType> = ResourceType::iter().collect(); // Define a vector of all possible resource types

    let mut rng = thread_rng(); // Create a new random number generator

    // Loop through each resource type and generate a random Resource object for it
    for resource_type in resource_types {
        let quantity: Option<u32> = if rng.gen_bool(0.7) {
            // Generate a random quantity with a 70% chance
            Some(rng.gen_range(10..100))
        } else {
            None // 30% chance of no quantity
        };

        let resource = Resource {
            // Create a new Resource object with the generated values
            resource_type,
            buy: None,
            sell: None,
            quantity,
        };
        inventory.push(resource); // Add the new Resource object to the inventory vector
    }

    inventory // Return the completed inventory vector
}

impl<'r> FromParam<'r> for ResourceType {
    type Error = &'r str;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        match param {
            "Water" => Ok(ResourceType::Water),
            "Food" => Ok(ResourceType::Food),
            "Fuel" => Ok(ResourceType::Fuel),
            "Minerals" => Ok(ResourceType::Minerals),
            "Metals" => Ok(ResourceType::Metals),
            "Electronics" => Ok(ResourceType::Electronics),
            "LuxuryGoods" => Ok(ResourceType::LuxuryGoods),
            "Narcotics" => Ok(ResourceType::Narcotics),
            _ => Err(param),
        }
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Water => write!(f, "Water"),
            ResourceType::Food => write!(f, "Food"),
            ResourceType::Fuel => write!(f, "Fuel"),
            ResourceType::Minerals => write!(f, "Minerals"),
            ResourceType::Metals => write!(f, "Metals"),
            ResourceType::Electronics => write!(f, "Electronics"),
            ResourceType::LuxuryGoods => write!(f, "LuxuryGoods"),
            ResourceType::Narcotics => write!(f, "Narcotics"),
        }
    }
}