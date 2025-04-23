pub mod models;
pub mod constants;
pub mod combat;
pub mod encounters;
pub mod routes;
pub mod game_state;

pub use constants::*;

#[cfg(test)]
mod tests {
    mod test_fleet_movement;
} 