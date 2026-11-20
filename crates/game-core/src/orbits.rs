//! Orbital motion. Bodies circle the star (or a moon its planet) on fixed
//! circular orbits around the vertical (y) axis, so a body's position is a pure
//! function of the world tick: nothing extra is saved and every machine computes
//! identical kilometres (integer fixed-point trigonometry, no floating point).
//!
//! A body with an [`Orbit`] keeps its tick-0 offset from what it circles
//! (`epoch`); [`place`] rotates every epoch to the given tick and writes the
//! result into `LocalBody::position`. Rules always read positions placed at the
//! current tick. Bodies without an orbit (older saves) stay where they are.
use crate::{BodyId, LocalBody, LocalPosition, StarSystem};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Orbit {
    /// The body circled; `None` for the star. Moons circle a body that itself
    /// circles the star (one level only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<BodyId>,
    /// World ticks per full revolution.
    pub period_ticks: u64,
    /// Offset from what is circled at tick 0, in km.
    pub epoch: LocalPosition,
}

/// Turn units per full revolution (angles are fractions of a turn).
const TURN: u64 = 1 << 32;
/// π/2 in Q60 fixed point.
const HALF_PI_Q60: i128 = 1_811_004_864_519_280_711;
const ONE_Q60: i128 = 1 << 60;

/// (cos, sin) of `turns / 2^32` of a revolution, in Q30 fixed point. Integer
/// Taylor series on a quarter turn: deterministic and accurate to ~1e-9.
fn cos_sin(turns: u64) -> (i64, i64) {
    let quadrant = (turns >> 30) & 3;
    let x = (i128::from(turns & ((1 << 30) - 1)) * HALF_PI_Q60) >> 30;
    let x2 = (x * x) >> 60;
    let (mut cos, mut sin) = (0_i128, 0_i128);
    let (mut cos_term, mut sin_term) = (ONE_Q60, x);
    for n in 0..10_i128 {
        cos += cos_term;
        sin += sin_term;
        cos_term = -((cos_term * x2) >> 60) / ((2 * n + 1) * (2 * n + 2));
        sin_term = -((sin_term * x2) >> 60) / ((2 * n + 2) * (2 * n + 3));
    }
    let q30 = |value: i128| ((value + (1 << 29)) >> 30) as i64;
    let (c, s) = (q30(cos), q30(sin));
    match quadrant {
        0 => (c, s),
        1 => (-s, c),
        2 => (-c, -s),
        _ => (s, -c),
    }
}

/// `offset` rotated about the vertical axis by the share of `period` that
/// `tick` reaches (rounded to the nearest km).
fn rotate(offset: LocalPosition, tick: u64, period: u64) -> LocalPosition {
    let turns = ((u128::from(tick % period) * u128::from(TURN)) / u128::from(period)) as u64;
    let (cos, sin) = cos_sin(turns);
    let (x, z) = (i128::from(offset.x_km), i128::from(offset.z_km));
    let (cos, sin) = (i128::from(cos), i128::from(sin));
    let round = |value: i128| ((value + (1 << 29)) >> 30) as i64;
    LocalPosition {
        x_km: round(x * cos - z * sin),
        y_km: offset.y_km,
        z_km: round(x * sin + z * cos),
    }
}

/// Moves every orbiting body of every system to where it is at `tick`.
pub fn place(systems: &mut [StarSystem], tick: u64) {
    for system in systems {
        place_system(system, tick);
    }
}

/// [`place`] on a copy.
pub fn placed(systems: &[StarSystem], tick: u64) -> Vec<StarSystem> {
    let mut systems = systems.to_vec();
    place(&mut systems, tick);
    systems
}

fn place_system(system: &mut StarSystem, tick: u64) {
    // Bodies circling the star first, then moons around their (placed) parents.
    for body in &mut system.bodies {
        if let Some(orbit) = body.orbit.as_ref().filter(|orbit| orbit.parent.is_none()) {
            body.position = rotate(orbit.epoch, tick, orbit.period_ticks);
        }
    }
    let anchors: Vec<(BodyId, LocalPosition)> = system
        .bodies
        .iter()
        .map(|body| (body.id, body.position))
        .collect();
    for body in &mut system.bodies {
        let Some(orbit) = &body.orbit else { continue };
        let Some(parent) = orbit.parent else { continue };
        if let Some((_, centre)) = anchors.iter().find(|(id, _)| *id == parent) {
            let offset = rotate(orbit.epoch, tick, orbit.period_ticks);
            body.position = LocalPosition {
                x_km: centre.x_km + offset.x_km,
                y_km: centre.y_km + offset.y_km,
                z_km: centre.z_km + offset.z_km,
            };
        }
    }
}

/// Reference period: a body circling the star `REFERENCE_RADIUS_KM` out takes
/// this many ticks (100 navigation runs of 4 ticks) per revolution.
pub const REFERENCE_PERIOD_TICKS: u64 = 400;
pub const REFERENCE_RADIUS_KM: u64 = 50_000_000;
/// A moon's revolution around its planet (12 navigation runs).
pub const MOON_PERIOD_TICKS: u64 = 48;

/// Kepler-like period for a body circling the star at `offset`: proportional
/// to (horizontal radius)^1.5, computed with integer square roots.
pub fn stellar_period(offset: LocalPosition) -> u64 {
    let radius = horizontal_radius(offset).max(1);
    let scaled = |value: u64| u128::from(value) * u128::from(value.isqrt());
    let period = u128::from(REFERENCE_PERIOD_TICKS) * scaled(radius) / scaled(REFERENCE_RADIUS_KM);
    period.clamp(8, 1_000_000) as u64
}

/// Distance from the vertical axis through what is circled, in km.
pub(crate) fn horizontal_radius(offset: LocalPosition) -> u64 {
    let x = i128::from(offset.x_km);
    let z = i128::from(offset.z_km);
    ((x * x + z * z) as u128).isqrt() as u64
}

/// Gives every body of `bodies` an orbit from its current position: around the
/// star, or around `parents` (moon → planet) where listed.
pub fn assign_orbits(bodies: &mut [LocalBody], parents: &[(BodyId, BodyId)]) {
    let positions: Vec<(BodyId, LocalPosition)> =
        bodies.iter().map(|body| (body.id, body.position)).collect();
    for body in bodies {
        let parent = parents
            .iter()
            .find(|(moon, _)| *moon == body.id)
            .and_then(|(_, parent)| positions.iter().find(|(id, _)| id == parent));
        body.orbit = Some(match parent {
            Some((parent, centre)) => Orbit {
                parent: Some(*parent),
                period_ticks: MOON_PERIOD_TICKS,
                epoch: LocalPosition {
                    x_km: body.position.x_km - centre.x_km,
                    y_km: body.position.y_km - centre.y_km,
                    z_km: body.position.z_km - centre.z_km,
                },
            },
            None => Orbit {
                parent: None,
                period_ticks: stellar_period(body.position),
                epoch: body.position,
            },
        });
    }
}

#[cfg(test)]
mod tests;
