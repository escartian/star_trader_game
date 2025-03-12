mod constants;
mod models;
use models::position;

use rocket::form::name;
use rocket::{get, routes, Request, Response};
use rocket_dyn_templates::{Template, tera::Tera, context};

use serde ::{Deserialize, Serialize};
use serde_json::{to_writer, Result};

use rocket::response::content;
use rocket::catchers;
use rocket::catch;

use std::fs;

use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::string::String;
use rand::Rng;

use crate::combat::combat::{auto_resolve_ship_combat, CombatResult};
use crate::models::galaxy::generate_galaxy;
use crate::models::player::Player;
use crate::models::ship::ship::Ship;
use crate::models::star_system::StarSystem;
use crate::models::trader::{Trader, TraderPersonality};
use crate::models::player::create_player;
mod combat;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::io::Read;
use std::ptr::null;

use crate::constants::HOST_PLAYER_NAME;
use crate::constants::GAME_ID;
use crate::constants::STAR_COUNT;

lazy_static! {
    static ref GLOBAL_GAME_WORLD: Vec<StarSystem> = {
        println!("Loading Game World");
        let data_path = Path::new("data")
            .join("game")
            .join(GAME_ID)
            .join("GameWorld.json");

            println!("{}", data_path.display());
            if data_path.exists() && data_path.is_file() {
                
                let file = File::open(data_path);
                let mut contents = String::new();
                file.expect("REASON").read_to_string(&mut contents);
                serde_json::from_str(&contents).unwrap()
        } else {
            println!("Game world is empty");
            create_game_world_file(GAME_ID, true)
        }
    };
}

#[catch(500)]
fn internal_error(_req: &Request) -> Template {
    Template::render("500", ())
}

/// Handles the root route (`/`) and renders the index page
#[get("/")]
fn index() -> Template {
    let mut context = HashMap::new();
    context.insert("player_name", HOST_PLAYER_NAME);
    Template::render("index", &context)
}

#[get("/player/<name>")]
fn get_player(name: String) -> String {
    let data_path = Path::new("data")
        .join("game")
        .join(GAME_ID)
        .join("players")
        .join(format!("{}.json", name));

    let file = File::open(data_path);
    let mut contents = String::new();
    file.expect("REASON").read_to_string(&mut contents);

    return contents;
}


// Returns a serialized JSON string representation of the galaxy map.
// This function handles the `/galaxy_map` route and provides the client
// with the current state of the galaxy map as a JSON string.
#[get("/galaxy_map")]
fn get_galaxy_map() -> String {
    serde_json::to_string(&get_global_game_world()).unwrap()
}
// Returns a star system with the given id from the galaxy map as a serialized JSON string
#[get("/star_system/<id>")]
fn get_star_system(id: usize) -> Option<String> {
    get_global_game_world()
        .get(id)
        .map(|system| serde_json::to_string(system).unwrap())
}

fn get_global_game_world() -> Vec<StarSystem> {
    if GLOBAL_GAME_WORLD.is_empty() {
        println!("Game world is empty");
        create_game_world_file(GAME_ID, true);
    }else{println!("Game world is not empty")};
    
    GLOBAL_GAME_WORLD.clone()
}

