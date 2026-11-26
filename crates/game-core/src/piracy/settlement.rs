//! Cargo seizures and the outcome of attacks on NPC traders.
use super::{position, would_yield};
use crate::{
    StarSystem,
    combat::Side,
    economy::Economy,
    fleet::{FleetError, FleetId, FleetMovementError},
    navigation::{EncounterChoice, NavigationError, NavigationEvent},
    npc::Trader,
    travel::{PlayerId, TravelError, TravelState},
};

/// Resolves a hailed trader's contact. Demand is settled here, against the
/// trader's hold; every other choice goes to `TravelState::resolve_encounter`.
pub fn resolve(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    encounter_id: u64,
    choice: EncounterChoice,
) -> Result<NavigationEvent, FleetMovementError> {
    let encounter = state
        .navigation
        .encounters
        .iter()
        .find(|encounter| encounter.id == encounter_id)
        .ok_or(NavigationError::UnknownEncounter)?
        .clone();
    if choice != EncounterChoice::Demand {
        return state.resolve_encounter(player, encounter_id, choice, &economy.rules);
    }
    let hailed = encounter
        .trader
        .as_ref()
        .ok_or(NavigationError::InvalidChoice)?;
    let fleet = state
        .fleets
        .iter()
        .find(|record| record.id == encounter.fleet)
        .ok_or(FleetError::UnknownFleet)?;
    if fleet.owner != player {
        return Err(FleetError::NotOwner.into());
    }
    if !would_yield(state, economy, &encounter) {
        return Err(NavigationError::WillNotYield.into());
    }
    let index = position(economy, hailed.id)?;
    let mut npc = economy.traders[index].clone();
    let half = npc.state.ships[0].cargo.iter().sum::<u32>().div_ceil(2);
    let taken = take_cargo(state, economy, encounter.fleet, &mut npc, half)?;
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let piracies = state
        .navigation
        .piracies
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    npc.last_action = format!("Handed {taken} cargo to pirates under threat");
    economy.traders[index] = npc;
    state.navigation.piracies = piracies;
    record_seizure(state, taken);
    state
        .navigation
        .encounters
        .retain(|record| record.id != encounter_id);
    state.revision = revision;
    Ok(NavigationEvent::EncounterResolved {
        fleet: encounter.fleet,
        outcome: format!("{} handed over {taken} cargo.", hailed.name),
    })
}

/// After a battle with a trader is filed (piracy, or a corsair's shakedown): the trader takes its damage, or, if it was
/// destroyed, its hold is plundered and it is refitted at its home port. Call
/// once per newly filed report.
pub fn settle_battle(
    state: &mut TravelState,
    economy: &mut Economy,
    systems: &[StarSystem],
) -> Result<(), FleetMovementError> {
    let Some(report) = state.navigation.last_combat.as_deref() else {
        return Ok(());
    };
    let Some(id) = report.trader else {
        return Ok(());
    };
    let fleet = report.fleet;
    let shakedown = report.shakedown;
    // The trader is the first hostile; a second one is its escort.
    let Some(result) = report.side(Side::Hostile).next().cloned() else {
        return Err(FleetError::InvalidSelection.into());
    };
    let escort_lost = report
        .side(Side::Hostile)
        .skip(1)
        .any(|unit| unit.destroyed);
    let index = position(economy, id)?;
    let mut npc = economy.traders[index].clone();
    if escort_lost {
        npc.escort_runs = 0;
    }
    let hull = format!("hull {}/{}", result.hull - result.final_damage, result.hull);
    let mut plunder = 0;
    if result.destroyed {
        // The fleet may itself be gone; then nothing is carried off.
        if state.fleets.iter().any(|record| record.id == fleet) {
            plunder = take_cargo(state, economy, fleet, &mut npc, u32::MAX)?;
        }
        let port = economy.refit(&mut npc, systems);
        npc.last_action = if shakedown {
            format!("Destroyed in a failed shakedown; refitting at {port}")
        } else {
            format!("Destroyed by pirates; refitting at {port}")
        };
    } else {
        npc.state.ships[0].hull_damage = result.final_damage;
        npc.last_action = if shakedown {
            format!("Beaten off in a shakedown ({hull})")
        } else {
            format!("Survived a pirate attack ({hull})")
        };
    }
    economy.traders[index] = npc;
    if !shakedown {
        record_seizure(state, plunder);
    }
    if let Some(report) = state.navigation.last_combat.as_deref_mut() {
        report.plunder = plunder;
    }
    Ok(())
}

/// A successful seizure of `units` units from a trader (none when 0): it
/// advances your pirate record and every active raid job.
fn record_seizure(state: &mut TravelState, units: u32) {
    if units == 0 {
        return;
    }
    let record = &mut state.navigation;
    record.pirate_seizures = record.pirate_seizures.saturating_add(1);
    record.pirate_cargo = record.pirate_cargo.saturating_add(u64::from(units));
    state.jobs.record_seizure(units);
}

/// Moves up to `limit` units from the trader's hold into the fleet's free
/// space, most valuable goods first. Returns the units moved.
pub(crate) fn take_cargo(
    state: &mut TravelState,
    economy: &Economy,
    fleet: FleetId,
    npc: &mut Trader,
    limit: u32,
) -> Result<u32, FleetMovementError> {
    let members = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .ok_or(FleetError::UnknownFleet)?
        .ships
        .clone();
    // List price: the average of the market templates' base prices.
    let templates = &economy.rules.markets;
    let list = |good: usize| -> u64 {
        let total: u64 = templates.iter().map(|m| m.base_prices[good].0).sum();
        total / templates.len().max(1) as u64
    };
    let mut goods: Vec<usize> = (0..6).collect();
    goods.sort_by_key(|good| (std::cmp::Reverse(list(*good)), *good));
    let hold = &mut npc.state.ships[0].cargo;
    let mut left = limit;
    for good in goods {
        for id in &members {
            let ship = state.ship_mut(*id)?;
            let space = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
            let moved = hold[good]
                .min(left)
                .min(space.min(u64::from(u32::MAX)) as u32);
            ship.cargo[good] += moved;
            hold[good] -= moved;
            left -= moved;
        }
    }
    Ok(limit - left)
}
