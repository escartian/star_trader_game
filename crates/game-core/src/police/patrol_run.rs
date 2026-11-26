//! One navigation run of police patrols: wanted status runs down, wiped-out
//! patrols are replaced, and each patrol hunts you (when its faction pursues
//! you), answers a raider band, or visits its next body.
use super::{
    BAND_FIGHT_COOLDOWN_RUNS, DETECTION_KM, FactionPatrol, PoliceContact, STRIKE_KM, exposed_at,
};
use crate::{
    LocalPosition, StarSystem,
    combat::{CombatantSetup, Side, engagement::MAX_COMBATANTS, tactical},
    economy::Economy,
    fleet::{FleetError, FleetId, FleetMovementError, ShipKind},
    navigation::{Encounter, EncounterKind, NavigationEvent, distance, step_toward},
    travel::{PlayerId, TravelError, TravelState},
};

#[derive(Clone, Copy)]
enum Quarry {
    Fleet(FleetId),
    Band(u64),
}

/// Run step for the police, after fleets, bands and traders moved. Call
/// once per navigation run, with bodies placed for the new tick.
pub fn patrol_run(
    state: &mut TravelState,
    economy: &mut Economy,
    player: PlayerId,
    systems: &[StarSystem],
) -> Result<Vec<NavigationEvent>, FleetMovementError> {
    economy.factions.pass_run();
    if !economy.factions.is_active() {
        return Ok(Vec::new());
    }
    respawn(state, economy, systems);
    let ids: Vec<u64> = state
        .navigation
        .police
        .patrols
        .iter()
        .map(|patrol| patrol.id)
        .collect();
    let mut events = Vec::new();
    for id in ids {
        let Some(patrol) = state.navigation.police.patrol(id).cloned() else {
            continue;
        };
        let Some(world) = systems.iter().find(|known| known.id == patrol.system) else {
            continue;
        };
        let quarry = match patrol.cooldown {
            0 if economy.factions.pursues_you(patrol.faction) => {
                nearest_fleet(state, player, &patrol, world, systems)
                    .map(|(fleet, at)| (Quarry::Fleet(fleet), at))
            }
            0 => nearest_band(state, &patrol).map(|(band, at)| (Quarry::Band(band), at)),
            _ => None,
        };
        let (stops, heading) = match quarry {
            Some((_, at)) => (patrol.stops, at),
            None => super::rounds(&patrol, world, &state.movement_rules),
        };
        let position = step_toward(&state.movement_rules, world, patrol.position, heading);
        if let Some(record) = state
            .navigation
            .police
            .patrols
            .iter_mut()
            .find(|known| known.id == id)
        {
            record.cooldown = record.cooldown.saturating_sub(1);
            record.stops = stops;
            record.heading = heading;
            record.position = position;
            (record.pursuing, record.answering) = match quarry {
                Some((Quarry::Fleet(fleet), _)) => (Some(fleet), None),
                Some((Quarry::Band(band), _)) => (None, Some(band)),
                None => (None, None),
            };
        }
        match quarry {
            Some((Quarry::Fleet(fleet), at)) if distance(position, at) <= STRIKE_KM => {
                events.push(stop_fleet(state, &patrol, fleet, false)?);
            }
            Some((Quarry::Band(band), at)) if distance(position, at) <= STRIKE_KM => {
                if let Some(note) = fight_band(state, economy, player, id, band, world)? {
                    events.push(NavigationEvent::PoliceNote(note));
                }
            }
            _ => {}
        }
    }
    Ok(events)
}

/// Replaces wiped-out patrols whose time has come, in systems still held.
fn respawn(state: &mut TravelState, economy: &Economy, systems: &[StarSystem]) {
    let run = state.navigation.run;
    let police = &mut state.navigation.police;
    let due: Vec<_> = police
        .respawns
        .iter()
        .filter(|(_, at)| *at <= run)
        .map(|(system, _)| *system)
        .collect();
    police.respawns.retain(|(_, at)| *at > run);
    for system in due {
        let manned = state
            .navigation
            .police
            .patrols
            .iter()
            .any(|patrol| patrol.system == system);
        let holder = economy.factions.owner(system);
        if let (false, Some(faction), Some(world)) = (
            manned,
            holder,
            systems.iter().find(|known| known.id == system),
        ) {
            state.add_patrol(faction, world);
        }
    }
}

/// Your nearest fleet out in the open in the patrol's system, within
/// detection range and without a contact pending.
fn nearest_fleet(
    state: &TravelState,
    player: PlayerId,
    patrol: &FactionPatrol,
    world: &StarSystem,
    systems: &[StarSystem],
) -> Option<(FleetId, LocalPosition)> {
    state
        .fleets
        .iter()
        .filter(|fleet| fleet.owner == player)
        .filter(|fleet| {
            !state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.fleet == fleet.id)
        })
        .filter_map(|fleet| {
            let at = exposed_at(state, fleet.id, world, systems)?;
            let gap = distance(patrol.position, at);
            (gap <= DETECTION_KM).then_some((gap, fleet.id, at))
        })
        .min_by_key(|(gap, fleet, _)| (*gap, fleet.0))
        .map(|(_, fleet, at)| (fleet, at))
}

