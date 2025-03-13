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