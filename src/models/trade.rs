use crate::models::resource::{Resource, ResourceType};
use crate::models::player::Player;
use crate::models::market::Market;
use crate::models::fleet::Fleet;
use crate::models::trader::Trader;
use crate::constants::PRINT_DEBUG;
use serde::Deserialize;
use crate::models::planet::Planet;
use crate::models::ship::ship::Ship;
use std::error::Error;

#[derive(Debug, PartialEq)]
pub enum TradeAction {
    Buy { resource_type: ResourceType, quantity: u32 },
    Sell { resource_type: ResourceType, quantity: u32 },
}

#[derive(Debug, PartialEq)]
pub enum TradeResult {
    Success,
    InsufficientResourcesBuying,
    InsufficientResourcesSelling,
    InvalidResource,
    TransactionFailed,
    TraderNotFound,
}

#[derive(Deserialize)]
pub struct ResourceTradeData {
    pub resource_type: ResourceType,
    pub quantity: u32,
    pub fleet_name: Option<String>,  // Optional: target fleet whose cargo is affected
    #[serde(default)]
    pub distribution_mode: Option<String>, // "first" | "even" | "specific"
    #[serde(default)]
    pub allocations: Option<Vec<ShipAllocation>>, // for mode="specific"
}

#[derive(Deserialize)]
pub struct ShipAllocation {
    pub ship_index: usize,
    pub quantity: u32,
}

#[derive(Deserialize)]
pub struct ShipTradeData {
    pub ship_index: usize,
    pub fleet_name: Option<String>
}

#[derive(Deserialize)]
pub struct ShipTradeInData {
    pub ship_index: usize,
    pub fleet_name: Option<String>,
    pub trade_in_ship_index: Option<usize>
}

pub fn trade_with_trader(player: &mut Player, trader: &mut Trader, action: TradeAction) -> TradeResult {
    // Check if the trader reference is valid
    if trader.is_null() {
        return TradeResult::TraderNotFound;
    }
    
    match action {
        TradeAction::Buy { resource_type, quantity } => {
            // Check for invalid quantity
            if quantity == 0 {
                return TradeResult::InvalidResource;
            }
            // Attempt to buy resource and handle potential errors
            match trader.buy_resource(resource_type, quantity, player) {
                Ok(_) => TradeResult::Success,
                Err(e) => {
                    eprintln!("Error during buying resource: {}", e);
                    TradeResult::TransactionFailed
                }
            }
        }
        TradeAction::Sell { resource_type, quantity } => {
            // Check for invalid quantity
            if quantity == 0 {
                return TradeResult::InvalidResource;
            }
            // Attempt to sell resource and handle potential errors
            match trader.sell_resource(resource_type, quantity, player) {
                Ok(_) => TradeResult::Success,
                Err(e) => {
                    eprintln!("Error during selling resource: {}", e);
                    TradeResult::TransactionFailed
                }
            }
        }
    }
}

pub fn buy_from_planet(
    planet: &mut Planet,
    player: &mut Player,
    resource_type: ResourceType,
    quantity: u32,
    system_id: usize,
    planet_id: usize
) -> Result<(), String> {
    let mut market = Market::load(system_id, planet_id).map_err(|e| format!("Failed to load market: {}", e))?;
    
    // Check if the player has enough credits
    let cost = market.buy_resource(resource_type, quantity, system_id, planet_id)?;
    if player.credits < cost as f32 {
        return Err("Insufficient credits".to_string());
    }

    // Update player's inventory and credits
    player.credits -= cost as f32;
    player.add_resource(resource_type, quantity);

    // Save the market state
    market.save(system_id, planet_id).map_err(|e| format!("Failed to save market: {}", e))?;

    if PRINT_DEBUG {
        println!(
            "Successfully bought {} {} from {} for {} credits",
            quantity, resource_type, planet.name, cost
        );
    }
    Ok(())
}

pub fn sell_to_planet(
    planet: &mut Planet,
    player: &mut Player,
    resource_type: ResourceType,
    quantity: u32,
    system_id: usize,
    planet_id: usize
) -> Result<(), String> {
    let mut market = Market::load(system_id, planet_id).map_err(|e| format!("Failed to load market: {}", e))?;
    
    // Check if the player has enough of the resource
    if !player.has_resource(resource_type, quantity) {
        return Err(format!("Not enough {} in your inventory", resource_type));
    }

    // Update player's inventory and credits
    let earnings = market.sell_resource(resource_type, quantity, system_id, planet_id)?;
    player.credits += earnings as f32;
    player.remove_resource(resource_type, quantity);

    // Save the market state
    market.save(system_id, planet_id).map_err(|e| format!("Failed to save market: {}", e))?;

    if PRINT_DEBUG {
        println!(
            "Successfully sold {} {} to {} for {} credits",
            quantity, resource_type, planet.name, earnings
        );
    }
    Ok(())
}

