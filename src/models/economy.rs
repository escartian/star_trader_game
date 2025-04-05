use strum_macros::EnumIter;
use serde::{Deserialize, Serialize};
use std::fmt;
use rand::distributions::{Distribution, Standard};
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, EnumIter, Serialize, Deserialize)]
pub enum Economy {
    Booming,
    Growing,
    Stable,
    Struggling,
    Declining,
    Crashing,
    Nonexistent,
}

impl fmt::Display for Economy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Distribution<Economy> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Economy {
        match rng.gen_range(0..7) {
            0 => Economy::Booming,
            1 => Economy::Growing,
            2 => Economy::Stable,
            3 => Economy::Struggling,
            4 => Economy::Declining,
            5 => Economy::Crashing,
            _ => Economy::Nonexistent,
        }
    }
} 