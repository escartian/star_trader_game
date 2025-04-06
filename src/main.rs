mod constants;
mod models;
mod routes;
mod engine;
mod combat;
mod encounters;

use rocket::{get, routes, Request, Response};
use std::sync::atomic::Ordering;

use rocket::catchers;
use serde_json::to_writer;

use std::fs;
use std::env;

use std::fs::File;
use std::path::Path;
use std::string::String;

use crate::routes::*;

use crate::models::player::Player;
use crate::models::star_system::StarSystem;
use crate::models::trader::Trader;
use lazy_static::lazy_static;
use std::io::Read;
use std::sync::Mutex;

use crate::models::settings::{GameSettings, load_settings, save_settings};
use crate::models::game_world;
use crate::models::game_world::create_game_world_file;
use crate::models::faction::{Faction, save_faction};
use crate::models::fleet::generate_and_save_fleet;
use crate::models::position::Position;
use crate::models::position::random_position;

use rocket_cors::{AllowedOrigins, CorsOptions, AllowedHeaders};
use rocket::fs::FileServer;

use crate::models::game_state::GameState;

lazy_static! {
    static ref EMPTY_WORLD: Vec<StarSystem> = Vec::new();
    static ref GLOBAL_GAME_WORLD: Mutex<Vec<StarSystem>> = {
        println!("Initializing empty game world");
        Mutex::new(Vec::new())
    };

    static ref GAME_STATE: Mutex<GameState> = {
        println!("Initializing game state");
        Mutex::new(GameState {
            settings: GameSettings::default(),
            credits: 0.0,
        })
    };
}

pub(crate) fn get_global_game_world() -> Vec<StarSystem> {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
}

pub(crate) fn get_game_state() -> GameState {
    if let Ok(guard) = GAME_STATE.lock() {
        guard.clone()
    } else {
        GameState {
            settings: GameSettings::default(),
            credits: 0.0,
        }
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
            routes::move_within_system,
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
