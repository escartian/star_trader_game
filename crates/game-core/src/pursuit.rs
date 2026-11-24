//! Pursuit: the flagship chases an NPC trader or a raider band. Each run,
//! after everything has moved, a flagship in the quarry's system and within
//! reach opens the contact (a hail for a trader, a raider fight for a band)
//! and the pursuit ends; otherwise its course is re-plotted to where the
//! quarry is now, or, when the quarry is in or bound for another system, to
//! that system. A quarry that is gone, or out of range or fuel, ends it, and
//! so does any other order for the flagship. Positions are kilometres.
use crate::{
    BodyId, LocalPosition, StarSystem, SystemId,
    economy::Economy,
    fleet::{FleetId, FleetMovementError},
    movement::{MoveTarget, MovementError, SpacePosition},
    navigation::{NavigationError, NavigationEvent, band_motion::distance},
    piracy,
    travel::{Location, PlayerId, ShipId, TravelState},
};
use serde::{Deserialize, Serialize};

/// What the flagship chases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quarry {
    /// An NPC trader (`npc::Trader::id`).
    Trader(u64),
    /// A raider band (`navigation::RaiderBand::id`).
    Band(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pursuit {
    pub fleet: FleetId,
    pub quarry: Quarry,
}

/// How close (km) the flagship must come to a raider band to engage it on
/// purpose: hailing range, wider than a band's own strike.
pub const ENGAGE_KM: i64 = piracy::HAIL_KM;

/// Where the quarry is now.
enum Whereabouts {
    /// In a star system: where to steer for it and where it is (kilometres
    /// from the star).
    In {
        system: SystemId,
        target: MoveTarget,
        at: LocalPosition,
    },
    /// Between systems (or crossing), bound for `system` (`body` when known).
    Bound {
        system: SystemId,
        body: Option<BodyId>,
    },
}

/// `None` once the quarry is gone, or adrift with nowhere to go.
fn whereabouts(
    state: &TravelState,
    economy: &Economy,
    quarry: Quarry,
    systems: &[StarSystem],
) -> Option<Whereabouts> {
    match quarry {
        Quarry::Band(id) => {
            let band = state.navigation.bands.iter().find(|band| band.id == id)?;
            if band.transit.is_some() {
                return Some(Whereabouts::Bound {
                    system: band.system,
                    body: None,
                });
            }
            let target = MoveTarget::Local {
                system: band.system,
                position: band.position,
            };
            Some(Whereabouts::In {
                system: band.system,
                target,
                at: band.position,
            })
        }
        Quarry::Trader(id) => {
            let npc = economy.traders.iter().find(|npc| npc.id == id)?;
            let ship = npc.state.ships.first()?;
            let Some(system) = ship.location.system() else {
                // Between systems: follow it to where it is bound.
                let order = npc.state.navigation.orders.first()?;
                let last = order.remaining_waypoints.last().unwrap_or(&order.target);
                return match *last {
                    MoveTarget::Body { system, body } => Some(Whereabouts::Bound {
                        system,
                        body: Some(body),
                    }),
                    MoveTarget::Local { system, .. } | MoveTarget::EnterSystem(system) => {
                        Some(Whereabouts::Bound { system, body: None })
                    }
                    _ => None,
                };
            };
            let at = local(
                npc.state.navigation_position(ship.id, systems)?,
                system,
                systems,
            )?;
            // A trader at a body is followed to that body; in open space, to its spot.
            let target = match ship.location {
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
            };
            Some(Whereabouts::In { system, target, at })
        }
    }
}

fn local(at: SpacePosition, system: SystemId, systems: &[StarSystem]) -> Option<LocalPosition> {
    let world = systems.iter().find(|known| known.id == system)?;
    let centre = SpacePosition::from_galaxy(world.position).ok()?;
    Some(LocalPosition {
        x_km: at.x_km.checked_sub(centre.x_km)?,
        y_km: at.y_km.checked_sub(centre.y_km)?,
        z_km: at.z_km.checked_sub(centre.z_km)?,
    })
}

/// The flagship's lead ship, and its system and position when in a system.
fn flagship_at(
    state: &TravelState,
    fleet: FleetId,
    systems: &[StarSystem],
) -> Option<(ShipId, Option<(SystemId, LocalPosition)>)> {
    let record = state.fleets.iter().find(|record| record.id == fleet)?;
    let lead = state.ship(*record.ships.first()?).ok()?;
    let here = lead.location.system().and_then(|system| {
        let at = local(
            state.navigation_position(lead.id, systems)?,
            system,
            systems,
        )?;
        Some((system, at))
    });
    Some((lead.id, here))
}

/// Orders the flagship to pursue `quarry`, in its system or beyond. Opens
/// the contact at once when already in reach; otherwise plots the first course.
pub fn pursue(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    quarry: Quarry,
    systems: &[StarSystem],
) -> Result<NavigationEvent, FleetMovementError> {
    if state.flagship(player) != Some(fleet) {
        return Err(NavigationError::NotFlagship.into());
    }
    if state.navigation.encounters.iter().any(|c| c.fleet == fleet) {
        return Err(NavigationError::EncounterPending(fleet).into());
    }
    if state
        .navigation
        .order(fleet)
        .is_some_and(|order| order.started)
    {
        return Err(NavigationError::Underway.into());
    }
    let missing = match quarry {
        Quarry::Trader(_) => NavigationError::UnknownTrader,
        Quarry::Band(_) => NavigationError::UnknownBand,
    };
    whereabouts(state, economy, quarry, systems).ok_or(missing)?;
    state.end_trade_route(fleet);
    state.end_patrol(fleet);
    state.navigation.holds.retain(|held| *held != fleet);
    state.navigation.pursuit = Some(Pursuit { fleet, quarry });
    let events = service_pursuit(state, economy, player, systems)?;
    let ordered =
        state.navigation.order(fleet).is_some() || state.navigation.holds.contains(&fleet);
    if state.navigation.pursuit.is_some() && !ordered {
        state.navigation.pursuit = None;
        return Err(NavigationError::NoCourse.into());
    }
    if let Some(NavigationEvent::PursuitNote { note, .. }) = events.first() {
        // Ended before it began (out of range, say): refuse with the reason.
        let reason = note.trim_start_matches("Pursuit ended: ");
        return Err(NavigationError::PursuitImpossible(reason.to_owned()).into());
    }
    Ok(events
        .into_iter()
        .next()
        .unwrap_or(NavigationEvent::CoursePlotted(fleet)))
}

/// Ends the pursuit when `fleet` was the pursuer (it got other orders).
pub fn end_pursuit(state: &mut TravelState, fleet: FleetId) -> bool {
    let ends = state
        .navigation
        .pursuit
        .is_some_and(|pursuit| pursuit.fleet == fleet);
    if ends {
        state.navigation.pursuit = None;
    }
    ends
}

/// After a run: catch the quarry, lose it, or steer for where it is now (in
/// its system, or the system it is bound for).
pub fn service_pursuit(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let Some(Pursuit { fleet, quarry }) = state.navigation.pursuit else {
        return Ok(Vec::new());
    };
    let note = |note: &str| NavigationEvent::PursuitNote {
        fleet,
        note: note.into(),
    };
    if state.flagship(player) != Some(fleet) {
        state.navigation.pursuit = None;
        return Ok(Vec::new());
    }
    // A contact (perhaps the quarry striking first) settles the chase.
    if state.navigation.encounters.iter().any(|c| c.fleet == fleet) {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: contact made.")]);
    }
    let found = whereabouts(state, economy, quarry, systems);
    let ours = flagship_at(state, fleet, systems);
    let (Some(found), Some((lead, here))) = (found, ours) else {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: the quarry got away.")]);
    };
    if let (Whereabouts::In { system, at, .. }, Some((ours, position))) = (&found, here)
        && *system == ours
        && let Some(contact) = catch(
            state,
            economy,
            player,
            fleet,
            quarry,
            (position, *at),
            systems,
        )?
    {
        state.navigation.pursuit = None;
        // Stop where the contact happened.
        if state
            .navigation
            .order(fleet)
            .is_some_and(|order| !order.started)
        {
            state.cancel_course(player, fleet)?;
        }
        return Ok(vec![contact]);
    }
    if state
        .navigation
        .order(fleet)
        .is_some_and(|order| order.started)
    {
        // Still on the last leg; steer again once it ends.
        return Ok(Vec::new());
    }
    let plotted = match found {
        Whereabouts::In { system, target, .. } if here.is_some_and(|(ours, _)| ours == system) => {
            state.plot_course(player, lead, target, systems).map(|_| ())
        }
        Whereabouts::In { system, target, .. } => {
            let body = match target {
                MoveTarget::Body { body, .. } => Some(body),
                _ => None,
            };
            follow(state, player, lead, fleet, system, body, systems)
        }
        // Already where it is bound: wait for it to arrive.
        Whereabouts::Bound { system, .. } if here.is_some_and(|(ours, _)| ours == system) => {
            Err(MovementError::AlreadyThere.into())
        }
        Whereabouts::Bound { system, body } => {
            follow(state, player, lead, fleet, system, body, systems)
        }
    };
    match plotted {
        Ok(()) => Ok(Vec::new()),
        // Already there (a docked trader at the same port, say), or the
        // quarry is passing a body's collision zone: wait a run for it.
        Err(FleetMovementError::Movement(
            MovementError::AlreadyThere | MovementError::BodyCollision,
        )) => {
            state.navigation.holds.retain(|held| *held != fleet);
            state.navigation.holds.push(fleet);
            Ok(Vec::new())
        }
        Err(_) => {
            state.navigation.pursuit = None;
            Ok(vec![note(
                "Pursuit ended: no course to the quarry (out of range or fuel).",
            )])
        }
    }
}

/// The contact when the flagship is in reach of its quarry; `(ours, theirs)`
/// are their positions in the same system.
fn catch(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    quarry: Quarry,
    (position, at): (LocalPosition, LocalPosition),
    systems: &[StarSystem],
) -> Result<Option<NavigationEvent>, FleetMovementError> {
    Ok(match quarry {
        Quarry::Trader(id) => piracy::hail(state, economy, player, fleet, id, systems).ok(),
        Quarry::Band(id) if distance(position, at) <= ENGAGE_KM => {
            let band = state
                .navigation
                .bands
                .iter()
                .find(|band| band.id == id)
                .cloned()
                .ok_or(NavigationError::UnknownBand)?;
            Some(state.strike(&band, fleet, state.navigation.run)?)
        }
        Quarry::Band(_) => None,
    })
}

/// Plots (or keeps) a course to another system, to `body` when known.
fn follow(
    state: &mut TravelState,
    player: PlayerId,
    lead: ShipId,
    fleet: FleetId,
    system: SystemId,
    body: Option<BodyId>,
    systems: &[StarSystem],
) -> Result<(), FleetMovementError> {
    // A course already bound there is kept, not re-plotted every run.
    let bound = state.navigation.order(fleet).is_some_and(|order| {
        let last = order.remaining_waypoints.last().unwrap_or(&order.target);
        match last {
            MoveTarget::Body { system: to, .. }
            | MoveTarget::Local { system: to, .. }
            | MoveTarget::EnterSystem(to) => *to == system,
            _ => false,
        }
    });
    if bound {
        return Ok(());
    }
    state
        .plot_destination(player, lead, system, body, systems)
        .map(|_| ())
}
