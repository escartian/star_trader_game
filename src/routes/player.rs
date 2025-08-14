use rocket::get;
use rocket::post;
use rocket::serde::json::Json;

use crate::models::player::Player;
use crate::models::response::ApiResponse;
// no settings API needed here
use strum::IntoEnumIterator;
use crate::models::game_state::load_player;

#[get("/player/<name>")]
pub fn get_player(name: &str) -> Json<ApiResponse<Player>> {
	let result: Result<Player, String> = (|| {
		let mut player = load_player(name)?;
		// Mirror summed cargo across all owned fleets into player.resources
		let fleets = crate::models::fleet::list_owner_fleets(name).map_err(|e| e.to_string())?;
		use std::collections::HashMap;
		let mut totals: std::collections::HashMap<crate::models::resource::ResourceType, u32> = HashMap::new();
		for fleet in &fleets {
			for ship in &fleet.ships {
				for cargo in &ship.cargo {
					let qty = cargo.quantity.unwrap_or(0);
					*totals.entry(cargo.resource_type).or_insert(0) += qty;
				}
			}
		}
		// Build a complete resource list, ensuring all types exist with a quantity
		let updated: Vec<crate::models::resource::Resource> = crate::models::resource::ResourceType::iter()
			.map(|rt| crate::models::resource::Resource { resource_type: rt, buy: None, sell: None, quantity: Some(*totals.get(&rt).unwrap_or(&0)) })
			.collect();
		player.resources = updated;
		// Best effort save so totals persist; ignore save error to not fail GET
		let _ = player.save();
		Ok(player)
	})();

	match result {
		Ok(player) => ApiResponse::success(player, "Successfully retrieved player".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[post("/player/<name>/add_credits", format = "json", data = "<amount>")]
pub fn add_credits(name: &str, amount: Json<f64>) -> Json<ApiResponse<String>> {
	let result: Result<String, String> = (|| {
		let mut player = load_player(name).map_err(|e| e.to_string())?;
		player.credits += *amount;
		if let Err(e) = player.save() {
			println!("Error saving player data: {}", e);
			return Err("Failed to save player data".to_string());
		}
		Ok(format!("Successfully added {} credits", *amount))
	})();

	match result {
		Ok(message) => ApiResponse::success(message, "Credits added successfully".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[post("/player/<name>/remove_credits", format = "json", data = "<amount>")]
pub fn remove_credits(name: &str, amount: Json<f64>) -> Json<ApiResponse<String>> {
	let result: Result<String, String> = (|| {
		let mut player = load_player(name).map_err(|e| e.to_string())?;
		if player.credits < *amount {
			return Err(format!("Not enough credits: need {} but have {}", *amount, player.credits));
		}
		player.credits -= *amount;
		if let Err(e) = player.save() {
			println!("Error saving player data: {}", e);
			return Err("Failed to save player data".to_string());
		}
		Ok(format!("Successfully removed {} credits", *amount))
	})();

	match result {
		Ok(message) => ApiResponse::success(message, "Credits removed successfully".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}


