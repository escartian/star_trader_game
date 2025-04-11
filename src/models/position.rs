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
    //To not loose presicison whilst calculating integer position distance, floating point is used.
    //How to use the distance function between two points.
    //let distance = pos1.distance(&pos2);
    //println!("Distance between pos1 and pos2: {}", distance);
    pub fn distance(&self, other: &Position) -> f64 {
        let x_dist = (self.x - other.x) as f64;
        let y_dist = (self.y - other.y) as f64;
        let z_dist = (self.z - other.z) as f64;

        ((x_dist * x_dist) + (y_dist * y_dist) + (z_dist * z_dist)).sqrt()
    }

    pub fn is_within_local_bounds(&self, settings: &GameSettings) -> bool {
        let max_coord = (settings.map_width as i32);
        let min_coord = -(settings.map_width as i32);
        self.x >= min_coord && self.x <= max_coord &&
        self.y >= min_coord && self.y <= max_coord &&
        self.z >= min_coord && self.z <= max_coord
    }
}

//since a random position is used often during world gen, this function was created
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

pub fn random_position(map_width: i32, map_height: i32, map_length: i32) -> Position {
    let mut rng = rand::thread_rng();
    Position {
        x: rng.gen_range(-map_width..=map_width),
        y: rng.gen_range(-map_height..=map_height),
        z: rng.gen_range(-map_length..=map_length),
    }
}

#[cfg(test)]
mod tests {
    use crate::models::position::Position;

    #[test]
    fn test_distance() {
        let pos1 = Position { x: 0, y: 0, z: 0 };
        let pos2 = Position { x: 3, y: 4, z: 5 };
        let distance = pos1.distance(&pos2);
        assert_eq!(distance, 7.0710678118654755);
    }
}
