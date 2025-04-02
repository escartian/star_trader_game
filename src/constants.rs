use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use crate::models::star_system::StarSystem;

pub(crate) const PRINT_DEBUG: bool = true;
pub(crate) const MAX_COMBAT_TIME: u32 = 500;

pub(crate) const MAP_WIDTH: i32 = 100;
pub(crate) const MAP_HEIGHT: i32 = 100;
pub(crate) const MAP_LENGTH: i32 = 100;
pub(crate) const INITIAL_CREDIT_COUNT: f32 = 1000.0;

//Current Game Constants (these can change between games)
//pub(crate) const MAX_FLEET_SHIP_COUNT: u32 = 10;
pub(crate) const HOST_PLAYER_NAME: &str = "Igor";
pub(crate) const HOST_PLAYER_ID: u32 = 0;
pub(crate) const GAME_ID: &str = "1";
pub(crate) const STAR_COUNT: i32 = 10;

pub(crate) static GAME_GENERATED: AtomicBool = AtomicBool::new(false);