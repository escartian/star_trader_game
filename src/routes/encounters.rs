use rocket::get;
use rocket::serde::json::Json;

use crate::encounters::generate_encounter_fleet;
use crate::models::fleet::Fleet;
use crate::models::settings::load_settings;

#[get("/fleet/<attacker_id>/<attacker_number>/attack/<defender_id>/<defender_number>")]
pub fn initiate_combat(attacker_id: String, attacker_number: usize, defender_id: String, defender_number: usize) -> Json<String> {
	println!("Starting combat initiation:");
	println!("  Attacker: Fleet_{}_{}", attacker_id, attacker_number);
	println!("  Defender: Fleet_{}_{}", defender_id, defender_number);

	// First try to load attacker fleet
	println!("Loading attacker fleet...");
	let attacker_name = format!("Fleet_{}_{}", attacker_id, attacker_number);
	let mut attacker_result = Err("Attacker fleet not found".to_string());
	
	// Try loading regular fleet first
	match crate::models::fleet::load_fleet(&attacker_name) {
		Ok(Some(fleet)) => {
			println!("Found attacker fleet directly: {}", fleet.name);
			attacker_result = Ok(fleet);
		}
		Ok(None) => {
			println!("Regular fleet not found, trying special types...");
			// Try special fleet types
			let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
			for fleet_type in special_types {
				let fleet_name = format!("Fleet_{}_{}", fleet_type, attacker_number);
				println!("Trying special fleet: {}", fleet_name);
				match crate::models::fleet::load_fleet(&fleet_name) {
					Ok(Some(fleet)) => {
						println!("Found attacker fleet: {}", fleet.name);
						attacker_result = Ok(fleet);
					break; 
					}
					Ok(None) => continue,
					Err(e) => {
						println!("Error loading special fleet: {}", e);
						continue;
					}
				}
			}
		}
		Err(e) => {
			println!("Error loading attacker fleet: {}", e);
		}
	}

	// Now try to load defender fleet
	println!("Loading defender fleet...");
	let defender_name = format!("Fleet_{}_{}", defender_id, defender_number);
	let mut defender_result = Err("Defender fleet not found".to_string());
	
	// Try loading regular fleet first
	match crate::models::fleet::load_fleet(&defender_name) {
		Ok(Some(fleet)) => {
			println!("Found defender fleet directly: {}", fleet.name);
			defender_result = Ok(fleet);
		}
		Ok(None) => {
			println!("Regular fleet not found, trying special types...");
			// Try special fleet types
			let special_types = vec!["Pirate", "Trader", "Military", "Mercenary"];
			for fleet_type in special_types {
				// Try both naming formats
				let fleet_names = vec![
					format!("Fleet_{}_{}", fleet_type, defender_number),
					format!("{}_Fleet_{}", fleet_type, defender_number)
				];
				
				for fleet_name in fleet_names {
					println!("Trying special fleet: {}", fleet_name);
					match crate::models::fleet::load_fleet(&fleet_name) {
						Ok(Some(fleet)) => {
							println!("Found defender fleet: {}", fleet.name);
							defender_result = Ok(fleet);
							break;
						}
						Ok(None) => continue,
						Err(e) => {
							println!("Error loading special fleet: {}", e);
							continue;
						}
					}
				}
				
				if defender_result.is_ok() {
					break;
				}
			}
		}
		Err(e) => {
			println!("Error loading defender fleet: {}", e);
		}
	}

	match (attacker_result, defender_result) {
		(Ok(mut attacker), Ok(mut defender)) => {
			println!("Both fleets loaded successfully:");
			println!("  Attacker: {} ({} ships)", attacker.name, attacker.ships.len());
			println!("  Defender: {} ({} ships)", defender.name, defender.ships.len());

			if !crate::combat::combat::can_engage_combat(&attacker, &defender) {
				println!("Fleets cannot engage in combat - not at same position");
				return Json("Fleets must be at the same position to engage in combat".to_string());
			}

			println!("Starting combat resolution...");
			let combat_result = crate::combat::combat::auto_resolve_ship_combat(&mut attacker, &mut defender);

			// Save the updated fleets
			println!("Saving updated fleets...");
			if let Err(e) = crate::models::fleet::save_fleet(&attacker) {
				println!("Error saving attacker fleet: {}", e);
				return Json("Error saving attacker fleet".to_string());
			}

			if let Err(e) = crate::models::fleet::save_fleet(&defender) {
				println!("Error saving defender fleet: {}", e);
				return Json("Error saving defender fleet".to_string());
			}

			// Format combat result
			let mut result = String::new();
			for log in combat_result.combat_log {
				result.push_str(&format!("{}\n", log));
			}
			result.push_str(&format!("\nFinal fleet sizes:\nAttacker: {} ships\nDefender: {} ships", 
				attacker.ships.len(), defender.ships.len()));

			println!("Combat completed successfully");
			Json(result)
		}
		(Err(e), _) => {
			println!("Error loading attacker fleet: {}", e);
			Json(e)
		}
		(_, Err(e)) => {
			println!("Error loading defender fleet: {}", e);
			Json(e)
		}
	}
}

