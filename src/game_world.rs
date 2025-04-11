use crate::models::star_system::StarSystem;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref GLOBAL_GAME_WORLD: Mutex<Vec<StarSystem>> = Mutex::new(Vec::new());
}

pub fn get_global_game_world() -> Vec<StarSystem> {
    if let Ok(guard) = GLOBAL_GAME_WORLD.lock() {
        guard.clone()
    } else {
        Vec::new()
    }
} 