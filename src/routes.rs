use crate::get_global_game_world;
use crate::GAME_ID;
use std::fs::File;
use std::path::Path;
use rocket_dyn_templates::Template;
use crate::HOST_PLAYER_NAME;
use std::collections::HashMap;
use rocket::Request;
use rocket::get;
use std::io::Read;
use crate::models::star_system::StarSystem;
use rocket::catch;
use rocket::serde::json::Json;
use crate::models::fleet::Fleet;
use rand;


#[catch(500)]
pub fn internal_error(_req: &Request) -> Template {
    Template::render("500", ())
}

/// Handles the root route (`/`) and renders the index page
#[get("/")]
pub fn index() -> Template {
    let mut context = HashMap::new();
    context.insert("player_name", HOST_PLAYER_NAME);
    Template::render("index", &context)
}

#[get("/player/<name>")]
pub fn get_player(name: &str) -> String {
    let data_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("players")
        .join(format!("{}.json", name));

    let file = File::open(data_path);
    let mut contents = String::new();
    file.expect("REASON").read_to_string(&mut contents);

    contents
}

// Returns a serialized JSON string representation of the galaxy map
#[get("/galaxy_map")]
pub fn get_galaxy_map() -> String {
    serde_json::to_string(&get_global_game_world()).unwrap()
}

// Returns a star system with the given id from the galaxy map as a serialized JSON string
#[get("/star_system/<id>")]
pub fn get_star_system(id: usize) -> Option<String> {
    get_global_game_world()
        .get(id)
        .map(|system| serde_json::to_string(system).unwrap())
}

#[get("/fleets/<owner_id>")]
pub fn get_owner_fleets(owner_id: String) -> Json<Vec<Fleet>> {
    println!("Getting fleets for owner: {}", owner_id);

    match crate::models::fleet::list_owner_fleets(&owner_id) {
        Ok(fleets) => {
            println!("Found {} fleets for owner", fleets.len());
            Json(fleets)
        }
        Err(e) => {
            println!("Error loading fleets: {}", e);
            Json(Vec::new())
        }
    }
}

#[get("/fleet/<owner_id>/<fleet_number>")]
pub fn get_fleet(owner_id: String, fleet_number: usize) -> Json<Option<Fleet>> {
    println!("Getting fleet {} for owner: {}", fleet_number, owner_id);

    // Try to load existing fleet
    let fleet_name = format!("Fleet_{}_{}", owner_id, fleet_number);
    println!("Looking for fleet with name: {}", fleet_name);
    
    match crate::models::fleet::load_fleet(&fleet_name) {
        Ok(fleet) => {
            println!("Fleet loaded successfully");
            Json(fleet)
        }
        Err(e) => {
            println!("Error loading fleet: {}", e);
            Json(None)
        }
    }
} 