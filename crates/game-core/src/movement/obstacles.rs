//! Collision volumes inside a system: the star's exclusion sphere and a
//! map-scale sphere around every body. A local segment may not pass through
//! any of them, except a body it starts or ends at (leaving it, or arriving).
//!
//! Radii are map-scale, not physical: a real rocky planet (~6,400 km) would be
//! invisible in a system 400 million km across, so bodies are sized to match
//! how the map draws them and to make courses route around them.
use super::{MovementError, MovementRules, geometry};
use crate::{BodyKind, LocalPosition, StarSystem};
use serde::{Deserialize, Serialize};

/// Collision radius per body kind, in km.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BodyRadii {
    pub rocky_planet_km: i64,
    pub gas_giant_km: i64,
    pub moon_km: i64,
    pub asteroid_km: i64,
    pub station_km: i64,
}

impl Default for BodyRadii {
    fn default() -> Self {
        Self {
            rocky_planet_km: 600_000,
            gas_giant_km: 2_000_000,
            moon_km: 250_000,
            asteroid_km: 150_000,
            station_km: 50_000,
        }
    }
}

impl BodyRadii {
    pub fn of(&self, kind: BodyKind) -> i64 {
        match kind {
            BodyKind::RockyPlanet => self.rocky_planet_km,
            BodyKind::GasGiant => self.gas_giant_km,
            BodyKind::Moon => self.moon_km,
            BodyKind::Asteroid => self.asteroid_km,
            BodyKind::Station => self.station_km,
        }
    }

    /// Every radius is positive and well inside the system cube.
    pub(super) fn valid(&self, half_extent_km: i64) -> bool {
        [
            self.rocky_planet_km,
            self.gas_giant_km,
            self.moon_km,
            self.asteroid_km,
            self.station_km,
        ]
        .iter()
        .all(|radius| (1..half_extent_km / 4).contains(radius))
    }
}

/// A sphere courses must avoid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Obstacle {
    pub centre: LocalPosition,
    pub radius: i64,
    /// The star never lets a segment through, even one that "ends" at its centre.
    pub star: bool,
}

impl Obstacle {
    pub fn error(self) -> MovementError {
        if self.star {
            MovementError::StarCollision
        } else {
            MovementError::BodyCollision
        }
    }
}

/// The star and every body of `system`.
pub(super) fn obstacles(rules: &MovementRules, system: &StarSystem) -> Vec<Obstacle> {
    let star = Obstacle {
        centre: LocalPosition {
            x_km: 0,
            y_km: 0,
            z_km: 0,
        },
        radius: rules.star_exclusion_radius_km,
        star: true,
    };
    std::iter::once(star)
        .chain(system.bodies.iter().map(|body| Obstacle {
            centre: body.position,
            radius: rules.body_radii.of(body.kind),
            star: false,
        }))
        .collect()
}

/// The first obstacle the segment passes through, ignoring a body it starts or
/// ends at.
pub(super) fn blocking(
    start: LocalPosition,
    end: LocalPosition,
    obstacles: &[Obstacle],
) -> Option<Obstacle> {
    obstacles
        .iter()
        .find(|obstacle| {
            (obstacle.star || (obstacle.centre != start && obstacle.centre != end))
                && geometry::crosses_sphere(start, end, obstacle.centre, obstacle.radius)
        })
        .copied()
}

/// The obstacle containing `point`, if any (its own centre excepted for bodies).
pub(super) fn containing(point: LocalPosition, obstacles: &[Obstacle]) -> Option<Obstacle> {
    obstacles
        .iter()
        .find(|obstacle| {
            (obstacle.star || obstacle.centre != point)
                && geometry::crosses_sphere(point, point, obstacle.centre, obstacle.radius)
        })
        .copied()
}
