use std::fs;
use std::path::Path;

use rocket::get;
use rocket::serde::json::Json;

use crate::models::fleet::Fleet;
use crate::models::response::ApiResponse;
use crate::models::settings::load_settings;
use crate::models::game_state::{load_fleet};

#[get("/fleet/<owner_id>")]
pub fn get_owner_fleets(owner_id: String) -> Json<ApiResponse<Vec<Fleet>>> {
	println!("Getting fleets for owner: {}", owner_id);

	let result: Result<Vec<Fleet>, String> = (|| {
		let settings = load_settings().map_err(|e| e.to_string())?;
		let fleets_dir = Path::new("data")
			.join("game")
			.join(&settings.game_id)
			.join("fleets");

		println!("Looking for fleets in directory: {}", fleets_dir.display());
		let mut fleets = Vec::new();

		if fleets_dir.exists() {
			if let Ok(entries) = fs::read_dir(fleets_dir) {
				for entry in entries {
					if let Ok(entry) = entry {
						if let Some(file_name) = entry.file_name().to_str() {
							println!("Found fleet file: {}", file_name);
							// Handle fleet naming scheme
							if file_name.starts_with("Fleet_") {
								let parts: Vec<&str> = file_name.split('_').collect();
								if parts.len() >= 3 {
									let fleet_owner = parts[1];
									if fleet_owner == owner_id {
										let fleet_name = file_name.trim_end_matches(".json");
										println!("Loading fleet: {}", fleet_name);
										match crate::models::fleet::load_fleet(fleet_name) {
											Ok(Some(fleet)) => {
												println!("Successfully loaded fleet: {} with {} ships", fleet.name, fleet.ships.len());
												fleets.push(fleet);
											},
											Ok(None) => println!("Fleet not found: {}", fleet_name),
											Err(e) => println!("Error loading fleet {}: {}", fleet_name, e),
										}
									}
								}
							}
						}
					}
				}
			} else {
				println!("Error reading fleets directory");
				return Err("Failed to read fleets directory".to_string());
			}
		} else {
			println!("Fleets directory does not exist");
			return Err("Fleets directory not found".to_string());
		}

		println!("Found {} fleets for owner {}", fleets.len(), owner_id);
		Ok(fleets)
	})();

	match result {
		Ok(fleets) => ApiResponse::success(fleets, "Successfully retrieved fleets".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[get("/fleet/<owner_id>/<fleet_number>")]
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<ApiResponse<Fleet>> {
	println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

	let result: Result<Fleet, String> = (|| {
		// First try to load by owner_id (for regular fleets)
		let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
		
		println!("Looking for fleet with name: {}", fleet_name);
		
		match crate::models::fleet::load_fleet(&fleet_name) {
			Ok(Some(fleet)) => Ok(fleet),
			Ok(None) => {
				// If not found, try to load by fleet type (for special fleets)
				let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
				for fleet_type in special_types {
					let fleet_name = format!("Fleet_{}_{}", fleet_type, fleet_number);
					match crate::models::fleet::load_fleet(&fleet_name) {
						Ok(Some(fleet)) => return Ok(fleet),
						Ok(None) => continue,
						Err(e) => {
							println!("Error loading special fleet: {}", e);
							continue;
						}
					}
				}
				Err("Fleet not found".to_string())
			}
			Err(e) => {
				println!("Error loading fleet: {}", e);
				Err(e.to_string())
			}
		}
	})();

	match result {
		Ok(fleet) => ApiResponse::success(fleet, "Successfully retrieved fleet".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[get("/fleet/owners")]
pub fn get_fleet_owners() -> Json<ApiResponse<Vec<String>>> {
	let result: Result<Vec<String>, String> = (|| {
		let settings = load_settings().map_err(|e| e.to_string())?;
		let fleets_dir = Path::new("data")
			.join("game")
			.join(&settings.game_id)
			.join("fleets");
		let mut owners = std::collections::HashSet::new();

		if fleets_dir.exists() {
			if let Ok(entries) = fs::read_dir(fleets_dir) {
				for entry in entries {
					if let Ok(entry) = entry {
						if let Some(file_name) = entry.file_name().to_str() {
							// Handle fleet naming scheme
							if file_name.starts_with("Fleet_") {
								// Regular fleet (Fleet_owner_number)
								if let Some(owner) = file_name.split('_').nth(1) {
									println!("Found fleet owner: {}", owner);
									owners.insert(owner.to_string());
								}
							}
						}
					}
				}
			}
		}

		let owners_vec: Vec<String> = owners.into_iter().collect();
		println!("Found {} fleet owners: {:?}", owners_vec.len(), owners_vec);
		Ok(owners_vec)
	})();

	match result {
		Ok(owners) => ApiResponse::success(owners, "Successfully retrieved fleet owners".to_string()),
		Err(e) => ApiResponse::error(e)
	}
}

#[get("/fleets")]
pub fn get_player_fleets() -> Json<ApiResponse<Vec<Fleet>>> {
	let settings = match load_settings() {
		Ok(settings) => settings,
		Err(e) => {
			println!("Error loading settings: {}", e);
			return ApiResponse::error("Failed to load game settings".to_string());
		}
	};

	println!("Loading fleets for player: {}", settings.player_name);
	
	// First load the player to get their fleet list
	let player = match crate::models::game_state::load_player(&settings.player_name) {
		Ok(player) => player,
		Err(e) => {
			println!("Error loading player: {}", e);
			return ApiResponse::error(format!("Failed to load player: {}", e));
		}
	};

	// Load each fleet from the player's fleet list
	let mut fleets = Vec::new();
	for fleet_name in &player.fleets {
		match load_fleet(fleet_name) {
			Ok(fleet) => {
				println!("Loaded fleet: {} with {} ships", fleet.name, fleet.ships.len());
				fleets.push(fleet);
			},
			Err(e) => {
				println!("Error loading fleet {}: {}", fleet_name, e);
				// Continue loading other fleets even if one fails
				continue;
			}
		}
	}

	if fleets.is_empty() {
		println!("No fleets found for player");
		return ApiResponse::error("No fleets found".to_string());
	}

	ApiResponse::success(fleets, "Successfully loaded fleets".to_string())
}


