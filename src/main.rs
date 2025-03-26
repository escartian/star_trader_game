mod constants;
mod models;
mod routes;
mod engine;

use rocket::{get, routes, Request, Response};
use rocket_dyn_templates::{Template, tera::Tera, context};

//use serde ::{Deserialize, Serialize};
//use serde_json::{to_writer, Result};

use rocket::catchers;
use serde_json::to_writer;

use std::fs;
use std::env;

use std::fs::File;
use std::path::Path;
use std::string::String;

use crate::routes::*;

//use crate::combat::combat::{auto_resolve_ship_combat, CombatResult};
//use crate::models::player::Player;
//use crate::models::ship::ship::Ship;
use crate::models::star_system::StarSystem;
use crate::models::trader::Trader;
mod combat;
use lazy_static::lazy_static;
use std::io::Read;


use crate::constants::HOST_PLAYER_NAME;
use crate::constants::GAME_ID;
use crate::constants::STAR_COUNT;
use crate::models::game_world;
use crate::models::game_world::create_game_world_file;

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


pub(crate) fn get_global_game_world() -> Vec<StarSystem> {
    if GLOBAL_GAME_WORLD.is_empty() {
        println!("Game world is empty");
        create_game_world_file(GAME_ID, true);
    }else{println!("Game world is not empty")};
    
    GLOBAL_GAME_WORLD.clone()
}

/// The main entry point for the Rocket application.
///
/// This function is responsible for launching the Rocket server and mounting
/// the routes.
#[rocket::main]
async fn main() {

    /***On game launch create the game. ***/
    let gameworld = get_global_game_world();
    let player = get_player(HOST_PLAYER_NAME);



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
    println!("Current working directory: {:?}", env::current_dir().unwrap());
    let template_dir = Path::new("src").join("templates");
    println!("Template directory: {:?}", template_dir);
    rocket::build()
        .mount("/", routes![index, get_player, get_galaxy_map, get_star_system, get_fleet])
        .attach(Template::fairing())
        .register("/", catchers![internal_error])
        .launch()
        .await
        .unwrap();
}
