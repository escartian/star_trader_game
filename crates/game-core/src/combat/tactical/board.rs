//! The battle board: hexes in axial coordinates, ranges, move costs and
//! deployment. Terrain lives in `terrain`. Each battle carries its own [`Board`], which
//! grows with the fleets.
//!
//! Axial coordinates (q, r), with the implied third coordinate s = −q − r.
//! Columns of constant q run between the two deployment edges: the player's
//! edge is the column q = −radius, the raiders' edge q = +radius. Drawn with
//! flat-topped hexes, a hex sits at x = 1.5 q, y = √3 (r + q / 2) in units of
//! the hex's circumradius, so each column is vertical and r grows downward.
use crate::combat::engagement::Side;
use serde::{Deserialize, Serialize};

/// Board radius (rings around the centre hex) for up to three ships.
pub const MIN_RADIUS: i8 = 4;
/// Board radius for the largest battles.
pub const MAX_RADIUS: i8 = 6;

/// One hex of the board; valid hexes satisfy [`Board::contains`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hex {
    pub q: i8,
    pub r: i8,
}

impl Hex {
    pub const fn new(q: i8, r: i8) -> Self {
        Self { q, r }
    }

    fn s(self) -> i16 {
        -i16::from(self.q) - i16::from(self.r)
    }

    /// Steps between two hexes.
    pub fn distance(self, other: Hex) -> u32 {
        let dq = (i16::from(self.q) - i16::from(other.q)).unsigned_abs();
        let dr = (i16::from(self.r) - i16::from(other.r)).unsigned_abs();
        let ds = (self.s() - other.s()).unsigned_abs();
        u32::from(dq.max(dr).max(ds))
    }

    /// Twice the vertical offset from the board's centre line (2r + q), an
    /// integer stand-in for y that orders hexes by closeness to the middle row.
    fn doubled_row(self) -> i16 {
        2 * i16::from(self.r) + i16::from(self.q)
    }
}

/// A hexagonal board of `radius` rings around the centre hex
/// (3r² + 3r + 1 hexes: 61 at radius 4, 127 at radius 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Board {
    pub radius: i8,
}

impl Board {
    /// Radius 4 for up to three ships and one more ring per four more ships
    /// (`4 + ships / 4`), at most [`MAX_RADIUS`].
    pub fn for_ships(ships: usize) -> Self {
        let rings = (ships / 4).min(usize::from((MAX_RADIUS - MIN_RADIUS) as u8));
        Self {
            radius: MIN_RADIUS + rings as i8,
        }
    }

    pub fn contains(self, hex: Hex) -> bool {
        let radius = i16::from(self.radius);
        i16::from(hex.q).abs() <= radius
            && i16::from(hex.r).abs() <= radius
            && hex.s().abs() <= radius
    }

    /// Every hex, column by column from the player's edge.
    pub fn hexes(self) -> impl Iterator<Item = Hex> {
        let radius = self.radius;
        (-radius..=radius)
            .flat_map(move |q| (-radius..=radius).map(move |r| Hex::new(q, r)))
            .filter(move |hex| self.contains(*hex))
    }

    /// The q column a side withdraws from.
    pub fn home_column(self, side: Side) -> i8 {
        match side {
            Side::Player => -self.radius,
            Side::Hostile => self.radius,
        }
    }

    /// Whether a ship of `side` on `hex` may withdraw.
    pub fn at_home_edge(self, side: Side, hex: Hex) -> bool {
        hex.q == self.home_column(side)
    }

    /// Deployment: a side's ships fill the column one in from its edge (so a
    /// damaged ship can step back and withdraw), middle row first, then
    /// alternating outward. A side never has more ships
    /// than that column holds (radius + 2 hexes, see the tests); should it,
    /// further ships take the next nearest columns.
    pub(super) fn deployment(self, side: Side, rank: usize) -> Hex {
        let column = match side {
            Side::Player => -self.radius + 1,
            Side::Hostile => self.radius - 1,
        };
        let mut hexes: Vec<Hex> = self.hexes().collect();
        hexes.sort_by_key(|hex| {
            let row = hex.doubled_row();
            (
                (hex.q - column).abs(),
                row.abs(),
                // Above the middle row before below it, for the player; the
                // raiders mirror that, so both sides read alike.
                if side == Side::Player { row } else { -row },
                hex.q,
            )
        });
        hexes[rank % hexes.len()]
    }

    /// Whether a saved battle with `ships` ships may use this board.
    pub fn valid_for(self, ships: usize) -> bool {
        self == Self::for_ships(ships)
    }
}

/// Movement points per turn: 1 + engine / 3 (engine 3 → 2, engine 8 → 3). A
/// hex step costs one point.
pub fn move_points(engine: u32) -> u32 {
    1 + engine / 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boards_grow_from_radius_four_to_six_with_the_fleets() {
        let radius = |ships| Board::for_ships(ships).radius;
        assert_eq!([radius(2), radius(3)], [4, 4]);
        assert_eq!([radius(4), radius(7)], [5, 5]);
        assert_eq!([radius(8), radius(11), radius(16)], [6, 6, 6]);
        assert_eq!(Board { radius: 4 }.hexes().count(), 61);
        assert_eq!(Board { radius: 6 }.hexes().count(), 127);
    }

    #[test]
    fn distances_count_hex_steps() {
        let origin = Hex::new(0, 0);
        assert_eq!(origin.distance(Hex::new(3, -1)), 3);
        assert_eq!(origin.distance(Hex::new(-2, 2)), 2);
        assert_eq!(Hex::new(-4, 0).distance(Hex::new(4, 0)), 8);
    }

    #[test]
    fn a_full_side_deploys_one_column_in_from_its_edge_in_distinct_hexes() {
        // The most ships a side brings is 8, in a battle of at least 9 ships;
        // a 6-ship side fights in a battle of at least 7.
        for (ships, side_size) in [(2, 1), (3, 2), (7, 6), (9, 8), (16, 8)] {
            let board = Board::for_ships(ships);
            for side in [Side::Player, Side::Hostile] {
                let placed: Vec<_> = (0..side_size)
                    .map(|rank| board.deployment(side, rank))
                    .collect();
                let unique: std::collections::HashSet<_> = placed.iter().collect();
                assert_eq!(unique.len(), side_size);
                for at in &placed {
                    assert!(board.contains(*at));
                    assert_eq!((at.q - board.home_column(side)).abs(), 1);
                }
            }
        }
        let board = Board::for_ships(2);
        let player = board.deployment(Side::Player, 0);
        let raider = board.deployment(Side::Hostile, 0);
        assert_eq!(player.distance(raider), 6, "a duel opens six hexes apart");
    }
}
