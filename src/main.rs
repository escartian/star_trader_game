mod constants;
mod models;
use models::position;
use rocket::form::name;
use rocket::{get, routes};
use serde_json::{to_writer, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use crate::combat::combat::{auto_resolve_ship_combat, CombatResult};
use crate::models::galaxy::generate_galaxy;
use crate::models::player::Player;
use crate::models::ship::ship::Ship;
use crate::models::star_system::StarSystem;
use crate::models::trader::{Trader, TraderPersonality};
mod combat;
use lazy_static::lazy_static;

lazy_static! {
    static ref GALACTIC_MAP: Vec<StarSystem> = generate_galaxy(10);
}

#[get("/galaxy_map")]
fn get_galaxy_map() -> String {
    let galaxy_map = GALACTIC_MAP.to_vec();
    serde_json::to_string(&galaxy_map).unwrap()
}
#[get("/star_system/<id>")]
fn get_star_system(id: usize) -> Option<String> {
    GALACTIC_MAP
        .get(id)
        .map(|system| serde_json::to_string(system).unwrap())
}

#[rocket::main]
async fn main() {
    /*
        //WORLD GENERATION
        // Generate the galaxy map

        let galactic_map = generate_galaxy(10);

        // Print the generated world map
        //println!("{:#?}", galactic_map);
        // Create the path to the JSON file
        let data_path = Path::new("data").join("game").join("GameWorld.json");

        // Create the file and handle any errors
        let file = match File::create(&data_path) {
            Ok(file) => file,
            Err(e) => panic!("Failed to create file: {}", e),
        };

        // Write the galaxy map to the file
        match to_writer(&file, &galactic_map) {
            Ok(_) => println!("Successfully wrote galaxy map to file"),
            Err(e) => panic!("Failed to write galaxy map to file: {}", e),
        }

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
        .mount("/", routes![get_galaxy_map, get_star_system])
        .launch()
        .await
        .unwrap();
}