/// The nearest raider band in the patrol's system that is hunting a fleet or
/// stalking a trader, within detection range. A band whose strike on you is
/// still pending is left to you.
fn nearest_band(state: &TravelState, patrol: &FactionPatrol) -> Option<(u64, LocalPosition)> {
    let navigation = &state.navigation;
    let engaged = |band: u64| {
        navigation
            .encounters
            .iter()
            .any(|known| known.band == Some(band))
    };
    navigation
        .bands
        .iter()
        .filter(|band| band.system == patrol.system && band.transit.is_none())
        .filter(|band| band.hunting.is_some() || band.stalking.is_some())
        .filter(|band| !engaged(band.id))
        .map(|band| {
            (
                distance(patrol.position, band.position),
                band.id,
                band.position,
            )
        })
        .filter(|(gap, _, _)| *gap <= DETECTION_KM)
        .min_by_key(|(gap, band, _)| (*gap, *band))
        .map(|(_, band, at)| (band, at))
}

/// Your flagship ran `patrol` down (see `pursuit`): a `Police` contact you
/// started, with no fine at stake.
pub(crate) fn intercept(
    state: &mut TravelState,
    patrol: &FactionPatrol,
    fleet: FleetId,
) -> Result<NavigationEvent, FleetMovementError> {
    stop_fleet(state, patrol, fleet, true)
}

fn stop_fleet(
    state: &mut TravelState,
    patrol: &FactionPatrol,
    fleet: FleetId,
    intercepted: bool,
) -> Result<NavigationEvent, FleetMovementError> {
    let navigation = &mut state.navigation;
    let next = navigation
        .next_encounter_id
        .checked_add(1)
        .ok_or(TravelError::TimeOverflow)?;
    let encounter = Encounter {
        id: navigation.next_encounter_id,
        fleet,
        kind: EncounterKind::Police,
        run: navigation.run,
        hostiles: patrol.ships.clone(),
        band: None,
        haven: false,
        trader: None,
        police: Some(PoliceContact {
            patrol: patrol.id,
            faction: patrol.faction,
            intercepted,
        }),
        rescue: None,
    };
    navigation.next_encounter_id = next;
    navigation.encounters.push(encounter.clone());
    Ok(NavigationEvent::EncounterFound(encounter))
}

/// Auto-resolves the patrol against a raider band. Both sides lose their
/// destroyed ships; the band backs off, the patrol rests. Returns news when
/// the band was hunting one of your fleets.
fn fight_band(
    state: &mut TravelState,
    economy: &Economy,
    player: PlayerId,
    patrol_id: u64,
    band_id: u64,
    world: &StarSystem,
) -> Result<Option<String>, FleetMovementError> {
    let (Some(patrol), Some(band)) = (
        state.navigation.police.patrol(patrol_id).cloned(),
        state
            .navigation
            .bands
            .iter()
            .find(|band| band.id == band_id)
            .cloned(),
    ) else {
        return Ok(None);
    };
    let rules = &economy.rules;
    let setup = |side, name: String, kind: ShipKind| {
        let profile = rules
            .ship_catalog
            .spec(kind)
            .map(|spec| spec.combat.clone())
            .ok_or(FleetError::UnknownClass)?;
        Ok::<_, FleetError>(CombatantSetup {
            name,
            side,
            ship: None,
            kind,
            profile,
            hull_damage: 0,
            gear: Vec::new(),
        })
    };
    let mut setups = Vec::new();
    for (index, kind) in patrol.ships.iter().enumerate() {
        let name = format!("{} {} {}", patrol.faction.label(), kind.label(), index + 1);
        setups.push(setup(Side::Player, name, *kind)?);
    }
    for (index, kind) in band.ships.iter().enumerate() {
        let name = format!("Raider {} {}", kind.label(), index + 1);
        setups.push(setup(Side::Hostile, name, *kind)?);
    }
    if setups.len() > MAX_COMBATANTS {
        return Ok(None);
    }
    let seed = state.band_roll();
    let result = tactical::auto_resolve(&setups, &rules.combat_catalog, seed)
        .map_err(|_| FleetError::InvalidSelection)?;
    let lost = |side| -> Vec<ShipKind> {
        result
            .combatants
            .iter()
            .filter(|unit| unit.side == side && unit.destroyed)
            .map(|unit| unit.kind)
            .collect()
    };
    let (police_lost, raiders_lost) = (lost(Side::Player), lost(Side::Hostile));
    state.band_after_contact(band_id, &raiders_lost);
    state.patrol_losses(patrol_id, &police_lost, BAND_FIGHT_COOLDOWN_RUNS);
    let hunted = band.hunting.and_then(|fleet| {
        state
            .fleets
            .iter()
            .find(|record| record.id == fleet && record.owner == player)
    });
    Ok(hunted.map(|fleet| {
        format!(
            "{} police engaged the raiders hunting {} in {}: {} raider(s) destroyed, {} patrol ship(s) lost.",
            patrol.faction.label(),
            fleet.name,
            world.name,
            raiders_lost.len(),
            police_lost.len()
        )
    }))
}
