use rocket::catch;
use rocket::Request;
use rocket::serde::json::Json;

use crate::models::response::ApiResponse;

mod movement;
mod markets;
mod encounters;
mod games;
mod fleets;
mod player;
mod system;
mod maintenance;

pub use movement::move_fleet;
pub use markets::{
	get_planet_market,
	get_planet_ship_market,
	buy_from_planet,
	sell_to_planet,
	buy_ship,
	sell_ship,
	trade_in_ship,
};
pub use encounters::{
	initiate_combat,
	check_for_encounter,
	trade_with_trader,
};
pub use games::{
	list_games,
	create_new_game,
	load_game,
	get_settings,
	update_settings,
	delete_game,
};
pub use fleets::{
	get_owner_fleets,
	get_fleet,
	get_fleet_owners,
	get_player_fleets,
};
pub use player::{
	get_player,
	add_credits,
	remove_credits,
};
pub use system::{
	get_galaxy_map,
	get_star_system,
};
pub use maintenance::clear_caches;

#[catch(500)]
pub fn internal_error(_req: &Request) -> Json<ApiResponse<String>> {
	ApiResponse::error("An internal server error occurred. Please try again later.".to_string())
}

#[rocket::get("/")]
pub fn index() -> Json<ApiResponse<String>> {
	ApiResponse::success("Welcome to Star Trader Game API".to_string(), "Success".to_string())
}


