//! Settling a `Police` contact, and paying a fine at a faction's port.
//!
//! - **Comply** charges the fine ([`fine`]) to the wallet. If the wallet falls
//!   short, the patrol takes what there is and impounds every cargo unit
//!   aboard the fleet. Either way the record is cleared.
//! - **Fight** and **Auto-resolve** open a battle against the patrol's ships,
//!   as with raiders, and count as a crime: the faction's `wanted_runs` again.
//!   No bounty is paid for police.
//! - **Evade** is the raiders' engine roll; failure opens the battle (it is
//!   not a further crime). An escape leaves the record as it was.
use super::PoliceContact;
use crate::{
    economy::{Credits, Economy, EconomyError, PRICE_LEVEL_UNIT},
    factions::Faction,
    fleet::{FleetError, FleetMovementError},
    navigation::{EncounterChoice, EncounterKind, NavigationError, NavigationEvent},
    travel::{PlayerId, Ship, TravelError, TravelState},
};

/// The fine that clears your record with `faction`, at today's prices.
pub fn fine(economy: &Economy, faction: Faction) -> Credits {
    let base = economy
        .factions
        .fine_cents(faction, &economy.rules.faction_catalog);
    let scaled =
        u128::from(base) * u128::from(economy.price_level_ppm) / u128::from(PRICE_LEVEL_UNIT);
    Credits(u64::try_from(scaled).unwrap_or(u64::MAX))
}

/// Resolves a `Police` contact (see the module docs).
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
    let (
        EncounterKind::Police,
        Some(PoliceContact {
            patrol,
            faction,
            intercepted,
        }),
    ) = (encounter.kind, encounter.police)
    else {
        return Err(NavigationError::InvalidChoice.into());
    };
    if intercepted {
        return resolve_intercept(state, economy, player, &encounter, choice, faction);
    }
    match choice {
        EncounterChoice::Fight | EncounterChoice::AutoResolve => {
            let event = state.resolve_encounter(player, encounter_id, choice, &economy.rules)?;
            let catalog = economy.rules.faction_catalog.clone();
            economy.factions.charge(faction, 1, &catalog);
            return Ok(event);
        }
        EncounterChoice::Evade => {
            return state.resolve_encounter(player, encounter_id, choice, &economy.rules);
        }
        EncounterChoice::Comply => {}
        _ => return Err(NavigationError::InvalidChoice.into()),
    }
    let fleet = state
        .fleets
        .iter()
        .find(|record| record.id == encounter.fleet)
        .ok_or(FleetError::UnknownFleet)?
        .clone();
    if fleet.owner != player {
        return Err(FleetError::NotOwner.into());
    }
    if state.navigation.battle.is_some() {
        return Err(NavigationError::BattleUnderway.into());
    }
    let revision = state
        .revision
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let due = fine(economy, faction);
    let paid = due.0.min(economy.credits.0);
    economy.credits = Credits(economy.credits.0 - paid);
    let mut impounded = 0;
    if paid < due.0 {
        for id in &fleet.ships {
            let ship = state.ship_mut(*id)?;
            impounded += ship.cargo.iter().sum::<u32>();
            ship.cargo = [0; 6];
        }
    }
    economy.factions.clear_wanted(faction);
    state.police_after_contact(patrol, &[]);
    state
        .navigation
        .encounters
        .retain(|record| record.id != encounter_id);
    state.revision = revision;
    let outcome = if paid == due.0 {
        format!(
            "Paid the {} fine of {due}. Your record is clear.",
            faction.label()
        )
    } else {
        format!(
            "Paid {} of the {} fine of {due}; the patrol impounded {impounded} cargo. Your record is clear.",
            Credits(paid),
            faction.label()
        )
    };
    Ok(NavigationEvent::EncounterResolved {
        fleet: fleet.id,
        outcome,
    })
}

/// A patrol you ran down: **Fight** or **Auto-resolve** attack it (a crime,
/// as when you resist a stop); **Bypass** lets it go and nothing happens.
fn resolve_intercept(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    encounter: &crate::navigation::Encounter,
    choice: EncounterChoice,
    faction: Faction,
) -> Result<NavigationEvent, FleetMovementError> {
    match choice {
        EncounterChoice::Fight | EncounterChoice::AutoResolve => {
            let event = state.resolve_encounter(player, encounter.id, choice, &economy.rules)?;
            let catalog = economy.rules.faction_catalog.clone();
            economy.factions.charge(faction, 1, &catalog);
            Ok(event)
        }
        EncounterChoice::Bypass => {
            let fleet = state
                .fleets
                .iter()
                .find(|record| record.id == encounter.fleet)
                .ok_or(FleetError::UnknownFleet)?;
            if fleet.owner != player {
                return Err(FleetError::NotOwner.into());
            }
            let revision = state
                .revision
                .checked_add(1)
                .ok_or(TravelError::TimeOverflow)?;
            state
                .navigation
                .encounters
                .retain(|record| record.id != encounter.id);
            state.revision = revision;
            Ok(NavigationEvent::EncounterResolved {
                fleet: encounter.fleet,
                outcome: format!("You let the {} patrol go.", faction.label()),
            })
        }
        _ => Err(NavigationError::InvalidChoice.into()),
    }
}

/// Pays your fine at the port `ship` is at, to the port's treasury; returns
/// the faction and the amount. The port must belong to a faction that wants you.
pub fn pay_fine_at_port(
    economy: &mut Economy,
    ship: &Ship,
) -> Result<(Faction, Credits), EconomyError> {
    let market = economy.current_market(ship)?.id;
    let faction = economy
        .factions
        .owner(market.system)
        .filter(|faction| economy.factions.is_wanted(*faction))
        .ok_or(EconomyError::NotWanted)?;
    let cost = fine(economy, faction);
    let wallet = economy
        .credits
        .0
        .checked_sub(cost.0)
        .ok_or(EconomyError::InsufficientCredits)?;
    let treasury = economy
        .market(market)?
        .credits
        .0
        .checked_add(cost.0)
        .ok_or(EconomyError::ArithmeticOverflow)?;
    economy.market_mut(market)?.credits = Credits(treasury);
    economy.credits = Credits(wallet);
    economy.factions.clear_wanted(faction);
    Ok((faction, cost))
}
