use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::models::star_system::StarSystem;
use lazy_static::lazy_static;

// Core game constants
pub const PRINT_DEBUG: bool = true;

// TODO: Implement fleet size limits
// pub const MAX_FLEET_SHIP_COUNT: u32 = 10;

//Current Game Constants (these can change between games)
//pub(crate) const MAX_FLEET_SHIP_COUNT: u32 = 10;
//pub(crate) const HOST_PLAYER_ID: u32 = 0;\
//pub const HOST_PLAYER_NAME: &str = "Player";

lazy_static! {
    pub static ref GLOBAL_GAME_WORLD: Mutex<Vec<StarSystem>> = Mutex::new(Vec::new());
    pub static ref GAME_GENERATED: AtomicBool = AtomicBool::new(false);
}