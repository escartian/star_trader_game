mod constants;
mod models;
use crate::combat::combat::{CombatResult, auto_resolve_ship_combat};
use crate::models::galaxy::generate_galaxy;
use crate::models::ship::ship::Ship;
mod combat;

fn main() {
    let galactic_map = generate_galaxy(10);
    let player_name = String::from("Igor");
    // Print the generated world map
    println!("{:#?}", galactic_map);

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
}
