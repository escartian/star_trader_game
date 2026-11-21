//! Battle terrain, movement paths and lines of fire.
//!
//! - **Asteroids** are impassable: no ship stands on or moves through one, and
//!   they block lines of fire.
//! - **Debris** is open ground that gives cover: shots at a ship standing in
//!   debris have [`COVER_PERCENT`] less chance to hit.
//! - Terrain comes from the battle seed ([`generate`]): `radius / 2` asteroid
//!   and debris hexes on each half of the board, mirrored through the centre so
//!   both sides face the same field. Edges and deployment columns stay clear.
//! - Movement follows the shortest path around asteroids (ships can be passed);
//!   a line of fire runs hex by hex between the two ships' centres.
use super::{Battle, Board, Hex};
use crate::combat::rng::CombatRng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Hit-chance penalty, in percentage points, against a ship in debris.
pub const COVER_PERCENT: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Asteroid,
    Debris,
}

impl Terrain {
    pub fn label(self) -> &'static str {
        match self {
            Terrain::Asteroid => "Asteroid",
            Terrain::Debris => "Debris field",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainHex {
    pub hex: Hex,
    pub kind: Terrain,
}

/// The terrain a battle on `board` with `seed` gets, sorted by hex.
pub fn generate(board: Board, seed: u64) -> Vec<TerrainHex> {
    // Its own stream, so terrain never shifts initiative or shots.
    let mut rng = CombatRng::new(seed ^ 0x7e11_a5ad_0b57_ac1e);
    let inner = board.radius - 2;
    // One half of the middle columns; the centre hex stays clear.
    let mut free: Vec<Hex> = board
        .hexes()
        .filter(|hex| hex.q.abs() <= inner && (hex.q > 0 || (hex.q == 0 && hex.r > 0)))
        .collect();
    let per_kind = usize::from(board.radius as u8 / 2);
    let mut terrain = Vec::new();
    for kind in [Terrain::Asteroid, Terrain::Debris] {
        for _ in 0..per_kind {
            if free.is_empty() {
                break;
            }
            let hex = free.swap_remove((rng.next_u64() % free.len() as u64) as usize);
            terrain.push(TerrainHex { hex, kind });
            terrain.push(TerrainHex {
                hex: Hex::new(-hex.q, -hex.r),
                kind,
            });
        }
    }
    terrain.sort_by_key(|cell| (cell.hex.q, cell.hex.r));
    terrain
}

/// Hexes on the straight line from `a` to `b`, both excluded, nudged off
/// hex corners consistently so the line is the same whichever end draws it
/// from the left.
fn line_between(a: Hex, b: Hex) -> Vec<Hex> {
    let (a, b) = if (a.q, a.r) <= (b.q, b.r) {
        (a, b)
    } else {
        (b, a)
    };
    let steps = a.distance(b);
    let cube = |hex: Hex| {
        let (q, r) = (f64::from(hex.q), f64::from(hex.r));
        (q, r, -q - r)
    };
    let (aq, ar, as_) = cube(a);
    let (bq, br, bs) = cube(b);
    (1..steps)
        .map(|step| {
            let t = f64::from(step) / f64::from(steps);
            let q = aq + (bq - aq) * t + 1e-6;
            let r = ar + (br - ar) * t + 2e-6;
            let s = as_ + (bs - as_) * t - 3e-6;
            let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
            let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
            if dq > dr && dq > ds {
                rq = -rr - rs;
            } else if dr > ds {
                rr = -rq - rs;
            }
            Hex::new(rq as i8, rr as i8)
        })
        .collect()
}

impl Battle {
    fn terrain_at(&self, hex: Hex) -> Option<Terrain> {
        self.terrain
            .iter()
            .find(|cell| cell.hex == hex)
            .map(|cell| cell.kind)
    }

    /// Whether `hex` is an asteroid.
    pub fn blocked(&self, hex: Hex) -> bool {
        self.terrain_at(hex) == Some(Terrain::Asteroid)
    }

    /// Whether a ship on `hex` has cover (debris).
    pub fn cover(&self, hex: Hex) -> bool {
        self.terrain_at(hex) == Some(Terrain::Debris)
    }

    /// Whether no asteroid stands between `from` and `to`.
    pub fn line_of_fire(&self, from: Hex, to: Hex) -> bool {
        !line_between(from, to)
            .into_iter()
            .any(|hex| self.blocked(hex))
    }

    /// Steps along the shortest path from `from` to every hex it can reach
    /// around asteroids, up to `limit` steps (ships can be passed).
    pub fn steps_from(&self, from: Hex, limit: u32) -> HashMap<Hex, u32> {
        let mut steps = HashMap::from([(from, 0)]);
        let mut queue = VecDeque::from([from]);
        while let Some(hex) = queue.pop_front() {
            let next = steps[&hex] + 1;
            if next > limit {
                continue;
            }
            for (dq, dr) in [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)] {
                let near = Hex::new(hex.q + dq, hex.r + dr);
                if self.board.contains(near) && !self.blocked(near) && !steps.contains_key(&near) {
                    steps.insert(near, next);
                    queue.push_back(near);
                }
            }
        }
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_is_mirrored_and_keeps_edges_and_deployment_clear() {
        for seed in 0..50 {
            for radius in [4, 5, 6] {
                let board = Board { radius };
                let terrain = generate(board, seed);
                assert_eq!(terrain.len(), 4 * usize::from(radius as u8 / 2));
                for cell in &terrain {
                    assert!(board.contains(cell.hex));
                    assert!(cell.hex.q.abs() <= radius - 2);
                    let mirror = Hex::new(-cell.hex.q, -cell.hex.r);
                    assert!(terrain.contains(&TerrainHex {
                        hex: mirror,
                        kind: cell.kind
                    }));
                }
                assert_eq!(terrain, generate(board, seed), "deterministic");
            }
        }
    }

    #[test]
    fn lines_pass_through_the_hexes_between() {
        let line = line_between(Hex::new(-2, 0), Hex::new(2, 0));
        assert_eq!(line, vec![Hex::new(-1, 0), Hex::new(0, 0), Hex::new(1, 0)]);
        assert!(line_between(Hex::new(0, 0), Hex::new(1, 0)).is_empty());
        // The same hexes whichever end fires.
        let a = line_between(Hex::new(-3, 1), Hex::new(2, -2));
        let b = line_between(Hex::new(2, -2), Hex::new(-3, 1));
        assert_eq!(a, b);
        assert_eq!(a.len(), 4);
    }
}
