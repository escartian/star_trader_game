//! Closing in on a quarry in the flagship's own system: a pursuit curve
//! (`navigation::chase`) instead of a straight course. The flagship holds
//! (a Wait order) while the run plays out; when the run is resolved it flies
//! up to a run's distance after the quarry, which moved from where it was
//! last seen to where it is now, and pays fuel for the distance flown. Caught
//! within reach, it is alongside and the contact opens (`catch`).
use super::{ENGAGE_KM, Quarry, catch, docked};
use crate::{
    LocalPosition, StarSystem, SystemId,
    economy::Economy,
    fleet::software::PURSUIT_COURSE_PERCENT,
    fleet::{FleetId, FleetMovementError, software::FleetSoftware},
    navigation::{NavigationEvent, chase::chase},
    piracy::HAIL_KM,
    travel::{Location, PlayerId, TravelState},
};

/// Where the chase is: the flagship's system and position, and the quarry's
/// position now and the point to close on (off the port for a docked trader).
pub(super) struct Closing {
    pub system: SystemId,
    pub from: LocalPosition,
    pub quarry_now: LocalPosition,
    pub target: LocalPosition,
}

/// One run of the in-system chase. `ran`: a run has just played out (else the
/// pursuit only began, and the flagship merely holds for the first run).
pub(super) fn close_in(
    state: &mut TravelState,
    economy: &Economy,
    (player, fleet): (PlayerId, FleetId),
    quarry: Quarry,
    closing: Closing,
    ran: bool,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    let note = |note: &str| NavigationEvent::PursuitNote {
        fleet,
        note: note.into(),
    };
    let seen = state
        .navigation
        .pursuit
        .and_then(|pursuit| pursuit.seen)
        .filter(|(system, _)| *system == closing.system)
        .map_or(closing.quarry_now, |(_, at)| at);
    remember(state, closing.system, closing.quarry_now);
    state.navigation.chase_trail.clear();
    // Hold during runs: the chase, not a course, moves the flagship.
    if state
        .navigation
        .order(fleet)
        .is_some_and(|order| !order.started)
    {
        state.cancel_course(player, fleet)?;
    }
    if !state.navigation.holds.contains(&fleet) {
        state.navigation.holds.push(fleet);
    }
    if !ran {
        return Ok(Vec::new());
    }
    let members = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .map(|record| (record.ships.clone(), record.software.clone()))
        .unwrap_or_default();
    let rules = &state.movement_rules;
    let mut budget = i64::try_from(
        rules
            .local_km_per_tick
            .saturating_mul(rules.ticks_per_navigation_run),
    )
    .unwrap_or(i64::MAX);
    if members.1.contains(&FleetSoftware::PursuitSuite) {
        budget = budget.saturating_mul(100) / PURSUIT_COURSE_PERCENT as i64;
    }
    let reach = match quarry {
        Quarry::Trader(_) => HAIL_KM,
        Quarry::Band(_) | Quarry::Patrol(_) => ENGAGE_KM,
    };
    let flight = chase(
        closing.from,
        budget,
        (seen, closing.target),
        reach,
        rules.star_exclusion_radius_km,
    );
    // Whole fuel units, rounded up, for the distance flown.
    let per_fuel = rules.local_km_per_fuel.max(1);
    let flown = u64::try_from(flight.flown_km).unwrap_or(0);
    let fuel = u32::try_from(flown.div_ceil(per_fuel)).unwrap_or(u32::MAX);
    let fueled = members
        .0
        .iter()
        .all(|id| state.ship(*id).is_ok_and(|ship| ship.fuel >= fuel));
    if !fueled {
        state.navigation.pursuit = None;
        return Ok(vec![note("Pursuit ended: not enough fuel to keep up.")]);
    }
    state.navigation.chase_trail = flight.path.clone();
    for id in &members.0 {
        let ship = state.ship_mut(*id)?;
        ship.fuel -= fuel;
        ship.location = Location::InSystem {
            system: closing.system,
            position: flight.end,
        };
    }
    if !flight.caught {
        return Ok(Vec::new());
    }
    let approaching = state
        .navigation
        .pursuit
        .is_some_and(|pursuit| !pursuit.quarry_docked)
        && docked(economy, quarry);
    let contact = catch(
        state,
        economy,
        (player, fleet),
        quarry,
        (flight.end, closing.quarry_now),
        approaching,
        systems,
    )?;
    match contact {
        Some(contact) => {
            state.navigation.pursuit = None;
            state.navigation.holds.retain(|held| *held != fleet);
            Ok(vec![contact])
        }
        None => Ok(Vec::new()),
    }
}

fn remember(state: &mut TravelState, system: SystemId, at: LocalPosition) {
    if let Some(pursuit) = &mut state.navigation.pursuit {
        pursuit.seen = Some((system, at));
    }
}
