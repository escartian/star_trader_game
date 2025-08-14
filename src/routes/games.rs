use std::path::Path;
use std::fs;

use rocket::get;
use rocket::post;
use rocket::delete;
use rocket::serde::json::Json;

use chrono::Utc;

use crate::models::response::ApiResponse;
use crate::models::settings::{GameSettings, SavedGame, load_settings};
use crate::models::game_state::{save_json, game_path};
use crate::models::player::Player;
use crate::models::fleet::{Fleet, generate_and_save_fleet};
use crate::models::position::random_position;
use crate::models::faction::{Faction, save_faction};
use crate::models::market::regenerate_system_markets;
use crate::models::star_system::StarSystem;

#[get("/games")]
pub fn list_games() -> Json<ApiResponse<Vec<SavedGame>>> {
	match SavedGame::list_saved_games() {
		Ok(games) => ApiResponse::success(games, "Successfully retrieved saved games".to_string()),
		Err(e) => ApiResponse::error(format!("Failed to load saved games: {}", e))
	}
}

#[post("/games/new", data = "<settings>")]
pub fn create_new_game(settings: Json<GameSettings>) -> Json<ApiResponse<String>> {
	println!("Starting create_new_game with settings: {:?}", settings);
	let mut settings = settings.into_inner();
	let game_id = settings.game_id.clone();
	let display_name = settings.display_name.clone();
	let now = Utc::now().to_rfc3339();
	settings.created_at = now.clone();
	settings.last_played = now;

	let game_dir = Path::new("data").join("game").join(&game_id);
	let markets_dir = game_dir.join("markets");
	let fleets_dir = game_dir.join("fleets");
	let players_dir = game_dir.join("players");
	let factions_dir = game_dir.join("factions");
	let star_systems_dir = game_dir.join("star_systems");
	for dir in [&game_dir, &markets_dir, &fleets_dir, &players_dir, &factions_dir, &star_systems_dir] { if let Err(e) = fs::create_dir_all(dir) { println!("Error creating directory {}: {}", dir.display(), e); return ApiResponse::error(format!("Failed to create game directories: {}", e)); } }

	let saved_game = SavedGame { game_id: game_id.clone(), display_name: display_name.clone(), created_at: settings.created_at.clone(), last_played: settings.last_played.clone(), settings: settings.clone() };
	if let Err(e) = saved_game.save_game() { println!("Error saving game: {}", e); return ApiResponse::error("Failed to save game".to_string()); }

	let settings_path = game_dir.join("settings.json");
	if let Err(e) = save_json(&settings_path, &settings) { println!("Error saving settings: {}", e); return ApiResponse::error("Failed to save settings".to_string()); }

	let mut state = match crate::models::game_state::get_game_state() { Ok(state) => state, Err(e) => return ApiResponse::error(format!("Failed to get game state: {}", e)), };
	state.current_game_id = Some(game_id.clone());
	state.credits = settings.starting_credits as f64;
	if let Err(e) = crate::models::game_state::save_game_state(state) { println!("Error updating game state: {}", e); return ApiResponse::error("Failed to update game state".to_string()); }

	let game_world = match crate::models::game_world::create_game_world_file(&settings, true) { Ok(world) => world, Err(e) => return ApiResponse::error(format!("Failed to create game world: {}", e)), };
	if let Err(e) = crate::models::game_state::save_star_systems(&game_world) { println!("Error saving game world: {}", e); return ApiResponse::error("Failed to create game world".to_string()); }
	if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { *guard = game_world.clone(); } else { return ApiResponse::error("Failed to update game world".to_string()); }

	let game_world = if let Ok(guard) = crate::GLOBAL_GAME_WORLD.lock() { guard.clone() } else { return ApiResponse::error("Failed to access game world".to_string()); };
    for (system_id, _system) in game_world.iter().enumerate() {
		if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { *guard = game_world.clone(); }
		if let Err(e) = regenerate_system_markets(system_id) { println!("Error generating markets for system {}: {}", system_id, e); return ApiResponse::error(format!("Failed to generate markets for system {}: {}", system_id, e)); }
	}

    for faction_settings in &settings.factions {
		let faction = Faction::new(
			faction_settings.name.clone(),
			format!("The {} Empire", faction_settings.name)
		);
		if let Err(e) = save_faction(&faction) { println!("Error saving faction {}: {}", faction_settings.name, e); return ApiResponse::error(format!("Failed to save faction {}: {}", faction_settings.name, e)); }
		let fleet_count = 2 + (faction_settings.influence as usize / 30);
        for _ in 0..fleet_count {
			let ship_count = 1 + (faction_settings.influence as usize / 20);
			let _ = generate_and_save_fleet(
				faction_settings.name.clone(),
				random_position(settings.map_width as i32, settings.map_height as i32, settings.map_length as i32),
				ship_count
			);
		}
	}

	let player = Player::new(&settings.player_name, settings.starting_credits as f64);
	let player_path = game_dir.join("players").join(format!("{}.json", settings.player_name));
	if let Err(e) = save_json(&player_path, &player) { println!("Error saving player: {}", e); return ApiResponse::error(format!("Failed to save player: {}", e)); }

	let mut player_fleet = Fleet::new(settings.player_name.clone(), random_position(settings.map_width as i32, settings.map_height as i32, settings.map_length as i32), 1);
	let mut starting_ship = crate::models::ship::ship::Ship::new(crate::models::ship::ship::ShipType::Freighter, crate::models::ship::ship::ShipSize::Small, crate::models::ship::ship::ShipEngine::Basic);
	starting_ship.position = player_fleet.position.clone();
	starting_ship.hp = 100;
	starting_ship.name = format!("{}'s First Ship", settings.player_name);
	player_fleet.ships.push(starting_ship);
	let fleet_path = game_dir.join("fleets").join(format!("Fleet_{}_{}.json", settings.player_name, 1));
	if let Err(e) = save_json(&fleet_path, &player_fleet) { println!("Error saving player fleet: {}", e); return ApiResponse::error(format!("Failed to save player fleet: {}", e)); }

	ApiResponse::success("Game created successfully".to_string(), "Success".to_string())
}

