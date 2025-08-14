use rocket::post;
use rocket::serde::json::Json;

use crate::models::response::ApiResponse;

#[post("/clear-caches")]
pub fn clear_caches() -> Json<ApiResponse<String>> {
	crate::models::game_state::SYSTEM_CACHE.remove_all();
	crate::models::game_state::FLEET_CACHE.remove_all();
	crate::models::game_state::MARKET_CACHE.remove_all();
	ApiResponse::success("Caches cleared successfully".to_string(), "Success".to_string())
}


