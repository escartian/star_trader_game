use crate::{constants::PRINT_DEBUG, models::planet::PlanetTrait};
use core::fmt;
use rand::{distributions::Standard, prelude::Distribution, Rng};
use serde::{Deserialize, Serialize};
use std::{fs::{self, File}, path::Path};
use crate::models::position::Position;

use super::{
    planet::Planet,
    player::Player,
    resource::{generate_resources, Resource, ResourceType},
};

#[derive(Serialize, Deserialize)]
struct Quote {
    personality: String,
    danger_level: String,
    quote: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TraderPersonality {
    Friendly,
    Neutral,
    Aggressive,
}
impl Distribution<TraderPersonality> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TraderPersonality {
        match rng.gen_range(0..3) {
            0 => TraderPersonality::Friendly,
            1 => TraderPersonality::Aggressive,
            _ => TraderPersonality::Neutral,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trader {
    pub name: String,
    pub position: Position,
    pub credits: f32,
    pub inventory: Vec<(String, u32)>, // (resource_type, quantity)
    pub personality: TraderPersonality,
    pub resources: Vec<Resource>,
}

impl fmt::Display for TraderPersonality {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TraderPersonality::Friendly => write!(f, "Friendly"),
            TraderPersonality::Neutral => write!(f, "Neutral"),
            TraderPersonality::Aggressive => write!(f, "Aggressive"),
        }
    }
}

impl Trader {
    pub fn is_null(&self) -> bool {
        &self as *const _ == std::ptr::null()
    }

    pub fn get_opening_line(&self, planet: &Planet) -> String {
        let data_path = Path::new("data")
            .join("quotes")
            .join("trader")
            .join("TraderDangerHello.json");
        println!("{}", data_path.display());

        let file = match File::open(data_path) {
            Ok(file) => file,
            Err(_) => return "Quote not found".to_string(),
        };
        let quotes: Vec<Quote> = match serde_json::from_reader(file) {
            Ok(quotes) => quotes,
            Err(_) => return "Quote not found".to_string(),
        };

        let danger_level = planet.get_danger();

        for quote in quotes {
            if quote.personality == self.personality.to_string()
                && quote.danger_level == danger_level.to_string()
            {
                return quote.quote.clone();
            }
        }
        "Quote not found".to_string()
    }

    pub fn new(name: String, position: Position, credits: f32) -> Self {
        let mut rng = rand::thread_rng();
        Trader {
            name,
            position,
            credits,
            inventory: Vec::new(),
            personality: match rng.gen_range(0..3) {
                0 => TraderPersonality::Friendly,
                1 => TraderPersonality::Aggressive,
                _ => TraderPersonality::Neutral,
            },
            resources: Vec::new(),
        }
    }

    // Sells a resource from the player to the trader
    pub fn sell_resource(
        &mut self,
        resource_type: ResourceType,
        quantity: u32,
        player: &mut Player,
    ) -> Result<(), String> {
        // Check if the trader sells this type of resource
        if let Some(resource) = self
            .resources
            .iter()
            .find(|r| r.resource_type == resource_type)
        {
            if let Some(sell_price) = resource.sell_price() {
                // Check if the player has this type of resource
                if let Some(player_resource) = player
                    .resources
                    .iter_mut()
                    .find(|r| r.resource_type == resource_type)
                {
                    // Check if the player has enough quantity to sell
                    if player_resource.quantity >= Some(quantity) {
                        let cost = quantity as f32 * sell_price;
                        player.credits -= cost;
                        //self.personality.on_sell_successful(&resource_type);
                        //self.personality.on_buy_attempt(&resource_type);
                        let mut player_quantity = player_resource.quantity.unwrap();
                        player_quantity -= quantity;
                        player_resource.quantity = Some(player_quantity);
                        self.credits += cost; // add earnings to player's credits
                        if PRINT_DEBUG {
                            println!("Successfully sold {:?} {:?} to trader. Player's credits: {:?}. Trader's credits: {:?}.", quantity, resource_type, self.credits, player.credits);
                        }
                        return Ok(());
                    } else {
                        if PRINT_DEBUG {
                            println!("Player does not have enough of {:?} to sell to trader. Available quantity: {:?}.", resource_type, player_resource.quantity);
                        }
                        return Err(format!(
                            "Player does not have enough of {:?} to sell.",
                            resource_type
                        ));
                    }
                } else {
                    if PRINT_DEBUG {
                        println!(
                            "Player does not have any {:?} to sell to trader.",
                            resource_type
                        );
                    }
                    return Err(format!(
                        "Player does not have any {:?} to sell.",
                        resource_type
                    ));
                }
            } else {
                // handle the case where sell_price is None
                if PRINT_DEBUG {
                    println!("Trader does not sell {:?}.", resource_type);
                }
                return Err(format!("Trader does not sell {:?}.", resource_type));
            }
        } else {
            if PRINT_DEBUG {
                println!("Trader does not sell {:?}.", resource_type);
            }
            return Err(format!("Trader does not sell {:?}.", resource_type));
        }
    }