#[get("/fleet/<owner_id>/<fleet_number>/encounter")]
pub fn check_for_encounter(owner_id: String, fleet_number: usize) -> Json<Vec<Fleet>> {
	println!("Checking for encounters for Fleet_{}_{}", owner_id, fleet_number);
	
	let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
	match crate::models::fleet::load_fleet(&fleet_name) {
		Ok(Some(fleet)) => {
			let mut encounters = Vec::new();
			
			// Check for star system encounters first
			if let Some(system_id) = fleet.current_system_id {
				let settings = load_settings().expect("Failed to load settings");
				if let Ok(Some(system)) = crate::models::game_world::load_star_system(&settings.game_id, system_id) {
					// Check if fleet is near any planets
					for planet in &system.planets {
						let planet_pos = planet.position;
						let distance = ((fleet.position.x - planet_pos.x).pow(2) + 
										(fleet.position.y - planet_pos.y).pow(2) + 
										(fleet.position.z - planet_pos.z).pow(2)) as f64;
						if distance <= 10.0 {
							// Create a fleet representing the planet's defenses
							let planet_fleet = Fleet {
								name: format!("Planet_{}", planet.name),
								owner_id: "Planet".to_string(),
								ships: Vec::new(),
								position: planet_pos,
								local_position: None,
								current_system_id: Some(system_id),
								last_move_distance: None,
							};
							encounters.push(planet_fleet);
						}
					}
				}
			}
			
			// Generate random encounters based on distance
			let distance = fleet.last_move_distance.unwrap_or(0.0);
			let max_encounters = (distance / 10.0).min(3.0) as i32;
			
			for _ in 0..max_encounters {
				if rand::random::<f64>() < 0.3 {
					let encounter_fleet = generate_encounter_fleet(fleet.position.clone());
					
					// Only add the encounter if it's not the same owner as the player's fleet
					if encounter_fleet.owner_id != owner_id {
						let fleet = Fleet {
							name: encounter_fleet.name,
							owner_id: encounter_fleet.owner_id,
							ships: encounter_fleet.ships,
							position: encounter_fleet.position,
							local_position: None,
							current_system_id: fleet.current_system_id,
							last_move_distance: None,
						};
						encounters.push(fleet);
					}
				}
			}
			
			// If we have encounters, save the current fleet's last move distance
			if !encounters.is_empty() {
				if let Ok(Some(mut current_fleet)) = crate::models::fleet::load_fleet(&fleet_name) {
					current_fleet.last_move_distance = Some(distance);
					if let Err(e) = crate::models::fleet::save_fleet(&current_fleet) {
						println!("Error saving fleet's last move distance: {}", e);
					}
				}
			}
			
			Json(encounters)
		}
		Ok(None) => Json(Vec::new()),
		Err(e) => {
			println!("Error loading fleet: {}", e);
			Json(Vec::new())
		}
	}
}

use crate::models::resource::ResourceType;

#[get("/fleet/<owner_id>/<fleet_number>/trade/<resource_type>/<quantity>/<trade_type>")]
pub fn trade_with_trader(owner_id: String, fleet_number: usize, resource_type: ResourceType, quantity: u32, trade_type: String) -> Json<String> {
	println!("Starting trade operation:");
	println!("  Fleet: Fleet_{}_{}", owner_id, fleet_number);
	println!("  Resource: {}", resource_type);
	println!("  Quantity: {}", quantity);
	println!("  Trade Type: {}", trade_type);

	let result: Result<String, String> = (|| {
		let settings = load_settings().map_err(|e| e.to_string())?;
		let mut player = crate::models::game_state::load_player(&settings.player_name)?;
		let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
		let trader_fleet_name = format!("Fleet_Trader_{}", fleet_number);

		// Load both fleets
		match (crate::models::fleet::load_fleet(&fleet_name), crate::models::fleet::load_fleet(&trader_fleet_name)) {
			(Ok(Some(mut player_fleet)), Ok(Some(mut trader_fleet))) => {
				match crate::models::trade::trade_with_fleet(&mut player_fleet, &mut trader_fleet, resource_type, quantity, &trade_type, &mut player) {
					Ok(_) => {
						// Save all changes
						if let Err(e) = crate::models::fleet::save_fleet(&player_fleet) {
							println!("Error saving player fleet: {}", e);
							return Err("Error saving player fleet".to_string());
						}

						if let Err(e) = crate::models::fleet::save_fleet(&trader_fleet) {
							println!("Error saving trader fleet: {}", e);
							return Err("Error saving trader fleet".to_string());
						}

						let player_path = std::path::Path::new("data")
							.join("game")
							.join(&settings.game_id)
							.join("players")
							.join(format!("{}.json", settings.player_name));
						if let Err(e) = crate::models::game_state::save_json(&player_path, &player) {
							println!("Error saving player: {}", e);
							return Err("Error saving player".to_string());
						}

						Ok("Success".to_string())
					},
					Err(e) => Err(e)
				}
			},
			(Ok(Some(_)), Ok(None)) => Err("Trader fleet not found".to_string()),
			(Ok(None), _) => Err("Player fleet not found".to_string()),
			(Err(e), _) => {
				println!("Error loading player fleet: {}", e);
				Err("Error loading player fleet".to_string())
			},
			(_, Err(e)) => {
				println!("Error loading trader fleet: {}", e);
				Err("Error loading trader fleet".to_string())
			}
		}
	})();

	match result {
		Ok(message) => Json(message),
		Err(e) => Json(e)
	}
}