pub fn trade_with_fleet(
    player_fleet: &mut Fleet,
    trader_fleet: &mut Fleet,
    resource_type: ResourceType,
    quantity: u32,
    trade_type: &str,
    player: &mut Player
) -> Result<(), String> {
    // Find the resource in trader's cargo
    let mut trader_resource = None;
    let mut trader_ship_index = 0;
    let mut cargo_index = 0;

    for (ship_idx, ship) in trader_fleet.ships.iter().enumerate() {
        for (cargo_idx, cargo) in ship.cargo.iter().enumerate() {
            if cargo.resource_type == resource_type {
                trader_resource = Some(cargo.clone());
                trader_ship_index = ship_idx;
                cargo_index = cargo_idx;
                break;
            }
        }
        if trader_resource.is_some() {
            break;
        }
    }

    match trader_resource {
        Some(resource) => {
            match trade_type {
                "buy" => {
                    // Calculate total cost
                    let total_cost = (resource.buy.unwrap_or(0.0) * quantity as f32) as f32;
                    
                    // Check if player has enough credits
                    if player.credits < total_cost {
                        return Err("Insufficient credits".to_string());
                    }

                    // Check if trader has enough quantity
                    if resource.quantity.unwrap_or(0) < quantity {
                        return Err("Trader doesn't have enough resources".to_string());
                    }

                    // Update player's credits and cargo
                    player.credits -= total_cost;
                    
                    // Add cargo to player's fleet
                    let mut found = false;
                    for ship in &mut player_fleet.ships {
                        for cargo in &mut ship.cargo {
                            if cargo.resource_type == resource_type {
                                cargo.quantity = Some(cargo.quantity.unwrap_or(0) + quantity);
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }

                    if !found {
                        // Add new cargo item if not found
                        if let Some(ship) = player_fleet.ships.first_mut() {
                            ship.cargo.push(Resource {
                                resource_type: resource_type.clone(),
                                quantity: Some(quantity),
                                buy: None,
                                sell: None
                            });
                        }
                    }

                    // Update trader's cargo
                    if let Some(ship) = trader_fleet.ships.get_mut(trader_ship_index) {
                        if let Some(cargo) = ship.cargo.get_mut(cargo_index) {
                            cargo.quantity = Some(cargo.quantity.unwrap_or(0) - quantity);
                        }
                    }
                },
                "sell" => {
                    // Find resource in player's fleet
                    let mut player_resource = None;
                    let mut player_ship_index = 0;
                    let mut player_cargo_index = 0;

                    for (ship_idx, ship) in player_fleet.ships.iter().enumerate() {
                        for (cargo_idx, cargo) in ship.cargo.iter().enumerate() {
                            if cargo.resource_type == resource_type {
                                player_resource = Some(cargo.clone());
                                player_ship_index = ship_idx;
                                player_cargo_index = cargo_idx;
                                break;
                            }
                        }
                        if player_resource.is_some() {
                            break;
                        }
                    }

                    match player_resource {
                        Some(resource) => {
                            // Check if player has enough quantity
                            if resource.quantity.unwrap_or(0) < quantity {
                                return Err("You don't have enough resources".to_string());
                            }

                            // Calculate total earnings
                            let total_earnings = (resource.sell.unwrap_or(0.0) * quantity as f32) as f32;
                            
                            // Update player's credits and cargo
                            player.credits += total_earnings;
                            
                            // Update player's cargo
                            if let Some(ship) = player_fleet.ships.get_mut(player_ship_index) {
                                if let Some(cargo) = ship.cargo.get_mut(player_cargo_index) {
                                    cargo.quantity = Some(cargo.quantity.unwrap_or(0) - quantity);
                                }
                            }

                            // Add cargo to trader's fleet
                            let mut found = false;
                            for ship in &mut trader_fleet.ships {
                                for cargo in &mut ship.cargo {
                                    if cargo.resource_type == resource_type {
                                        cargo.quantity = Some(cargo.quantity.unwrap_or(0) + quantity);
                                        found = true;
                                        break;
                                    }
                                }
                                if found {
                                    break;
                                }
                            }

                            if !found {
                                // Add new cargo item if not found
                                if let Some(ship) = trader_fleet.ships.first_mut() {
                                    ship.cargo.push(Resource {
                                        resource_type: resource_type.clone(),
                                        quantity: Some(quantity),
                                        buy: resource.buy,
                                        sell: resource.sell
                                    });
                                }
                            }
                        },
                        None => {
                            return Err("You don't have this resource".to_string());
                        }
                    }
                },
                _ => {
                    return Err("Invalid trade type".to_string());
                }
            }
            Ok(())
        },
        None => {
            Err("Resource not found in trader's cargo".to_string())
        }
    }
}