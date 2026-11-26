//! Band geometry: positions relative to a star, distances and one run's
//! movement around bodies. Every value is in kilometres on the system plane.
use super::raiders::{BAND_KM_PER_RUN, NPC_STRIKE_KM};
use crate::{
    LocalPosition, StarSystem,
    movement::{SpacePosition, safe_route},
};

/// `at` relative to `world`'s star, if it lies inside the system (within
/// `half_extent_km` on every axis).
pub(crate) fn local_in(
    world: &StarSystem,
    at: SpacePosition,
    half_extent_km: i64,
) -> Option<LocalPosition> {
    let centre = SpacePosition::from_galaxy(world.position).ok()?;
    Some(LocalPosition {
        x_km: at.x_km.checked_sub(centre.x_km)?,
        y_km: at.y_km.checked_sub(centre.y_km)?,
        z_km: at.z_km.checked_sub(centre.z_km)?,
    })
    .filter(|local| {
        [local.x_km, local.y_km, local.z_km]
            .iter()
            .all(|value| value.abs() <= half_extent_km)
    })
}

pub(crate) fn distance(a: LocalPosition, b: LocalPosition) -> i64 {
    let squares: i128 = [(a.x_km, b.x_km), (a.y_km, b.y_km), (a.z_km, b.z_km)]
        .iter()
        .map(|(a, b)| (i128::from(*a) - i128::from(*b)).pow(2))
        .sum();
    i64::try_from((squares as u128).isqrt()).unwrap_or(i64::MAX)
}

pub(crate) fn near(a: LocalPosition, b: LocalPosition) -> bool {
    distance(a, b) <= NPC_STRIKE_KM
}

/// Up to one run's distance toward `goal`, along a route that avoids the star
/// and every body (a band that cannot find one holds still). On the plane,
/// orbiting bodies often cross the straight line, so the band follows the
/// detour's waypoints for as far as one run takes it.
pub(crate) fn step_toward(
    rules: &crate::movement::MovementRules,
    world: &StarSystem,
    from: LocalPosition,
    goal: LocalPosition,
) -> LocalPosition {
    let Ok(path) = safe_route(rules, world, from, goal) else {
        return from;
    };
    let mut at = from;
    let mut budget = BAND_KM_PER_RUN;
    for next in path.into_iter().chain(std::iter::once(goal)) {
        let gap = distance(at, next);
        if gap <= budget {
            budget -= gap;
            at = next;
            continue;
        }
        let scale = |a: i64, b: i64| {
            a + i64::try_from(i128::from(b - a) * i128::from(budget) / i128::from(gap)).unwrap_or(0)
        };
        return LocalPosition {
            x_km: scale(at.x_km, next.x_km),
            y_km: scale(at.y_km, next.y_km),
            z_km: scale(at.z_km, next.z_km),
        };
    }
    at
}
