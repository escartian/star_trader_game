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
                position: band.position,
            };
            Some(Sighting {
                now,
                steer: (band.system, target),
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
                (Some(bound), _) => bound,
                (None, Some((system, at))) => (system, berth(&ship.location, system, at, systems)?),
                (None, None) => return None,
            };
            Some(Sighting { now, steer })
        }
    }
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
