//! The battle volume: cells, distances, movement range and deployment edges.
use crate::combat::engagement::Side;
use serde::{Deserialize, Serialize};

/// Cells along x, the axis between the two deployment edges.
pub const WIDTH: i8 = 9;
/// Cells along y.
pub const DEPTH: i8 = 9;
/// Cells along z (vertical layers).
pub const HEIGHT: i8 = 5;

/// A grid cell; valid cells satisfy [`Cell::in_bounds`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Cell {
    /// Chebyshev distance in cells.
    pub fn distance(self, other: Cell) -> u32 {
        let axis = |a: i8, b: i8| u32::from((i16::from(a) - i16::from(b)).unsigned_abs());
        axis(self.x, other.x)
            .max(axis(self.y, other.y))
            .max(axis(self.z, other.z))
    }

    pub fn in_bounds(self) -> bool {
        (0..WIDTH).contains(&self.x)
            && (0..DEPTH).contains(&self.y)
            && (0..HEIGHT).contains(&self.z)
    }
}

/// Cells a ship may move per turn: 1 + engine / 3 (engine 3 → 2, engine 8 → 3).
pub fn move_range(engine: u32) -> u32 {
    1 + engine / 3
}

/// The x column a side deploys on and withdraws from.
pub fn home_edge(side: Side) -> i8 {
    match side {
        Side::Player => 0,
        Side::Hostile => WIDTH - 1,
    }
}

/// Deployment: a side's ships fill its second column (one cell in from the edge,
/// so a damaged ship can step back and withdraw), nearest the centre first.
pub(super) fn deployment_cell(side: Side, rank: usize) -> Cell {
    let x = match side {
        Side::Player => 1,
        Side::Hostile => WIDTH - 2,
    };
    let (mid_y, mid_z) = (DEPTH / 2, HEIGHT / 2);
    let mut cells: Vec<Cell> = (0..DEPTH)
        .flat_map(|y| (0..HEIGHT).map(move |z| Cell { x, y, z }))
        .collect();
    cells.sort_by_key(|cell| {
        (
            (cell.y - mid_y).abs() * 2 + (cell.z - mid_z).abs(),
            cell.y,
            cell.z,
        )
    });
    cells[rank % cells.len()]
}
