//! Hailing and robbing NPC traders.
//!
//! - The flagship may **hail** an NPC trader within [`HAIL_KM`] of it, both
//!   in open space or orbit (a fleet docked at a station or landed is out of
//!   reach, and so is a trader laid up for a refit). Hailing opens a `Trader`
//!   contact showing the trader's hold and hull.
//! - A trader **docked** at a station can be hailed only by a flagship docked
//!   at the same port, and only to trade: the hail opens its `TraderOffer`
//!   (its hold below list price, see `approach`); the station's guns rule out
//!   demands and attacks.
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
//! - Every demand and attack counts in `NavigationState::piracies` and causes
//!   faction standing and wanted consequences where factions hold territory.
//!   Successful cargo seizures advance the Pirate road.
use crate::{
    StarSystem,
    combat::gear::ESCORT_KIND,
    economy::Economy,
    fleet::{FleetError, FleetId, FleetMovementError, ShipCatalog, ShipKind},
    navigation::{Encounter, EncounterKind, NavigationError, NavigationEvent, TraderContact},
    npc::{Personality, PersonalityCatalog, Trader},
    travel::{Location, PlayerId, TravelError, TravelState},
};

mod settlement;
pub use settlement::{resolve, settle_battle};

/// Hailing range between the flagship and a trader, in km (about half the
/// Earth–Moon distance). A chase meets a trader where it is bound, so this
/// only needs to cover arriving at the same spot or berth.
pub const HAIL_KM: i64 = 200_000;

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
    reach_check(state, economy, player, fleet, trader, systems, false)
}

/// `hail_check`; `approaching` treats a trader that has just docked as still
/// in open space (a pursuer caught it on its way in, see `hail_on_approach`).
fn reach_check(
    state: &TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    trader: u64,
    systems: &[StarSystem],
    approaching: bool,
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
    if npc.refit_runs > 0 {
        return Err(NavigationError::TraderProtected.into());
    }
    let lead = state.ship(*record.ships.first().ok_or(FleetError::UnknownFleet)?)?;
    // A docked trader can be hailed only from the same port, and only to
    // trade: the station's guns protect it (see `hail`).
    if let Location::Docked(port) = theirs.location
        && !approaching
    {
        return match lead.location {
            Location::Docked(ours) if ours == port && state.navigation.run > 0 => Ok(()),
            _ => Err(NavigationError::TraderProtected.into()),
        };
    }
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
    open_hail(state, economy, player, fleet, trader, systems, false)
}

/// A pursuing flagship within reach of a trader that docked this very run
/// caught it on its way in: the full hail, as in open space, before the
/// station's guns cover it.
pub fn hail_on_approach(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    trader: u64,
    systems: &[StarSystem],
) -> Result<NavigationEvent, FleetMovementError> {
    open_hail(state, economy, player, fleet, trader, systems, true)
}

fn open_hail(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    fleet: FleetId,
    trader: u64,
    systems: &[StarSystem],
    approaching: bool,
) -> Result<NavigationEvent, FleetMovementError> {
    reach_check(state, economy, player, fleet, trader, systems, approaching)?;
    let npc = find(economy, trader)?;
    // At a port the station's guns allow trade only: its offer of its hold.
    let docked = !approaching && matches!(npc.state.ships[0].location, Location::Docked(_));
    if docked && npc.state.ships[0].cargo_used() == 0 {
        return Err(NavigationError::NothingToSell.into());
    }
    let kind = if docked {
        EncounterKind::TraderOffer
    } else {
        EncounterKind::Trader
    };
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
        kind,
        run: navigation.run,
        hostiles,
        band: None,
        haven: false,
        trader: Some(contact),
        police: None,
        rescue: None,
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
