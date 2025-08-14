use std::path::Path;

use rocket::get;
use rocket::post;
use rocket::serde::json::Json;

use crate::models::market::{Market, ShipMarket, calculate_ship_price};
use crate::models::response::ApiResponse;
use crate::models::settings::load_settings;
use crate::models::game_state::{load_player, load_star_system, load_json};
use crate::models::trade::{ResourceTradeData, ShipTradeData, ShipTradeInData};
use crate::models::resource::Resource;
// no Fleet imports required in this module

use crate::models::planet::{load_planet_market, load_planet_ship_market};

#[get("/planet/<system_id>/<planet_id>/market")]
pub fn get_planet_market(system_id: usize, planet_id: usize) -> Json<ApiResponse<Market>> {
	match load_planet_market(system_id, planet_id) {
		Ok(market) => ApiResponse::success(market, "Successfully retrieved market".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[get("/planet/<system_id>/<planet_id>/ships")]
pub fn get_planet_ship_market(system_id: usize, planet_id: usize) -> Json<ApiResponse<ShipMarket>> {
	match load_planet_ship_market(system_id, planet_id) {
		Ok(market) => ApiResponse::success(market, "Successfully retrieved ship market".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[post("/planet/<system_id>/<planet_id>/buy", format = "json", data = "<data>")]
pub fn buy_from_planet(system_id: usize, planet_id: usize, data: Json<ResourceTradeData>) -> Json<ApiResponse<String>> {
	let result: Result<String, String> = (|| {
		let settings = load_settings().map_err(|e| e.to_string())?;
		let mut player = load_player(&settings.player_name)?;
		let mut system = load_star_system(system_id)?;
		let mut market = Market::load(system_id, planet_id).map_err(|e| e.to_string())?;
		
		let planet = system.planets.get_mut(planet_id)
			.ok_or_else(|| "Planet not found".to_string())?;

		// Enforce fleet selection and co-location for trading
		let fleet_name = data.fleet_name.clone().ok_or_else(|| "Select a fleet to trade at this planet".to_string())?;
		if let Ok(Some(fleet_for_check)) = crate::models::fleet::load_fleet(&fleet_name) {
			// Validate system id
			if fleet_for_check.current_system_id != Some(system_id) {
				return Err("Fleet must be in this system to trade".to_string());
			}
			// Validate strict local position
			let lp = fleet_for_check.local_position.as_ref().ok_or_else(|| "Fleet must be at this planet to trade".to_string())?;
			if lp.x != planet.position.x || lp.y != planet.position.y || lp.z != planet.position.z {
				return Err("Fleet must be at this planet to trade".to_string());
			}
		} else {
			return Err("Fleet not found for trading".to_string());
		}
		
		// 1) Capacity pre-check: if buying into a fleet, ensure enough cargo space
		if let Ok(Some(fleet)) = crate::models::fleet::load_fleet(&fleet_name) {
				let mode = data.distribution_mode.clone().unwrap_or_else(|| "first".to_string());
				let mut capacity_available: u32 = 0;
				match mode.as_str() {
					"specific" => {
						if let Some(allocs) = &data.allocations {
							for alloc in allocs {
								if let Some(ship) = fleet.ships.get(alloc.ship_index) {
									let space = ship.get_cargo_capacity().saturating_sub(ship.get_current_cargo());
									capacity_available = capacity_available.saturating_add(space);
								}
							}
						}
					}
					"even" => {
						for ship in &fleet.ships {
							let space = ship.get_cargo_capacity().saturating_sub(ship.get_current_cargo());
							capacity_available = capacity_available.saturating_add(space);
						}
					}
					_ => {
						if let Some(ship) = fleet.ships.first() {
							capacity_available = ship.get_cargo_capacity().saturating_sub(ship.get_current_cargo());
						}
					}
				}
				if capacity_available < data.quantity {
					return Err(format!(
						"Not enough cargo capacity in selected ship(s). Required {}, available {}",
						data.quantity, capacity_available
					));
				}
		}

        // 2) Calculate total cost and update market quantities
        let total_cost = market.buy_resource(data.resource_type, data.quantity, system_id, planet_id)
            .map_err(|e| e.to_string())?;
        if player.credits < total_cost { return Err("Insufficient credits".to_string()); }
        player.credits -= total_cost;
        if let Ok(Some(mut fleet)) = crate::models::fleet::load_fleet(&fleet_name) {
            let mode = data.distribution_mode.clone().unwrap_or_else(|| "first".to_string());
            let mut remaining = data.quantity;
            match mode.as_str() {
                "specific" => {
                    if let Some(allocs) = &data.allocations {
                        for alloc in allocs { if remaining==0 { break; } if let Some(ship) = fleet.ships.get_mut(alloc.ship_index) { let cap=ship.get_cargo_capacity(); let used=ship.get_current_cargo(); let space=cap.saturating_sub(used); let desired=alloc.quantity.min(remaining); let add_q=desired.min(space); let mut found=false; for c in &mut ship.cargo { if c.resource_type==data.resource_type { c.quantity=Some(c.quantity.unwrap_or(0)+add_q); found=true; break; } } if !found { ship.cargo.push(Resource{resource_type:data.resource_type,buy:None,sell:None,quantity:Some(add_q)}); } remaining-=add_q; } }
                    }
                }
                "even" => {
                    while remaining>0 { let mut progressed=false; for ship in &mut fleet.ships { if remaining==0 { break; } let cap=ship.get_cargo_capacity(); let used=ship.get_current_cargo(); let space=cap.saturating_sub(used); if space==0 { continue; } let add_q=1u32.min(remaining).min(space); let mut found=false; for c in &mut ship.cargo { if c.resource_type==data.resource_type { c.quantity=Some(c.quantity.unwrap_or(0)+add_q); found=true; break; } } if !found { ship.cargo.push(Resource{resource_type:data.resource_type,buy:None,sell:None,quantity:Some(add_q)}); } remaining-=add_q; progressed=true; } if !progressed { break; } }
                }
                _ => {
                    if let Some(ship) = fleet.ships.first_mut() { let cap=ship.get_cargo_capacity(); let used=ship.get_current_cargo(); let space=cap.saturating_sub(used); let add_q=remaining.min(space); if add_q>0 { let mut found=false; for c in &mut ship.cargo { if c.resource_type==data.resource_type { c.quantity=Some(c.quantity.unwrap_or(0)+add_q); found=true; break; } } if !found { ship.cargo.push(Resource{resource_type:data.resource_type,buy:None,sell:None,quantity:Some(add_q)}); } remaining-=add_q; } }
                }
            }
            if remaining>0 { return Err("Not enough cargo capacity to store purchased goods".to_string()); }
            crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
        }
        player.save().map_err(|e| e.to_string())?;
        market.save(system_id, planet_id).map_err(|e| e.to_string())?;
        Ok("Successfully bought resource".to_string())
    })();

    match result { Ok(message) => ApiResponse::success(message, "Trade completed successfully".to_string()), Err(e) => ApiResponse::error(e) }
}

#[post("/planet/<system_id>/<planet_id>/sell", format = "json", data = "<data>")]
pub fn sell_to_planet(system_id: usize, planet_id: usize, data: Json<ResourceTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let mut system = load_star_system(system_id).map_err(|e| e.to_string())?;
        let mut market = Market::load(system_id, planet_id).map_err(|e| e.to_string())?;
        let planet = system.planets.get_mut(planet_id).ok_or_else(|| "Planet not found".to_string())?;
        let fleet_name = data.fleet_name.clone().ok_or_else(|| "Select a fleet to trade at this planet".to_string())?;
        if let Ok(Some(fleet_for_check)) = crate::models::fleet::load_fleet(&fleet_name) {
            if fleet_for_check.current_system_id != Some(system_id) { return Err("Fleet must be in this system to trade".to_string()); }
            let lp = fleet_for_check.local_position.as_ref().ok_or_else(|| "Fleet must be at this planet to trade".to_string())?;
            if lp.x != planet.position.x || lp.y != planet.position.y || lp.z != planet.position.z { return Err("Fleet must be at this planet to trade".to_string()); }
        } else { return Err("Fleet not found for trading".to_string()); }

        if let Ok(Some(mut fleet)) = crate::models::fleet::load_fleet(&fleet_name) {
            let mode = data.distribution_mode.clone().unwrap_or_else(|| "first".to_string());
            let mut remaining = data.quantity;
            match mode.as_str() {
                "specific" => {
                    if let Some(allocs) = &data.allocations {
                        for alloc in allocs { if remaining==0 { break; } if let Some(ship) = fleet.ships.get_mut(alloc.ship_index) { let mut need = alloc.quantity.min(remaining); for cargo in &mut ship.cargo { if cargo.resource_type==data.resource_type { let have=cargo.quantity.unwrap_or(0); let take=have.min(need); cargo.quantity=Some(have-take); need-=take; break; } } remaining -= alloc.quantity.min(alloc.quantity - need); } }
                    }
                }
                "even" => {
                    let mut idx = 0usize; while remaining>0 && !fleet.ships.is_empty() { if idx>=fleet.ships.len() { idx=0; } let ship=&mut fleet.ships[idx]; let mut took=false; for cargo in &mut ship.cargo { if cargo.resource_type==data.resource_type { let have=cargo.quantity.unwrap_or(0); if have>0 { cargo.quantity=Some(have-1); remaining-=1; took=true; } break; } } if !took { idx+=1; } else { idx+=1; } }
                }
                _ => {
                    for ship in &mut fleet.ships { for cargo in &mut ship.cargo { if cargo.resource_type==data.resource_type { let have=cargo.quantity.unwrap_or(0); if have>=remaining { cargo.quantity=Some(have-remaining); remaining=0; break; } else { cargo.quantity=Some(0); remaining-=have; } } } if remaining==0 { break; } }
                }
            }
            if remaining>0 { return Err("Not enough resources in fleet cargo".to_string()); }
            crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
        } else { return Err("Fleet not found for selling".to_string()); }

        let total_value = market.sell_resource(data.resource_type, data.quantity, system_id, planet_id).map_err(|e| e.to_string())?;
        player.credits += total_value;
        player.save().map_err(|e| e.to_string())?; market.save(system_id, planet_id).map_err(|e| e.to_string())?;
        Ok("Successfully sold resource".to_string())
    })();

    match result { Ok(message) => ApiResponse::success(message, "Trade completed successfully".to_string()), Err(e) => ApiResponse::error(e) }
}

#[post("/planet/<system_id>/<planet_id>/buy_ship", format = "json", data = "<data>")]
pub fn buy_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data").join("game").join(&settings.game_id).join("markets").join(format!("ships_{}_{}.json", system_id, planet_id));
        println!("Buy ship request: system={}, planet={}, ship_index={}, fleet_name={:?}", system_id, planet_id, data.ship_index, data.fleet_name);
        println!("Player credits before purchase: {}", player.credits);
        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        if data.ship_index >= ship_market.ships.len() { return Err(format!("Invalid ship index: {} (market has {} ships)", data.ship_index, ship_market.ships.len())); }
        let ship = &ship_market.ships[data.ship_index];
        if let Some(price) = ship.price {
            if player.credits >= price {
                player.credits -= price;
                println!("Player credits after purchase: {}", player.credits);
                let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
                let mut fleet = crate::models::fleet::load_fleet(&fleet_name).map_err(|e| e.to_string())?.ok_or_else(|| "Fleet not found".to_string())?;
                println!("Adding ship {} to fleet {}", ship.name, fleet.name);
                fleet.ships.push(ship.clone());
                ship_market.ships.remove(data.ship_index);
                load_json::<ShipMarket>(&market_path).ok();
                crate::models::game_state::save_json(&market_path, &ship_market).map_err(|e| e.to_string())?;
                if let Err(e) = player.save() { println!("Error saving player data: {}", e); return Err("Failed to save player data".to_string()); }
                crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
                Ok(format!("Successfully bought ship for {} credits", price))
            } else { Err(format!("Not enough credits: need {} but have {}", price, player.credits)) }
        } else { Err("Ship is not for sale".to_string()) }
    })();

    match result { Ok(message) => ApiResponse::success(message, "Ship purchase completed successfully".to_string()), Err(e) => ApiResponse::error(e) }
}

#[post("/planet/<system_id>/<planet_id>/sell_ship", format = "json", data = "<data>")]
pub fn sell_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data").join("game").join(&settings.game_id).join("markets").join(format!("ships_{}_{}.json", system_id, planet_id));
        println!("Sell ship request: system={}, planet={}, ship_index={}, fleet_name={:?}", system_id, planet_id, data.ship_index, data.fleet_name);
        println!("Player credits before sale: {}", player.credits);
        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
        let mut fleet = crate::models::fleet::load_fleet(&fleet_name).map_err(|e| e.to_string())?.ok_or_else(|| format!("Fleet not found: {}", fleet_name))?;
        if data.ship_index >= fleet.ships.len() { return Err(format!("Invalid ship index: {} (fleet has {} ships)", data.ship_index, fleet.ships.len())); }
        let ship = fleet.ships[data.ship_index].clone();
        let price = calculate_ship_price(&ship) * 0.7;
        println!("Selling ship: {} for {} credits", ship.name, price);
        player.credits += price;
        println!("Player credits after sale: {}", player.credits);
        let mut market_ship = ship.clone(); market_ship.price = Some(price); ship_market.ships.push(market_ship);
        fleet.ships.remove(data.ship_index);
        crate::models::game_state::save_json(&market_path, &ship_market).map_err(|e| e.to_string())?;
        if let Err(e) = player.save() { println!("Error saving player data: {}", e); return Err("Failed to save player data".to_string()); }
        crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?;
        Ok(format!("Successfully sold ship for {} credits", price))
    })();

    match result { Ok(message) => ApiResponse::success(message, "Ship sale completed successfully".to_string()), Err(e) => ApiResponse::error(e) }
}

