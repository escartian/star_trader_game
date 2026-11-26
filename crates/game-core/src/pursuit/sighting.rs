//! Where a pursued quarry is now (for the catch) and where to steer for it:
//! a trader's destination when it has one, so the flagship intercepts it
//! there instead of trailing it at the same speed; otherwise its berth or
//! position. Positions are kilometres from the star.
use super::Quarry;
use crate::{
    LocalPosition, StarSystem, SystemId,
    economy::Economy,
    movement::{MoveTarget, SpacePosition},
    travel::{Location, TravelState},
};

pub(super) struct Sighting {
    /// Its system and position now, when it is in a system.
    pub now: Option<(SystemId, LocalPosition)>,
    /// Where to steer: a course target in that system (`EnterSystem` when
    /// only the system is known).
    pub steer: (SystemId, MoveTarget),
}

/// `None` once the quarry is gone, or adrift with nowhere to go.
pub(super) fn sight(
    state: &TravelState,
    economy: &Economy,
    quarry: Quarry,
    systems: &[StarSystem],
) -> Option<Sighting> {
    match quarry {
        Quarry::Band(id) => {
            let band = state.navigation.bands.iter().find(|band| band.id == id)?;
            // A crossing band's position is where it will arrive.
            let now = band
                .transit
                .is_none()
                .then_some((band.system, band.position));
            let target = MoveTarget::Local {
                system: band.system,
                position: next_step(state, systems, band.system, band.position, band.heading),
            };
            Some(Sighting {
                now,
                steer: (band.system, target),
            })
        }
        Quarry::Patrol(id) => {
            // Patrols never leave their system: steer for where it will be.
            let patrol = state.navigation.police.patrol(id)?;
            // A patrol on its rounds turns for its next post on arrival.
            let heading = match (patrol.pursuing, patrol.answering) {
                (None, None) => systems
                    .iter()
                    .find(|known| known.id == patrol.system)
                    .map_or(patrol.heading, |world| {
                        crate::police::rounds(patrol, world, &state.movement_rules).1
                    }),
                _ => patrol.heading,
            };
            let target = MoveTarget::Local {
                system: patrol.system,
                position: next_step(state, systems, patrol.system, patrol.position, heading),
            };
            Some(Sighting {
                now: Some((patrol.system, patrol.position)),
                steer: (patrol.system, target),
            })
        }
        Quarry::Trader(id) => {
            let npc = economy.traders.iter().find(|npc| npc.id == id)?;
            let ship = npc.state.ships.first()?;
            let now = ship.location.system().and_then(|system| {
                let at = local(
                    npc.state.navigation_position(ship.id, systems)?,
                    system,
                    systems,
                )?;
                Some((system, at))
            });
            let bound = npc.state.navigation.orders.first().and_then(|order| {
                let last = order.remaining_waypoints.last().unwrap_or(&order.target);
                match last {
                    MoveTarget::Body { system, .. } | MoveTarget::Local { system, .. } => {
                        Some((*system, last.clone()))
                    }
                    MoveTarget::EnterSystem(system) => {
                        Some((*system, MoveTarget::EnterSystem(*system)))
                    }
                    MoveTarget::LeaveSystem(_) | MoveTarget::DeepSpace(_) => None,
                }
            });
            let steer = match (bound, now) {
                (Some(bound), _) => off_station(bound, now, systems),
                (None, Some((system, at))) => {
                    // Docked: wait off the port for it to come out.
                    off_station(
                        (system, berth(&ship.location, system, at, systems)?),
                        None,
                        systems,
                    )
                }
                (None, None) => return None,
            };
            Some(Sighting { now, steer })
        }
    }
}

/// Where a band or patrol at `from`, heading for `heading`, will be after
/// its next run's step (see `navigation::band_motion::step_toward`). Steering
/// there, instead of where it is now, meets it rather than trailing it.
fn next_step(
    state: &TravelState,
    systems: &[StarSystem],
    system: SystemId,
    from: LocalPosition,
    heading: LocalPosition,
) -> LocalPosition {
    systems
        .iter()
        .find(|known| known.id == system)
        .map_or(from, |world| {
            crate::navigation::band_motion::step_toward(&state.movement_rules, world, from, heading)
        })
}

/// How far off a station (km) the flagship waits for a trader bound there:
/// inside hailing range, outside the station's own berth, so the trader is
/// caught on its way in rather than under the station's guns.
const STATION_STANDOFF_KM: i64 = crate::piracy::HAIL_KM * 2 / 5;

/// A trader bound for a station is met just off it, on the side it comes
/// from, instead of docking there too.
fn off_station(
    bound: (SystemId, MoveTarget),
    now: Option<(SystemId, LocalPosition)>,
    systems: &[StarSystem],
) -> (SystemId, MoveTarget) {
    let (system, MoveTarget::Body { body, .. }) = bound else {
        return bound;
    };
    let Some(station) = systems
        .iter()
        .find(|known| known.id == system)
        .and_then(|known| known.bodies.iter().find(|known| known.id == body))
        .filter(|known| known.kind == crate::BodyKind::Station)
    else {
        return bound;
    };
    let at = station.position;
    // Toward the trader when it is in the system, else toward the star.
    let from = match now {
        Some((here, position)) if here == system => position,
        _ => LocalPosition {
            x_km: 0,
            y_km: 0,
            z_km: 0,
        },
    };
    let delta = [
        (from.x_km - at.x_km) as f64,
        (from.y_km - at.y_km) as f64,
        (from.z_km - at.z_km) as f64,
    ];
    let length = delta.iter().map(|value| value * value).sum::<f64>().sqrt();
    let unit = if length > 1.0 {
        delta.map(|value| value / length)
    } else {
        [1.0, 0.0, 0.0]
    };
    let step = |value: f64| (value * STATION_STANDOFF_KM as f64).round() as i64;
    let position = LocalPosition {
        x_km: at.x_km + step(unit[0]),
        y_km: at.y_km + step(unit[1]),
        z_km: at.z_km + step(unit[2]),
    };
    (system, MoveTarget::Local { system, position })
}

/// A resting trader is followed to its body (the port when docked), one in
/// open space to its spot.
fn berth(
    location: &Location,
    system: SystemId,
    at: LocalPosition,
    systems: &[StarSystem],
) -> Option<MoveTarget> {
    Some(match *location {
        Location::Orbiting { body, .. } | Location::Landed { body, .. } => {
            MoveTarget::Body { system, body }
        }
        Location::Docked(_) => MoveTarget::Body {
            system,
            body: systems
                .iter()
                .find(|known| known.id == system)?
                .bodies
                .iter()
                .find(|body| body.kind == crate::BodyKind::Station)?
                .id,
        },
        _ => MoveTarget::Local {
            system,
            position: at,
        },
    })
}

pub(super) fn local(
    at: SpacePosition,
    system: SystemId,
    systems: &[StarSystem],
) -> Option<LocalPosition> {
    let world = systems.iter().find(|known| known.id == system)?;
    let centre = SpacePosition::from_galaxy(world.position).ok()?;
    Some(LocalPosition {
        x_km: at.x_km.checked_sub(centre.x_km)?,
        y_km: at.y_km.checked_sub(centre.y_km)?,
        z_km: at.z_km.checked_sub(centre.z_km)?,
    })
}