    pub fn buy_resource(
        &mut self,
        resource_type: ResourceType,
        quantity: u32,
        player: &mut Player,
    ) -> Result<(), String> {
        // Check if the trader buys this type of resource
        if let Some(resource) = self
            .resources
            .iter()
            .find(|r| r.resource_type == resource_type)
        {
            // Check if the trader has a buy price for this resource
            if let Some(buy_price) = resource.buy_price() {
                // Calculate the total cost of the purchase


                
                //self.calculate_price(resource_type, quantity);


                let cost = quantity as f32 * buy_price;

                // Check if the player has enough credits to buy
                if player.credits >= cost {
                    // Deduct the cost from the player's credits and add it to the seller's credits
                    player.credits -= cost;
                    self.credits += cost;

                    // Increase the player's quantity of the purchased resource
                    let mut player_quantity = 0;
                    if let Some(player_resource) = player
                        .resources
                        .iter_mut()
                        .find(|r| r.resource_type == resource_type)
                    {
                        // The player already has some of this resource, so add to their existing quantity
                        if player_resource.quantity >= Some(quantity) {
                            player_quantity = player_resource.quantity.unwrap();
                        } else {
                            // The player has none of this resource, so start with a quantity of zero
                            player_quantity = 0;
                        }
                        player_quantity += quantity;
                        player_resource.quantity = Some(player_quantity);
                    } else {
                        // The player does not have any of this resource to begin with, so create a new Resource object
                        player
                            .resources
                            .push(Resource::new(resource_type, quantity));
                    }

                    // Debug print statement to indicate that the purchase was successful
                    if PRINT_DEBUG {
                        println!(
                            "Purchase successful: {} {:?} bought for {} credits",
                            quantity, resource_type, cost
                        );
                    }
                    return Ok(());
                } else {
                    // Debug print statement to indicate that the purchase was unsuccessful due to lack of credits
                    if PRINT_DEBUG {
                        println!("Purchase unsuccessful: player does not have enough credits to buy {:?}", resource_type);
                    }
                    return Err(format!(
                        "Player does not have enough credits to buy {:?}.",
                        resource_type
                    ));
                }
            } else {
                // Debug print statement to indicate that the purchase was unsuccessful due to lack of a buy price for this resource
                if PRINT_DEBUG {
                    println!(
                        "Purchase unsuccessful: trader does not buy {:?}",
                        resource_type
                    );
                }
                return Err(format!("Trader does not buy {:?}.", resource_type));
            }
        } else {
            // Debug print statement to indicate that the purchase was unsuccessful due to the trader not selling this resource
            if PRINT_DEBUG {
                println!(
                    "Purchase unsuccessful: trader does not sell {:?}",
                    resource_type
                );
            }
            return Err(format!("Trader does not sell {:?}.", resource_type));
        }
    }

    //Traders can trade amongst themselved with the below (primary purpose is for testing trades)

