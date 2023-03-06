mod constants;
mod models;
//use crate::combat::combat::{CombatResult, auto_resolve_ship_combat};
//use crate::models::galaxy::generate_galaxy;
//use crate::models::player::Player;
use crate::models::resource::generate_resources;
//use crate::models::planet::Planet;
//use crate::models::ship::ship::Ship;
use crate::models::trader::{Trader, TraderPersonality};
mod combat;

fn main() {
//WORLD GENERATION TEST
    //let galactic_map = generate_galaxy(10);
    // Print the generated world map
    //println!("{:#?}", galactic_map);

    //let player_name = String::from("Igor");
    let trader_personality = rand::random::<TraderPersonality>();
    let mut trader1 = Trader::new(trader_personality, generate_resources());
    
    let trader_personality = rand::random::<TraderPersonality>();
    let mut trader2 = Trader::new(trader_personality, generate_resources());
    //let player = Player::new("Igor");
    //let quote = trader1.get_opening_line(&galactic_map[0].planets[0]);
    //println!("{}", quote);
    let result = trader1.sell_resource_trader_to_trader(models::resource::ResourceType::Fuel, 10, &mut trader2);

    println!("{:?}", result);

/* SHIP/FLEET GENERATION AND AUTO RESOLVE BATTLE TEST

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
        CombatResult::TimedOut(attacking_ships,defending_ships) => {
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
}
