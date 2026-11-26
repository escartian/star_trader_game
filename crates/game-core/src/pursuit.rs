//! Pursuit: the flagship chases an NPC trader or a raider band. Each run,
//! after everything has moved, a flagship in the quarry's system and within
//! reach opens the contact (a hail for a trader, a raider fight for a band)
//! and the pursuit ends. Otherwise, topped up at any market it rests at, it
//! steers to intercept: for a trader, where it is bound (its body or point,
//! in this system or another), since both move at the same speed; for a
//! band (half that speed), where it is. A quarry that is gone, or out of
//! range or fuel, ends it, and so does any other order for the flagship.
//! Positions are kilometres.
use crate::{
    BodyId, LocalPosition, StarSystem, SystemId,
    economy::Economy,
    fleet::{FleetId, FleetMovementError},
    movement::{MoveTarget, MovementError},
    navigation::{NavigationError, NavigationEvent, band_motion::distance},
    piracy,
    travel::{PlayerId, ShipId, TravelState},
};
use serde::{Deserialize, Serialize};

mod sighting;

/// What the flagship chases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Quarry {
    /// An NPC trader (`npc::Trader::id`).
    Trader(u64),
    /// A raider band (`navigation::RaiderBand::id`).
    Band(u64),
    /// A faction police patrol (`police::FactionPatrol::id`); caught, it is a
    /// `Police` contact you started: fight it or let it go.
    Patrol(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pursuit {
    pub fleet: FleetId,
    pub quarry: Quarry,
    /// The trader was docked when last seen: one that docks between two
    /// looks, within reach of the flagship, is caught on its way in.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quarry_docked: bool,
}

/// Whether the quarry is a trader docked at a station right now.
fn docked(economy: &Economy, quarry: Quarry) -> bool {
    let Quarry::Trader(id) = quarry else {
        return false;
    };
    economy
        .traders
        .iter()
        .find(|npc| npc.id == id)
        .is_some_and(|npc| {
            matches!(
                npc.state.ships[0].location,
                crate::travel::Location::Docked(_)
            )
        })
}

/// How close (km) the flagship must come to a raider band or police patrol
/// to engage it on purpose: the same distance at which they strike or stop
/// your fleets (`navigation::STRIKE_KM`). Both keep moving and turn at
/// orbiting bodies, so a chase gets close rather than exactly onto them.
pub const ENGAGE_KM: i64 = crate::navigation::STRIKE_KM;

/// The flagship's lead ship, and its system and position when in a system.
fn flagship_at(
    state: &TravelState,
    fleet: FleetId,
    systems: &[StarSystem],
) -> Option<(ShipId, Option<(SystemId, LocalPosition)>)> {
    let record = state.fleets.iter().find(|record| record.id == fleet)?;
    let lead = state.ship(*record.ships.first()?).ok()?;
    let here = lead.location.system().and_then(|system| {
        let at = sighting::local(
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
    economy: &mut Economy,
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
        Quarry::Patrol(_) => NavigationError::UnknownPatrol,
    };
    sighting::sight(state, economy, quarry, systems).ok_or(missing)?;
    state.end_trade_route(fleet);
    state.end_patrol(fleet);
    state.navigation.holds.retain(|held| *held != fleet);
    state.navigation.pursuit = Some(Pursuit {
        fleet,
        quarry,
        quarry_docked: docked(economy, quarry),
    });
    let events = service_pursuit(state, economy, player, systems)?;
    let ordered =
        state.navigation.order(fleet).is_some() || state.navigation.holds.contains(&fleet);
    if state.navigation.pursuit.is_some() && !ordered {
        state.navigation.pursuit = None;
        return Err(NavigationError::NoCourse.into());
    }
    if let Some(NavigationEvent::PursuitNote { note, .. }) = events.first()
        && note.starts_with("Pursuit ended")
    {
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
    economy: &mut Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let Some(Pursuit {
        fleet,
        quarry,
        quarry_docked,
    }) = state.navigation.pursuit
    else {
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
    let found = sighting::sight(state, economy, quarry, systems);
    let ours = flagship_at(state, fleet, systems);
    let (Some(found), Some((lead, here))) = (found, ours) else {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: the quarry got away.")]);
    };
    if let (Some((system, at)), Some((ours, position))) = (found.now, here)
        && system == ours
        && let Some(contact) = catch(
            state,
            economy,
            (player, fleet),
            quarry,
            (position, at),
            !quarry_docked && docked(economy, quarry),
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
    if let Some(pursuit) = &mut state.navigation.pursuit {
        pursuit.quarry_docked = docked(economy, quarry);
    }
    if state
        .navigation
        .order(fleet)
        .is_some_and(|order| order.started)
    {
        // Still on the last leg; steer again once it ends.
        return Ok(Vec::new());
    }
    // At a market on the way: top up, as route fleets do.
    refuel(state, economy, fleet);
    let (system, target) = found.steer;
    let plotted = if here.is_some_and(|(ours, _)| ours == system) {
        match target {
            // Already in the system it is entering: wait for it.
            MoveTarget::EnterSystem(_) => Err(MovementError::AlreadyThere.into()),
            target => state.plot_course(player, lead, target, systems).map(|_| ()),
        }
    } else {
        let body = match target {
            MoveTarget::Body { body, .. } => Some(body),
            _ => None,
        };
        follow(state, player, lead, fleet, system, body, systems)
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
/// `approaching`: the trader docked since the last look, so it is caught on
/// its way in.
fn catch(
    state: &mut TravelState,
    economy: &Economy,
    (player, fleet): (PlayerId, FleetId),
    quarry: Quarry,
    (position, at): (LocalPosition, LocalPosition),
    approaching: bool,
    systems: &[StarSystem],
) -> Result<Option<NavigationEvent>, FleetMovementError> {
    Ok(match quarry {
        Quarry::Trader(id) if approaching => {
            piracy::hail_on_approach(state, economy, player, fleet, id, systems).ok()
        }
        // Under a station's guns: wait off the port for it to come out.
        Quarry::Trader(_) if docked(economy, quarry) => None,
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
        Quarry::Patrol(id) if distance(position, at) <= ENGAGE_KM => {
            let patrol = state
                .navigation
                .police
                .patrol(id)
                .cloned()
                .ok_or(NavigationError::UnknownPatrol)?;
            Some(crate::police::intercept(state, &patrol, fleet)?)
        }
        Quarry::Patrol(_) => None,
    })
}

/// Tops up every ship in `fleet` that rests at a market, as far as credits
/// allow (trade-route fleets refuel the same way).
fn refuel(state: &mut TravelState, economy: &mut Economy, fleet: FleetId) {
    let ships = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .map(|record| record.ships.clone())
        .unwrap_or_default();
    for id in ships {
        if let Ok(ship) = state.ship_mut(id)
            && let Ok(units) = economy.max_refuel_units(ship)
        {
            let _ = economy.refuel(ship, units);
        }
    }
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