    // Sells a resource from the player to the trader
    pub fn sell_resource_trader_to_trader(
        &mut self,
        resource_type: ResourceType,
        quantity: u32,
        trader: &mut Trader,
    ) -> Result<(), String> {
        // Check if the trader sells this type of resource
        if let Some(resource) = self
            .resources
            .iter()
            .find(|r| r.resource_type == resource_type)
        {
            if let Some(sell_price) = resource.sell_price() {
                // Check if the player has this type of resource
                if let Some(player_resource) = trader
                    .resources
                    .iter_mut()
                    .find(|r| r.resource_type == resource_type)
                {
                    // Check if the player has enough quantity to sell
                    if player_resource.quantity >= Some(quantity) {
                        let cost = quantity as f32 * sell_price;
                        trader.credits -= cost;
                        //self.personality.on_sell_successful(&resource_type);
                        //self.personality.on_buy_attempt(&resource_type);
                        let mut player_quantity = player_resource.quantity.unwrap();
                        player_quantity -= quantity;
                        player_resource.quantity = Some(player_quantity);
                        self.credits += cost; // add earnings to player's credits
                        if PRINT_DEBUG {
                            println!("Successfully sold {:?} {:?} to trader. Player's credits: {:?}. Trader's credits: {:?}.", quantity, resource_type, self.credits, trader.credits);
                        }
                        return Ok(());
                    } else {
                        if PRINT_DEBUG {
                            println!("Player does not have enough of {:?} to sell to trader. Available quantity: {:?}.", resource_type, player_resource.quantity);
                        }
                        return Err(format!(
                            "Player does not have enough of {:?} to sell.",
                            resource_type
                        ));
                    }
                } else {
                    if PRINT_DEBUG {
                        println!(
                            "Player does not have any {:?} to sell to trader.",
                            resource_type
                        );
                    }
                    return Err(format!(
                        "Player does not have any {:?} to sell.",
                        resource_type
                    ));
                }
            } else {
                // handle the case where sell_price is None
                if PRINT_DEBUG {
                    println!("Trader does not sell {:?}.", resource_type);
                }
                return Err(format!("Trader does not sell {:?}.", resource_type));
            }
        } else {
            if PRINT_DEBUG {
                println!("Trader does not sell {:?}.", resource_type);
            }
            return Err(format!("Trader does not sell {:?}.", resource_type));
        }
    }
    pub fn buy_resource_trader_to_trader(
        &mut self,
        resource_type: ResourceType,
        quantity: u32,
        trader: &mut Trader,
    ) -> Result<(), String> {
        // Check if the trader buys this type of resource
        if let Some(resource) = self
            .resources
            .iter()
            .find(|r| r.resource_type == resource_type)
        {
            // Check if the trader has a buy price for this resource
            if let Some(buy_price) = resource.buy_price() {
                // Calculate the total cost of the purchase
                let cost = quantity as f32 * buy_price;

                // Check if the player has enough credits to buy
                if trader.credits >= cost {
                    // Deduct the cost from the player's credits and add it to the seller's credits
                    trader.credits -= cost;
                    self.credits += cost;

                    // Increase the player's quantity of the purchased resource
                    let mut player_quantity = 0;
                    if let Some(player_resource) = trader
                        .resources
                        .iter_mut()
                        .find(|r| r.resource_type == resource_type)
                    {
                        // The player already has some of this resource, so add to their existing quantity
                        if player_resource.quantity >= Some(quantity) {
                            player_quantity = player_resource.quantity.unwrap();
                        } else {
                            // The player has none of this resource, so start with a quantity of zero
                            player_quantity = 0;
                        }
                        player_quantity += quantity;
                        player_resource.quantity = Some(player_quantity);
                    } else {
                        // The player does not have any of this resource to begin with, so create a new Resource object
                        trader
                            .resources
                            .push(Resource::new(resource_type, quantity));
                    }

                    // Debug print statement to indicate that the purchase was successful
                    if PRINT_DEBUG {
                        println!(
                            "Purchase successful: {} {:?} bought for {} credits",
                            quantity, resource_type, cost
                        );
                    }
                    return Ok(());
                } else {
                    // Debug print statement to indicate that the purchase was unsuccessful due to lack of credits
                    if PRINT_DEBUG {
                        println!("Purchase unsuccessful: player does not have enough credits to buy {:?}", resource_type);
                    }
                    return Err(format!(
                        "Player does not have enough credits to buy {:?}.",
                        resource_type
                    ));
                }
            } else {
                // Debug print statement to indicate that the purchase was unsuccessful due to lack of a buy price for this resource
                if PRINT_DEBUG {
                    println!(
                        "Purchase unsuccessful: trader does not buy {:?}",
                        resource_type
                    );
                }
                return Err(format!("Trader does not buy {:?}.", resource_type));
            }
        } else {
            // Debug print statement to indicate that the purchase was unsuccessful due to the trader not selling this resource
            if PRINT_DEBUG {
                println!(
                    "Purchase unsuccessful: trader does not sell {:?}",
                    resource_type
                );
            }
            return Err(format!("Trader does not sell {:?}.", resource_type));
        }
    }
}
