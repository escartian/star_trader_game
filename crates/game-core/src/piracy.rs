//! Hailing and robbing NPC traders.
//!
//! - The flagship may **hail** an NPC trader within [`HAIL_KM`] of it, both
//!   in open space or orbit (a fleet docked at a station or landed is out of
//!   reach, and so is a trader docked at a station or laid up for a refit).
//!   Hailing opens a `Trader` contact showing the trader's hold and hull.
//! - **Let it go** (`Bypass`) ends the contact.
//! - **Demand cargo** works only when the trader is outgunned: your fleet's
//!   [`strength`] must reach [`yield_percent`] of the trader's (its
//!   personality's `yield_percent`: 150% for a cautious trader, 300% for an
//!   opportunist, 400% for a corsair; see `config/npc.json`). It hands over half its cargo,
//!   rounded up, most valuable goods first, as far as your holds have room.
//! - **Attack** opens the usual tactical battle against the trader's ship.
//!   If the trader is destroyed you plunder its hold (what fits), and it is
//!   refitted at its home port; if it withdraws it escapes with its cargo.
//!   Bounties are for raiders only. Giving up just breaks the attack off.
//! - A trader's hired escort fights beside it and counts toward its strength;
//!   so does yours. Destroying the escort ends the trader's contract.
//! - Every demand and attack counts in `NavigationState::piracies`. Law
//!   consequences (wanted status, police) arrive with factions.
use crate::{
    StarSystem,
    combat::{Side, gear::ESCORT_KIND},
    economy::Economy,
    fleet::{FleetError, FleetId, FleetMovementError, ShipCatalog, ShipKind},
    navigation::{
        Encounter, EncounterChoice, EncounterKind, NavigationError, NavigationEvent, TraderContact,
    },
    npc::{Personality, PersonalityCatalog, Trader},
    travel::{Location, PlayerId, TravelError, TravelState},
};

/// Hailing range between the flagship and a trader, in km.
pub const HAIL_KM: i64 = 20_000_000;

/// The share of the trader's strength (in percent) a fleet needs for a demand.
pub fn yield_percent(catalog: &PersonalityCatalog, personality: Personality) -> u64 {
    catalog.get(personality).yield_percent
}

/// Fighting strength of some hulls: hull points left plus shields, times the
/// weapons fitted (at least one). Unknown hulls count for nothing.
pub fn strength(ships: impl IntoIterator<Item = (ShipKind, u32)>, catalog: &ShipCatalog) -> u64 {
    ships
        .into_iter()
        .filter_map(|(kind, damage)| {
            let combat = &catalog.spec(kind)?.combat;
            let points = u64::from(combat.hull.saturating_sub(damage) + combat.shields);
            Some(points * combat.loadout.len().max(1) as u64)
        })
        .sum()
}

/// Checks that the flagship `fleet` can hail `trader` now.
pub fn hail_check(
    state: &TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    trader: u64,
    systems: &[StarSystem],
) -> Result<(), FleetMovementError> {
    let record = state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .ok_or(FleetError::UnknownFleet)?;
    if record.owner != player {
        return Err(FleetError::NotOwner.into());
    }
    if state.flagship(player) != Some(fleet) {
        return Err(NavigationError::NotFlagship.into());
    }
    if state.navigation.battle.is_some() {
        return Err(NavigationError::BattleUnderway.into());
    }
    if state.navigation.encounters.iter().any(|c| c.fleet == fleet) {
        return Err(NavigationError::EncounterPending(fleet).into());
    }
    let npc = find(economy, trader)?;
    let theirs = &npc.state.ships[0];
    if npc.refit_runs > 0 || matches!(theirs.location, Location::Docked(_)) {
        return Err(NavigationError::TraderProtected.into());
    }
    let lead = state.ship(*record.ships.first().ok_or(FleetError::UnknownFleet)?)?;
    // Contacts need a run on record; nobody has met anyone on run 0.
    if state.navigation.run == 0
        || matches!(lead.location, Location::Docked(_) | Location::Landed { .. })
    {
        return Err(NavigationError::TraderOutOfReach.into());
    }
    let (Some(ours), Some(at)) = (
        state.navigation_position(lead.id, systems),
        npc.state.navigation_position(theirs.id, systems),
    ) else {
        return Err(NavigationError::TraderOutOfReach.into());
    };
    let gap: i128 = [
        (ours.x_km, at.x_km),
        (ours.y_km, at.y_km),
        (ours.z_km, at.z_km),
    ]
    .iter()
    .map(|(a, b)| (i128::from(*a) - i128::from(*b)).pow(2))
    .sum();
    if gap > i128::from(HAIL_KM).pow(2) {
        return Err(NavigationError::TraderOutOfReach.into());
    }
    Ok(())
}

