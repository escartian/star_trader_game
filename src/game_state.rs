use std::sync::Mutex;
use lazy_static::lazy_static;
use crate::models::game_state::GameState;

lazy_static! {
    pub static ref GAME_STATE: Mutex<GameState> = {
        println!("Initializing game state");
        Mutex::new(GameState {
            current_game_id: None,
            credits: 0.0,
        })
    };
} 