#[post("/planet/<system_id>/<planet_id>/trade_in_ship", format = "json", data = "<data>")]
pub fn trade_in_ship(system_id: usize, planet_id: usize, data: Json<ShipTradeInData>) -> Json<ApiResponse<String>> {
    let result: Result<String, String> = (|| {
        let settings = load_settings().map_err(|e| e.to_string())?;
        let mut player = load_player(&settings.player_name).map_err(|e| e.to_string())?;
        let market_path = Path::new("data").join("game").join(&settings.game_id).join("markets").join(format!("ships_{}_{}.json", system_id, planet_id));
        let mut ship_market: ShipMarket = load_json(&market_path).map_err(|e| e.to_string())?;
        if data.ship_index >= ship_market.ships.len() { return Err(format!("Invalid ship index: {} (market has {} ships)", data.ship_index, ship_market.ships.len())); }
        let new_ship = ship_market.ships[data.ship_index].clone();
        if let Some(price) = new_ship.price {
            let fleet_name = data.fleet_name.clone().unwrap_or_else(|| format!("Fleet_{}_1", settings.player_name));
            let mut fleet = crate::models::fleet::load_fleet(&fleet_name).map_err(|e| e.to_string())?.ok_or_else(|| "Fleet not found".to_string())?;
            let mut trade_in_value = 0.0;
            if let Some(trade_in_index) = data.trade_in_ship_index { if trade_in_index >= fleet.ships.len() { return Err(format!("Invalid trade-in ship index: {} (fleet has {} ships)", trade_in_index, fleet.ships.len())); } let trade_in_ship = fleet.ships[trade_in_index].clone(); trade_in_value = calculate_ship_price(&trade_in_ship) * 0.7; let mut market_ship = trade_in_ship.clone(); market_ship.price = Some(calculate_ship_price(&trade_in_ship) * 0.8); ship_market.ships.push(market_ship); fleet.ships.remove(trade_in_index); }
            let final_price = price - trade_in_value; println!("Trade calculation: Market price {} - Trade-in value {} = Final price {}", price, trade_in_value, final_price);
            if player.credits >= final_price { player.credits -= final_price; println!("Player credits after trade: {}", player.credits); fleet.ships.push(new_ship); ship_market.ships.remove(data.ship_index); crate::models::game_state::save_json(&market_path, &ship_market).map_err(|e| e.to_string())?; if let Err(e) = player.save() { println!("Error saving player data: {}", e); return Err("Failed to save player data".to_string()); } crate::models::fleet::save_fleet(&fleet).map_err(|e| e.to_string())?; Ok(format!("Successfully traded in ship for {} credits", final_price)) } else { Err(format!("Not enough credits: need {} but have {}", final_price, player.credits)) }
        } else { Err("Ship is not for sale".to_string()) }
    })();

    match result { Ok(message) => ApiResponse::success(message, "Ship trade-in completed successfully".to_string()), Err(e) => ApiResponse::error(e) }
}


