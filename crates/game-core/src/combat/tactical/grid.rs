//! The battle volume: cells, distances, movement range and deployment edges.
//! Each battle carries its own [`GridSize`], which grows with the fleets.
use crate::combat::engagement::Side;
use serde::{Deserialize, Serialize};

/// Grid edge for a duel; one more cell per extra pair of ships.
pub const DUEL_EDGE: i8 = 10;
/// Grid edge for the largest battle (`MAX_COMBATANTS` ships).
pub const MAX_EDGE: i8 = 17;

/// A grid cell; valid cells satisfy [`GridSize::contains`].
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
}

/// Cells along each axis: x runs between the two deployment edges, y across
/// them, z is the vertical layers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridSize {
    pub width: i8,
    pub depth: i8,
    pub height: i8,
}

impl Default for GridSize {
    /// The fixed 9 × 9 × 5 volume of battles saved before grids scaled.
    fn default() -> Self {
        Self {
            width: 9,
            depth: 9,
            height: 5,
        }
    }
}

impl GridSize {
    /// A cube whose edge is 10 for a duel and grows by one cell for every two
    /// more ships (`9 + ships / 2`), up to [`MAX_EDGE`] for 16 ships.
    pub fn for_ships(ships: usize) -> Self {
        let edge = (9 + ships / 2).clamp(DUEL_EDGE as usize, MAX_EDGE as usize) as i8;
        Self {
            width: edge,
            depth: edge,
            height: edge,
        }
    }

    pub fn contains(self, cell: Cell) -> bool {
        (0..self.width).contains(&cell.x)
            && (0..self.depth).contains(&cell.y)
            && (0..self.height).contains(&cell.z)
    }

    /// The x column a side withdraws from.
    pub fn home_edge(self, side: Side) -> i8 {
        match side {
            Side::Player => 0,
            Side::Hostile => self.width - 1,
        }
    }

    /// Deployment: a side's ships fill its second column (one cell in from the
    /// edge, so a damaged ship can step back and withdraw) in a checkerboard
    /// around the centre: no two ships share a face, so even a large fleet stays
    /// readable. Order: the centre cell, then the middle layer, then the layers
    /// beside it. Every grid has room for a full side on the checkerboard.
    pub(super) fn deployment_cell(self, side: Side, rank: usize) -> Cell {
        let x = match side {
            Side::Player => 1,
            Side::Hostile => self.width - 2,
        };
        let (mid_y, mid_z) = (self.depth / 2, self.height / 2);
        let mut cells: Vec<Cell> = (0..self.depth)
            .flat_map(|y| (0..self.height).map(move |z| Cell { x, y, z }))
            .collect();
        cells.sort_by_key(|cell| {
            let (dy, dz) = ((cell.y - mid_y).abs(), (cell.z - mid_z).abs());
            let off_lattice = (dy + dz) % 2 != 0;
            (off_lattice, dy + dz, dz, cell.y, cell.z)
        });
        cells[rank % cells.len()]
    }

    /// Whether a saved battle may use this size: the default (older saves) or
    /// the size its ship count gives.
    pub fn valid_for(self, ships: usize) -> bool {
        self == Self::default() || self == Self::for_ships(ships)
    }
}

/// Cells a ship may move per turn: 1 + engine / 3 (engine 3 → 2, engine 8 → 3).
pub fn move_range(engine: u32) -> u32 {
    1 + engine / 3
}