/** Creates a new game world file in the specified game directory and writes the generated
* galaxy map to it. The file is stored in the "data/game/<game_id>/GameWorld.json" directory.
* If the necessary directories do not exist, this function will create them.
*
* # Arguments
* - `game_id` - A string slice that holds the identifier for the game instance.
*
* # Errors
* This function will panic if it is unable to create the file or write the galaxy map to it.
**/
fn create_game_world_file(game_id: &str, empty_world: bool) ->Vec<StarSystem>{
    //WORLD GENERATION
    // Generate the galaxy map
    println!("Creating Game World File");
    let galactic_map;
    if empty_world{   
        galactic_map = generate_galaxy(STAR_COUNT);
        let data_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("GameWorld.json");

        // Create the necessary directories if they don't exist
        if let Some(parent) = data_path.parent() {
            println!("{}", parent.display());
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
        
        // Create the file and handle any errors
        let file = match File::create(&data_path) {
            Ok(file) => file,
            Err(e) => panic!("Failed to create file: {}", e),
        };

        println!("Game world file created at: {:?}", data_path);

        // Write the galaxy map to the file
        match to_writer(&file, &galactic_map) {
            Ok(_) => println!("Successfully wrote galaxy map to file"),
            Err(e) => panic!("Failed to write galaxy map to file: {}", e),

        }
        return galactic_map;
    }else{
        println!("Game World Already Exists");
        let data_path = Path::new("data")
        .join("game")
        .join(game_id)
        .join("GameWorld.json");
        let file = File::open(data_path);
        let mut contents = String::new();
        file.expect("REASON").read_to_string(&mut contents);
        galactic_map = serde_json::from_str(&contents).unwrap();

        return galactic_map
    }
    panic!("World failed to generate!");
    return generate_galaxy(1);
}

/** Creates a new player with the specified name and saves their data to a JSON file 
* within the game directory. If necessary, creates the required directories. 
* Returns the newly created Player object.
* # Arguments
* - `game_id` - A string slice that holds the identifier for the game instance.
* - `player_name` - A string slice that holds the name of the player.
**/


/// The main entry point for the Rocket application.
///
/// This function is responsible for launching the Rocket server and mounting
/// the routes.
#[rocket::main]
async fn main() {

    /***On game launch create the game. ***/
    let gameworld = get_global_game_world();
    let player = create_player(GAME_ID, HOST_PLAYER_NAME);



        /*
        //TRADING
        let player_name = String::from("Igor");
        let mut player: Player = Player::new(&player_name);
        println!("{:?}", player);
        let trader_personality = rand::random::<TraderPersonality>();
        let mut trader1 = Trader::new(trader_personality);

        let trader_personality = rand::random::<TraderPersonality>();
        let mut trader2 = Trader::new(trader_personality);
        let quote = trader1.get_opening_line(&galactic_map[0].planets[0]);
        println!("{}", quote);

        println!("{:?}", player.resources[0]);
        println!("{}", player.credits);
        let result = trader1.buy_resource(models::resource::ResourceType::Water, 10, &mut player);
        println!("{:?}", result);

        println!("{:?}", player.resources[0]);
        println!("{}", player.credits);
        let result = trader2.buy_resource(models::resource::ResourceType::Water, 10, &mut player);

        println!("{:?}", player.resources[0]);
        println!("{}", player.credits);
        println!("{:?}", result);
        let result = trader1.sell_resource(models::resource::ResourceType::Water, 10, &mut player);
        println!("{:?}", result);

        // SHIP/FLEET GENERATION AND AUTO RESOLVE BATTLE TEST

        let player_name = String::from("Igor");

        let ship_count_player = 5;
        let ship_count_computer = 5;

        let mut fleet1 = Vec::new();
        let mut fleet2 = Vec::new();

        for _ in 0..ship_count_player {
            let mut ship = rand::random::<Ship>();
            ship.owner = player_name.to_string();
            fleet1.push(ship);
        }

        for _ in 0..ship_count_computer {
            let ship = rand::random::<Ship>();
            //println!("{:#?}",ship);
            fleet2.push(ship);
        }

        let result = auto_resolve_ship_combat(&mut fleet1, &mut fleet2);

        match result {
            CombatResult::AttackersVictory(remaining_ships) => {
                println!("Attackers won with the following ships remaining:");
                for ship in remaining_ships {
                    println!("- {}", ship.name);
                }
            }
            CombatResult::DefendersVictory(remaining_ships) => {
                println!("Defenders won with the following ships remaining:");
                for ship in remaining_ships {
                    println!("- {}", ship.name);
                }
            }
            CombatResult::TotalDestruction() => {
                println!("All ships were destroyed in the battle");
            }
            CombatResult::TimedOut(attacking_ships, defending_ships) => {
                println!("Auto Combat took too long");
                println!("Attacker's surviving ships");
                for ship in attacking_ships {
                    println!("- {}", ship.name);
                }
                println!("Defender's surviving ships");
                for ship in defending_ships {
                    println!("- {}", ship.name);
                }
            }
        }
    */
    rocket::build()
        .mount("/", routes![index, get_player,get_galaxy_map, get_star_system])
        .attach(Template::fairing())
        .register("/", catchers![internal_error])
        .launch()
        .await
        .unwrap();
}
