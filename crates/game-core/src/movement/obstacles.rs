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

/// Save/world check that holds at every tick, orbits included: each body's
/// volume stays inside the cube and clear of the star and of every other body.
///
/// Bodies circling the star sweep rings around the vertical axis; a planet's
/// moons ride inside its ring (its extent grows by their distance and size).
/// Two rings never meet when, in the radius/height plane, their centres are
/// farther apart than their extents. Bodies without an orbit count as rings
/// too, which is conservative.
pub(super) fn validate_volumes(
    rules: &MovementRules,
    system: &StarSystem,
) -> Result<(), MovementError> {
    if system.bodies.iter().all(|body| body.orbit.is_none()) {
        return validate_fixed_volumes(rules, system);
    }
    let invalid = || MovementError::InvalidRules;
    let radius = |kind| i128::from(rules.body_radii.of(kind));
    let parent_of = |body: &crate::LocalBody| body.orbit.as_ref().and_then(|orbit| orbit.parent);
    // Check untrusted orbit data before computing ring geometry or placing it.
    // In particular, a zero period must never reach the rotation divisor.
    for body in &system.bodies {
        if let Some(orbit) = &body.orbit
            && (orbit.period_ticks == 0
                || geometry::local(orbit.epoch)
                    .iter()
                    .any(|value| value.unsigned_abs() > rules.system_half_extent_km as u64))
        {
            return Err(invalid());
        }
    }
    // (horizontal radius, height, extent) per body circling the star.
    let mut rings = Vec::new();
    for body in system
        .bodies
        .iter()
        .filter(|body| parent_of(body).is_none())
    {
        let offset = body
            .orbit
            .as_ref()
            .map_or(body.position, |orbit| orbit.epoch);
        let mut extent = radius(body.kind);
        let moons = system
            .bodies
            .iter()
            .filter(|moon| parent_of(moon) == Some(body.id));
        for moon in moons {
            let epoch = moon.orbit.as_ref().ok_or_else(invalid)?.epoch;
            let reach = length(epoch);
            if reach <= radius(body.kind) + radius(moon.kind) {
                return Err(invalid());
            }
            extent = extent.max(reach + radius(moon.kind));
        }
        rings.push((
            i128::from(crate::orbits::horizontal_radius(offset)),
            i128::from(offset.y_km),
            extent,
        ));
    }
    for body in &system.bodies {
        if let Some(orbit) = &body.orbit {
            let parent = orbit
                .parent
                .map(|id| system.bodies.iter().find(|known| known.id == id));
            let nested = matches!(parent, Some(Some(parent)) if parent_of(parent).is_some());
            if orbit.period_ticks == 0 || matches!(parent, Some(None)) || nested {
                return Err(invalid());
            }
        }
    }
    let half = i128::from(rules.system_half_extent_km);
    let star = i128::from(rules.star_exclusion_radius_km);
    for (index, &(horizontal, height, extent)) in rings.iter().enumerate() {
        if horizontal + extent > half
            || height.abs() + extent > half
            || horizontal * horizontal + height * height <= (star + extent).pow(2)
        {
            return Err(invalid());
        }
        for &(other_horizontal, other_height, other_extent) in &rings[..index] {
            let apart = (horizontal - other_horizontal).pow(2) + (height - other_height).pow(2);
            if apart <= (extent + other_extent).pow(2) {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

/// Legacy worlds have stationary bodies: check their actual spheres rather
/// than treating them as swept rings.
fn validate_fixed_volumes(rules: &MovementRules, system: &StarSystem) -> Result<(), MovementError> {
    let volumes = obstacles(rules, system);
    for (index, volume) in volumes.iter().enumerate() {
        let reach = u64::try_from(volume.radius).map_err(|_| MovementError::InvalidRules)?;
        let coordinates = geometry::local(volume.centre);
        if coordinates
            .iter()
            .any(|value| value.unsigned_abs() + reach > rules.system_half_extent_km as u64)
        {
            return Err(MovementError::InvalidRules);
        }
        for other in &volumes[..index] {
            let apart = geometry::distance_squared(coordinates, geometry::local(other.centre))?;
            let touching = u128::try_from(volume.radius + other.radius)
                .map_err(|_| MovementError::InvalidRules)?
                .pow(2);
            if apart <= touching {
                return Err(MovementError::InvalidRules);
            }
        }
    }
    Ok(())
}

fn length(offset: LocalPosition) -> i128 {
    let squares: i128 = geometry::local(offset)
        .iter()
        .map(|value| i128::from(*value).pow(2))
        .sum();
    (squares as u128).isqrt() as i128
}
