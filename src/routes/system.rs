use std::path::Path;

use rocket::get;
use rocket::serde::json::Json;

use crate::models::response::ApiResponse;
use crate::models::settings::load_settings;
use crate::models::star_system::StarSystem;
use crate::models::game_state::{game_path, load_json};

#[get("/galaxy_map")]
pub fn get_galaxy_map() -> Json<ApiResponse<Vec<StarSystem>>> {
	let _settings = load_settings().expect("Failed to load settings");
	let game_world_path = game_path(&["GameWorld.json"]);

	// Try to load from GameWorld.json
	if let Ok(world) = load_json::<Vec<StarSystem>>(&game_world_path) {
		return ApiResponse::success(world, "Successfully retrieved galaxy map".to_string());
	}

	// If GameWorld.json doesn't exist or can't be loaded, return empty vector
	ApiResponse::success(Vec::new(), "No galaxy map found".to_string())
}

#[get("/star_system/<system_id>")]
pub fn get_star_system(system_id: usize) -> Json<ApiResponse<StarSystem>> {
	let settings = load_settings().expect("Failed to load settings");
	let system_path = Path::new("data")
		.join("game")
		.join(&settings.game_id)
		.join("star_systems")
		.join(format!("Star_System_{}.json", system_id));

	match load_json::<StarSystem>(&system_path) {
		Ok(system) => ApiResponse::success(system, "Successfully retrieved star system".to_string()),
		Err(e) => {
			println!("Error loading star system: {}", e);
			// Try loading from the game world file
			let game_world_path = game_path(&["GameWorld.json"]);
			match load_json::<Vec<StarSystem>>(&game_world_path) {
				Ok(systems) => {
					if let Some(sys) = systems.iter().find(|s| s.id == system_id) {
						ApiResponse::success(sys.clone(), "Successfully retrieved star system from game world".to_string())
					} else {
						ApiResponse::error(format!("System ID {} not found in game world", system_id))
					}
				},
				Err(e) => ApiResponse::error(format!("Failed to load star system: {}", e))
			}
		}
	}
}


