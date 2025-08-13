pub mod constants;
pub mod models;
pub mod routes;
pub mod combat;
pub mod encounters;

use rocket::routes;
use rocket::catchers;

use std::env;

use crate::routes::*;
use crate::models::star_system::StarSystem;

use rocket_cors::{AllowedOrigins, CorsOptions, AllowedHeaders};
use rocket::fs::FileServer;

use crate::constants::GLOBAL_GAME_WORLD;

pub(crate) fn get_global_game_world() -> Vec<StarSystem> {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
}

/// The main entry point for the Rocket application.
///
/// This function is responsible for launching the Rocket server and mounting
/// the routes.
#[rocket::main]
async fn main() {
    // Configure CORS
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec!["Get", "Post", "Put", "Delete", "Options", "Patch"]
                .into_iter()
                .map(|s| s.parse().unwrap())
                .collect(),
        )
        .allowed_headers(AllowedHeaders::all())
        .allow_credentials(true)
        .expose_headers(vec!["Access-Control-Allow-Origin".to_string()].into_iter().collect())
        .to_cors()
        .expect("Failed to create CORS fairing");

    println!("Current working directory: {:?}", env::current_dir().unwrap());
    
    let _ = rocket::build()
        .mount("/", FileServer::from("frontend/build"))
        .mount("/api", routes![
            routes::get_player,
            routes::get_galaxy_map,
            routes::get_star_system,
            routes::get_owner_fleets,
            routes::get_fleet,
            routes::move_fleet,
            routes::get_fleet_owners,
            routes::initiate_combat,
            routes::check_for_encounter,
            routes::trade_with_trader,
            routes::get_planet_market,
            routes::get_planet_ship_market,
            routes::buy_from_planet,
            routes::sell_to_planet,
            routes::list_games,
            routes::load_game,
            routes::create_new_game,
            routes::get_settings,
            routes::update_settings,
            routes::delete_game,
            routes::buy_ship,
            routes::sell_ship,
            routes::trade_in_ship,
            routes::get_player_fleets,
            routes::clear_caches,
        ])
        .attach(cors)
        .register("/", catchers![internal_error])
        .configure(rocket::Config::figment()
            .merge(("address", "0.0.0.0"))
            .merge(("port", 8000)))
        .launch()
        .await;
}
