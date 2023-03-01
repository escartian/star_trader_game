use rand::{Rng};
use serde::{Deserialize, Serialize};

//how to create an instance of type Position
//let pos = Position { x: 3, y: 4, z: 0 };
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
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

    pub fn random_position(x_range: i32, y_range: i32, z_range: i32) -> Position {
        let mut rng = rand::thread_rng();

        let x = rng.gen_range(-x_range..=x_range);
        let y = rng.gen_range(-y_range..=y_range);
        let z = rng.gen_range(-z_range..=z_range);

        Position { x, y, z }
    }