use rand::Rng;
use serde::{Deserialize, Serialize};
use crate::models::settings::GameSettings;

//how to create an instance of type Position
//let pos = Position { x: 3, y: 4, z: 0 };
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Position {
    ///To not loose presicison whilst calculating integer position distance, floating point is used.
    /// # Usage:
    ///
    /// ```
    /// # use star_trader_game::models::position::Position;
    /// let a = Position { x: 0, y: 0, z: 0 };
    /// let b = Position { x: 1, y: 2, z: 2 };
    /// assert_eq!(a.distance(&b), 3.0);
    /// ```
    pub fn distance(&self, other: &Position) -> f64 {
        let x_dist = (self.x - other.x) as f64;
        let y_dist = (self.y - other.y) as f64;
        let z_dist = (self.z - other.z) as f64;

        ((x_dist * x_dist) + (y_dist * y_dist) + (z_dist * z_dist)).sqrt()
    }

    pub fn is_within_local_bounds(&self, settings: &GameSettings) -> bool {
        let max_coord = settings.map_width as i32;
        let min_coord = -(settings.map_width as i32);
        self.x >= min_coord && self.x <= max_coord &&
        self.y >= min_coord && self.y <= max_coord &&
        self.z >= min_coord && self.z <= max_coord
    }
}


/// Generates a random position within the specified range, ensuring that the generated position is not 0,0,0
/// because this position is reserved for the star at the center of the star system.
///
/// # Arguments
/// * `x_range` - The range of the x coordinate
/// * `y_range` - The range of the y coordinate
/// * `z_range` - The range of the z coordinate
///
/// # Returns
/// A new `Position` with a random, but non-zero, x, y and z coordinate within the specified range.
/// # Usage:
/// ```
/// # use star_trader_game::models::position::random_nonzero_position;
/// let pos = random_nonzero_position(10, 10, 10);
/// assert!(pos.x != 0 && pos.y != 0 && pos.z != 0);
/// ```
pub fn random_nonzero_position(x_range: i32, y_range: i32, z_range: i32) -> Position {
    let mut rng = rand::thread_rng();

    let mut x = rng.gen_range(-x_range..=x_range);
    let mut y = rng.gen_range(-y_range..=y_range);
    let mut z = rng.gen_range(-z_range..=z_range);
    //in the unlikely case that position values are 0,0,0, rerun random until not 0,0,0
    //as this position is reserved for the star at the center of the star system
    while x == 0 && y == 0 && z == 0 {
        x = rng.gen_range(-x_range..=x_range);
        y = rng.gen_range(-y_range..=y_range);
        z = rng.gen_range(-z_range..=z_range);
    }

    Position { x, y, z }
}

/// Generates a random position within the specified map dimensions.
///
/// # Arguments
/// * `map_width` - The range for the x coordinate (-map_width to map_width)
/// * `map_height` - The range for the y coordinate (-map_height to map_height)
/// * `map_length` - The range for the z coordinate (-map_length to map_length)
///
/// # Returns
/// A `Position` with random x, y, and z coordinates within the specified ranges.
/// # Usage:
/// ```
/// # use star_trader_game::models::position::random_position;
/// let pos = random_position(10, 10, 10);
/// assert!(pos.x >= -10 && pos.x <= 10 && pos.y >= -10 && pos.y <= 10 && pos.z >= -10 && pos.z <= 10);
/// ```
pub fn random_position(map_width: i32, map_height: i32, map_length: i32) -> Position {
    let mut rng = rand::thread_rng();
    Position {
        x: rng.gen_range(-map_width..=map_width),
        y: rng.gen_range(-map_height..=map_height),
        z: rng.gen_range(-map_length..=map_length),
    }
}
