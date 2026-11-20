//! The battle board: hexes in axial coordinates, three altitude bands, ranges,
//! move costs and deployment. Each battle carries its own [`Board`], which
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

/// Altitude band. One ship per hex whatever its band, so bands never block a
/// hex; they change ranges and hit chances.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Band {
    Low,
    Level,
    High,
}

impl Band {
    pub const ALL: [Band; 3] = [Band::Low, Band::Level, Band::High];

    /// 0 (Low) to 2 (High).
    pub fn height(self) -> u8 {
        self as u8
    }

    /// Bands between this one and `other`.
    pub fn difference(self, other: Band) -> u32 {
        u32::from(self.height().abs_diff(other.height()))
    }

    pub fn above(self) -> Option<Band> {
        Self::ALL.get(usize::from(self.height()) + 1).copied()
    }

    pub fn below(self) -> Option<Band> {
        self.height()
            .checked_sub(1)
            .map(|height| Self::ALL[usize::from(height)])
    }

    pub fn label(self) -> &'static str {
        match self {
            Band::Low => "Low",
            Band::Level => "Level",
            Band::High => "High",
        }
    }
}

/// Where a ship stands: a hex and a band.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub hex: Hex,
    pub band: Band,
}

impl Position {
    pub const fn new(hex: Hex, band: Band) -> Self {
        Self { hex, band }
    }

    /// Weapon range and move cost alike: hex steps plus bands crossed.
    pub fn range(self, other: Position) -> u32 {
        self.hex.distance(other.hex) + self.band.difference(other.band)
    }
}

/// Hit-chance modifier, in percentage points, for firing from `from` at a ship
/// in `to`: +10 at a lower band, −10 at a higher one, 0 on the same band.
pub fn high_ground_percent(from: Band, to: Band) -> i32 {
    match from.cmp(&to) {
        std::cmp::Ordering::Greater => HIGH_GROUND_PERCENT,
        std::cmp::Ordering::Less => -HIGH_GROUND_PERCENT,
        std::cmp::Ordering::Equal => 0,
    }
}

/// Hit-chance bonus for firing down a band (and penalty for firing up).
pub const HIGH_GROUND_PERCENT: i32 = 10;

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
    /// alternating outward, all on the Level band. A side never has more ships
    /// than that column holds (radius + 2 hexes, see the tests); should it,
    /// further ships take the next nearest columns.
    pub(super) fn deployment(self, side: Side, rank: usize) -> Position {
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
        Position::new(hexes[rank % hexes.len()], Band::Level)
    }

    /// Whether a saved battle with `ships` ships may use this board.
    pub fn valid_for(self, ships: usize) -> bool {
        self == Self::for_ships(ships)
    }
}

/// Movement points per turn: 1 + engine / 3 (engine 3 → 2, engine 8 → 3). A
/// hex step costs one point, and so does a climb or dive of one band.
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
    fn distances_count_hex_steps_and_bands() {
        let origin = Hex::new(0, 0);
        assert_eq!(origin.distance(Hex::new(3, -1)), 3);
        assert_eq!(origin.distance(Hex::new(-2, 2)), 2);
        assert_eq!(Hex::new(-4, 0).distance(Hex::new(4, 0)), 8);
        let low = Position::new(origin, Band::Low);
        assert_eq!(low.range(Position::new(Hex::new(2, 0), Band::High)), 4);
        assert_eq!(Band::Level.above(), Some(Band::High));
        assert_eq!(Band::High.above(), None);
        assert_eq!(Band::Low.below(), None);
        assert_eq!(high_ground_percent(Band::High, Band::Low), 10);
        assert_eq!(high_ground_percent(Band::Low, Band::Level), -10);
        assert_eq!(high_ground_percent(Band::Level, Band::Level), 0);
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
                let unique: std::collections::HashSet<_> = placed.iter().map(|at| at.hex).collect();
                assert_eq!(unique.len(), side_size);
                for at in &placed {
                    assert!(board.contains(at.hex));
                    assert_eq!((at.hex.q - board.home_column(side)).abs(), 1);
                    assert_eq!(at.band, Band::Level);
                }
            }
        }
        let board = Board::for_ships(2);
        let player = board.deployment(Side::Player, 0).hex;
        let raider = board.deployment(Side::Hostile, 0).hex;
        assert_eq!(player.distance(raider), 6, "a duel opens six hexes apart");
    }
}
