//! Pursuit: the flagship chases an NPC trader or a raider band in its own
//! star system. Each run, after everything has moved, a flagship within reach
//! of its quarry opens the contact (a hail for a trader, a raider fight for a
//! band) and the pursuit ends; otherwise its course is re-plotted to where the
//! quarry is now. A quarry that leaves the system or is gone ends it, and so
//! does any other order for the flagship. Positions are kilometres.
use crate::{
    LocalPosition, StarSystem, SystemId,
    economy::Economy,
    fleet::{FleetId, FleetMovementError},
    movement::{MoveTarget, SpacePosition},
    navigation::{NavigationError, NavigationEvent, band_motion::distance},
    piracy,
    travel::{Location, PlayerId, TravelState},
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

/// Where the quarry is now: its system, where to steer for it, and its
/// position (kilometres from the star). `None` once it has gone or is
/// between systems.
fn whereabouts(
    state: &TravelState,
    economy: &Economy,
    quarry: Quarry,
    systems: &[StarSystem],
) -> Option<(SystemId, MoveTarget, LocalPosition)> {
    match quarry {
        Quarry::Band(id) => {
            let band = state.navigation.bands.iter().find(|band| band.id == id)?;
            if band.transit.is_some() {
                return None;
            }
            let target = MoveTarget::Local {
                system: band.system,
                position: band.position,
            };
            Some((band.system, target, band.position))
        }
        Quarry::Trader(id) => {
            let npc = economy.traders.iter().find(|npc| npc.id == id)?;
            let ship = npc.state.ships.first()?;
            let system = ship.location.system()?;
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
            Some((system, target, at))
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

/// The flagship's lead ship, its system and position, when it is in a system.
fn flagship_at(
    state: &TravelState,
    fleet: FleetId,
    systems: &[StarSystem],
) -> Option<(crate::travel::ShipId, SystemId, LocalPosition)> {
    let record = state.fleets.iter().find(|record| record.id == fleet)?;
    let lead = state.ship(*record.ships.first()?).ok()?;
    let system = lead.location.system()?;
    let at = local(
        state.navigation_position(lead.id, systems)?,
        system,
        systems,
    )?;
    Some((lead.id, system, at))
}

/// Orders the flagship to pursue `quarry` in its own system. Opens the
/// contact at once when already in reach; otherwise plots the first course.
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
    let missing = match quarry {
        Quarry::Trader(_) => NavigationError::UnknownTrader,
        Quarry::Band(_) => NavigationError::UnknownBand,
    };
    let (system, ..) = whereabouts(state, economy, quarry, systems).ok_or(missing)?;
    let (_, here, _) =
        flagship_at(state, fleet, systems).ok_or(NavigationError::PursuitOutOfSystem)?;
    if here != system {
        return Err(NavigationError::PursuitOutOfSystem.into());
    }
    state.end_trade_route(fleet);
    state.end_patrol(fleet);
    state.navigation.holds.retain(|held| *held != fleet);
    state.navigation.pursuit = Some(Pursuit { fleet, quarry });
    let events = service_pursuit(state, economy, player, systems)?;
    if state.navigation.pursuit.is_some() && state.navigation.order(fleet).is_none() {
        // No course could be plotted: nothing was ordered.
        state.navigation.pursuit = None;
        return Err(NavigationError::NoCourse.into());
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

/// After a run: catch the quarry, lose it, or steer for where it is now.
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
    let (Some((system, target, at)), Some((lead, here, position))) = (found, ours) else {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: the quarry got away.")]);
    };
    if system != here {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: the quarry left the system.")]);
    }
    let caught = match quarry {
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
    };
    if let Some(contact) = caught {
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
    use crate::movement::MovementError;
    match state.plot_course(player, lead, target, systems) {
        Ok(_) => Ok(Vec::new()),
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
            Ok(vec![note("Pursuit ended: no course to the quarry.")])
        }
    }
}