/// The traders the player's flagship can hail right now.
pub fn in_reach(
    state: &TravelState,
    economy: &Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Vec<u64> {
    let Some(flagship) = state.flagship(player) else {
        return Vec::new();
    };
    economy
        .traders
        .iter()
        .filter(|npc| hail_check(state, economy, player, flagship, npc.id, systems).is_ok())
        .map(|npc| npc.id)
        .collect()
}

/// Opens a `Trader` contact between the flagship `fleet` and `trader`.
pub fn hail(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    trader: u64,
    systems: &[StarSystem],
) -> Result<NavigationEvent, FleetMovementError> {
    hail_check(state, economy, player, fleet, trader, systems)?;
    let npc = find(economy, trader)?;
    let navigation = &mut state.navigation;
    let next = navigation
        .next_encounter_id
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let (contact, hostiles) = contact_of(npc);
    let encounter = Encounter {
        id: navigation.next_encounter_id,
        fleet,
        kind: EncounterKind::Trader,
        run: navigation.run,
        hostiles,
        band: None,
        haven: false,
        trader: Some(contact),
        police: None,
    };
    navigation.next_encounter_id = next;
    navigation.encounters.push(encounter.clone());
    state.revision = revision;
    Ok(NavigationEvent::EncounterFound(encounter))
}

/// A contact's view of `npc` as it stands, and its hostiles: its hull, then
/// its hired escort if it has one.
pub(crate) fn contact_of(npc: &Trader) -> (TraderContact, Vec<ShipKind>) {
    let ship = &npc.state.ships[0];
    let escort = npc.escort_runs > 0;
    let mut hostiles = vec![ship.kind];
    if escort {
        hostiles.push(ESCORT_KIND);
    }
    let contact = TraderContact {
        id: npc.id,
        name: npc.name.clone(),
        hull_damage: ship.hull_damage,
        gear: ship.gear.clone(),
        escort,
    };
    (contact, hostiles)
}

/// Whether the hailed trader would hand cargo over to the contact's fleet.
pub fn would_yield(state: &TravelState, economy: &Economy, encounter: &Encounter) -> bool {
    let catalog = &economy.rules.ship_catalog;
    let Some(npc) = encounter
        .trader
        .as_ref()
        .and_then(|hailed| find(economy, hailed.id).ok())
    else {
        return false;
    };
    // Hired escorts count on both sides.
    let our_escort =
        (state.navigation.escort_runs(encounter.fleet) > 0).then_some((ESCORT_KIND, 0));
    let ours = state
        .fleets
        .iter()
        .find(|record| record.id == encounter.fleet)
        .into_iter()
        .flat_map(|record| &record.ships)
        .filter_map(|id| state.ship(*id).ok())
        .map(|ship| (ship.kind, ship.hull_damage))
        .chain(our_escort);
    let ship = &npc.state.ships[0];
    let their_escort = (npc.escort_runs > 0).then_some((ESCORT_KIND, 0));
    let theirs = strength(
        std::iter::once((ship.kind, ship.hull_damage)).chain(their_escort),
        catalog,
    );
    u128::from(strength(ours, catalog)) * 100
        >= u128::from(theirs)
            * u128::from(yield_percent(
                &economy.rules.personality_catalog,
                npc.personality,
            ))
}

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
    if let Some(report) = state.navigation.last_combat.as_deref_mut() {
        report.plunder = plunder;
    }
    Ok(())
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

/// Save check: each hailed trader contact matches the trader as it stands
/// (no run can pass while the contact is pending).
pub fn validate_contacts(state: &TravelState, economy: &Economy) -> Result<(), String> {
    for encounter in &state.navigation.encounters {
        let Some(hailed) = &encounter.trader else {
            continue;
        };
        let npc = find(economy, hailed.id).map_err(|error| error.to_string())?;
        let ship = &npc.state.ships[0];
        let mut hostiles = vec![ship.kind];
        if npc.escort_runs > 0 {
            hostiles.push(ESCORT_KIND);
        }
        if npc.name != hailed.name
            || npc.refit_runs > 0
            || encounter.hostiles != hostiles
            || hailed.escort != (npc.escort_runs > 0)
            || ship.gear != hailed.gear
            || ship.hull_damage != hailed.hull_damage
        {
            return Err("Hailed trader differs from the saved trader".into());
        }
    }
    Ok(())
}

pub(crate) fn find(economy: &Economy, trader: u64) -> Result<&Trader, NavigationError> {
    economy
        .traders
        .iter()
        .find(|npc| npc.id == trader)
        .ok_or(NavigationError::UnknownTrader)
}

pub(crate) fn position(economy: &Economy, trader: u64) -> Result<usize, NavigationError> {
    economy
        .traders
        .iter()
        .position(|npc| npc.id == trader)
        .ok_or(NavigationError::UnknownTrader)
}

#[cfg(test)]
mod tests;