#[get("/games/<game_id>/load")]
pub fn load_game(game_id: String) -> Json<ApiResponse<String>> {
	let result: Result<String, String> = (|| {
		let saved_game = SavedGame::load_game(&game_id).map_err(|e| e.to_string())?.ok_or_else(|| "Game not found".to_string())?;
		if saved_game.settings.game_id != "test_game" { if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { if guard.is_empty() { match crate::models::game_world::load_game_world(&saved_game.settings.game_id) { Ok(world) => { println!("Loaded {} systems into GLOBAL_GAME_WORLD at runtime", world.len()); *guard = world.clone(); }, Err(e) => println!("Failed to load GameWorld.json at startup: {}", e), } } } }
		let mut game_world = if saved_game.settings.game_id == "test_game" { let system_path = crate::models::game_state::game_data_path(&saved_game.settings.game_id, &["star_systems", "Star_System_0.json"]); match crate::models::game_state::load_json::<StarSystem>(&system_path) { Ok(system) => vec![system], Err(e) => { println!("  Error loading test system: {}. Using empty world.", e); Vec::new() } } } else { crate::models::game_world::get_global_game_world() };
		if saved_game.settings.game_id != "test_game" && game_world.is_empty() {
			let star_dir = game_path(&["star_systems"]);
			if star_dir.exists() {
				let mut systems = Vec::new();
				for entry in std::fs::read_dir(&star_dir).map_err(|e| e.to_string())? {
					let entry = entry.map_err(|e| e.to_string())?; let path = entry.path(); if path.extension().and_then(|s| s.to_str()) == Some("json") { if let Ok(system) = crate::models::game_state::load_json::<StarSystem>(&path) { systems.push(system); } }
				}
				if !systems.is_empty() { if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { *guard = systems.clone(); } game_world = systems; }
			}
		}
		if let Ok(mut guard) = crate::GLOBAL_GAME_WORLD.lock() { *guard = game_world; } else { return Err("Failed to update game world".to_string()); }
		let player_path = Path::new("data").join("game").join(&saved_game.settings.game_id).join("players").join(format!("{}.json", saved_game.settings.player_name));
		let player_credits = if player_path.exists() { let file = std::fs::File::open(&player_path).map_err(|e| format!("Failed to open player file: {}", e))?; let player: Player = serde_json::from_reader(file).map_err(|e| format!("Failed to parse player data: {}", e))?; player.credits } else { saved_game.settings.starting_credits as f64 };
		let mut state = crate::models::game_state::get_game_state().map_err(|e| format!("Failed to get game state: {}", e))?; state.current_game_id = Some(saved_game.settings.game_id.clone()); state.credits = player_credits; crate::models::game_state::save_game_state(state).map_err(|e| format!("Failed to save game state: {}", e))?;
		Ok("Game loaded successfully".to_string())
	})();

	match result { Ok(message) => ApiResponse::success(message, "Success".to_string()), Err(e) => ApiResponse::error(e) }
}

#[get("/settings")]
pub fn get_settings() -> Json<ApiResponse<GameSettings>> {
	match load_settings() { Ok(settings) => ApiResponse::success(settings, "Successfully retrieved settings".to_string()), Err(_) => ApiResponse::error("No active game found".to_string()) }
}

#[post("/settings", data = "<settings>")]
pub fn update_settings(settings: Json<GameSettings>) -> Json<ApiResponse<String>> {
	let settings = settings.into_inner();
	match SavedGame::load_game(&settings.game_id) { Ok(Some(mut saved_game)) => { saved_game.settings = settings; match saved_game.save_game() { Ok(_) => ApiResponse::success("Settings updated successfully".to_string(), "Success".to_string()), Err(e) => ApiResponse::error(format!("Failed to save settings: {}", e)) } }, Ok(None) => ApiResponse::error("Game not found".to_string()), Err(e) => ApiResponse::error(format!("Failed to load game: {}", e)) }
}

#[delete("/games/<game_id>")]
pub fn delete_game(game_id: String) -> Json<ApiResponse<String>> {
	let game_dir = Path::new("data").join("game").join(&game_id);
	let save_file = Path::new("data").join("saves").join(format!("{}.json", game_id));
	if save_file.exists() { if let Err(e) = fs::remove_file(&save_file) { return ApiResponse::error(format!("Failed to delete save file: {}", e)); } }
	if game_dir.exists() { if let Err(e) = fs::remove_dir_all(&game_dir) { return ApiResponse::error(format!("Failed to delete game directory: {}", e)); } }
	ApiResponse::success("Game deleted successfully".to_string(), "Success".to_string())
}


