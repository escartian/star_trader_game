//! Distress calls: a trader that a raider band stalks asks your flagship for
//! help. Fighting opens a battle against the band, as any band contact does
//! (its bounty is paid as usual, and the band stops stalking whatever
//! happens). If you win, the trader pays you `rescue_reward_percent` of its
//! wallet and its faction grants `rescue_standing` (see `standing` in the
//! host). Leaving it costs nothing; the band strikes it in its own time.
use crate::{
    combat::CombatOutcome,
    economy::{Credits, Economy},
    fleet::{FleetId, FleetMovementError},
    navigation::{Encounter, EncounterKind, RaiderBand},
    npc::Trader,
    travel::{TravelError, TravelState},
};

/// The band stalking `trader`, if one is.
pub(super) fn stalker(state: &TravelState, trader: u64) -> Option<&RaiderBand> {
    state
        .navigation
        .bands
        .iter()
        .find(|band| band.stalking == Some(trader))
}

/// Opens `npc`'s call for help against `band` to the flagship `fleet`.
pub(super) fn open_distress(
    state: &mut TravelState,
    fleet: FleetId,
    npc: &Trader,
    band: &RaiderBand,
) -> Result<Encounter, FleetMovementError> {
    let navigation = &mut state.navigation;
    let next = navigation
        .next_encounter_id
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let encounter = Encounter {
        id: navigation.next_encounter_id,
        fleet,
        kind: EncounterKind::TraderDistress,
        run: navigation.run,
        hostiles: band.ships.clone(),
        band: Some(band.id),
        haven: false,
        trader: None,
        police: None,
        rescue: Some(npc.id),
    };
    navigation.next_encounter_id = next;
    navigation.encounters.push(encounter.clone());
    Ok(encounter)
}

/// After the battle of a distress call is filed and won: the trader pays its
/// reward (recorded in the report). Call once per newly filed report.
pub fn settle_rescue(
    state: &mut TravelState,
    economy: &mut Economy,
) -> Result<(), FleetMovementError> {
    let Some(report) = state.navigation.last_combat.as_deref_mut() else {
        return Ok(());
    };
    let Some(id) = report.rescue else {
        return Ok(());
    };
    let Some(npc) = economy.traders.iter_mut().find(|npc| npc.id == id) else {
        return Ok(());
    };
    if report.outcome != CombatOutcome::Victory {
        npc.last_action = "Called for help; the raiders were not driven off".into();
        return Ok(());
    }
    let percent = economy
        .rules
        .personality_catalog
        .approaches
        .rescue_reward_percent;
    let reward = npc.credits.0 / 100 * percent;
    npc.credits = Credits(npc.credits.0 - reward);
    npc.last_action = format!(
        "Rescued from raiders; paid its rescuers {} cr",
        Credits(reward)
    );
    report.rescue_reward = Credits(reward);
    economy.credits = Credits(
        economy
            .credits
            .0
            .checked_add(reward)
            .ok_or(TravelError::TimeOverflow)?,
    );
    Ok(())
}
