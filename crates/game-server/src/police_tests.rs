//! Faction police and wanted status (`game_core::police`,
//! `game_core::factions::wanted`, `standing`).
use super::{
    GameSetup, Session,
    band_tests::run,
    combat_tests::{choose, reload},
    faction_tests::{claimed, standing_events},
    navigation_tests::{succeed, two_fleets},
    piracy_tests::{hailed, meet_trader},
};
use game_core::{
    LocalPosition, SystemId,
    economy::{Commodity, EconomyError, EconomyRules, TradeSide},
    factions::Faction,
    fleet::FleetId,
    navigation::{EncounterChoice, EncounterKind, NavigationEvent},
    police::{self, BAND_FIGHT_COOLDOWN_RUNS, COOLDOWN_RUNS},
    travel::Location,
};
use game_protocol::{ClientCommand, CommandError, ServerMessage, SessionEvent};

const FEDERATION_WANTED_RUNS: u32 = 40;

/// System 2 held by the Federation with its patrol, no raider bands, and
/// `fleet` waiting in the open 2 million km from the patrol; wanted by the
/// Federation for `wanted` runs.
pub(super) fn policed(mut session: Session, fleet: FleetId, wanted: u32) -> Session {
    session.economy.factions = claimed([None, Some(Faction::Federation), None])
        .economy
        .factions;
    session.economy.factions.wanted[Faction::Federation.index()] = wanted;
    session.state.navigation.bands.clear();
    session
        .state
        .spawn_police(&session.economy.factions, &session.systems);
    let patrol = session.state.navigation.police.patrols[0].clone();
    assert_eq!(patrol.system, SystemId(2));
    let members = session
        .state
        .fleets
        .iter()
        .find(|record| record.id == fleet)
        .unwrap()
        .ships
        .clone();
    for id in members {
        session.state.ship_mut(id).unwrap().location = Location::InSystem {
            system: SystemId(2),
            position: LocalPosition {
                z_km: patrol.position.z_km + 2_000_000,
                ..patrol.position
            },
        };
    }
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet,
            expected_revision: revision,
        },
    );
    session
}

fn wanted_events(message: &ServerMessage) -> Vec<(Faction, u32)> {
    let ServerMessage::SessionSnapshot { events, .. } = message else {
        return Vec::new();
    };
    events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::WantedChanged { faction, runs } => Some((*faction, *runs)),
            _ => None,
        })
        .collect()
}

fn stopped(session: &mut Session) -> u64 {
    let events = run(session);
    let found = events.iter().find_map(|event| match event {
        NavigationEvent::EncounterFound(found) if found.kind == EncounterKind::Police => {
            Some(found.clone())
        }
        _ => None,
    });
    let contact = found.expect("the patrol stopped the fleet");
    assert_eq!(contact.police.unwrap().faction, Faction::Federation);
    contact.id
}

#[test]
fn faction_setup_places_a_patrol_in_every_claimed_system_that_saves_keep() {
    let session = Session::new_game(
        3,
        EconomyRules::default(),
        Default::default(),
        &GameSetup::default(),
    )
    .unwrap();
    let factions = &session.economy.factions;
    let patrols = &session.state.navigation.police.patrols;
    assert_eq!(patrols.len(), factions.territory.len());
    assert!(
        patrols
            .iter()
            .all(|patrol| factions.owner(patrol.system) == Some(patrol.faction))
    );
    let restored = reload(&session);
    assert_eq!(
        restored.state.navigation.police,
        session.state.navigation.police
    );
    // A patrol outside its faction's territory is not a legal save.
    let mut forged = session.state.clone();
    let other = Faction::ALL
        .into_iter()
        .find(|faction| {
            Some(*faction) != factions.owner(forged.navigation.police.patrols[0].system)
        })
        .unwrap();
    forged.navigation.police.patrols[0].faction = other;
    assert!(forged.validate_police_factions(factions).is_err());
}

#[test]
fn piracy_makes_you_wanted_and_its_ports_refuse_you_until_you_pay() {
    let mut session = Session::default();
    let (fleet, trader) = meet_trader(&mut session, 0);
    let system = session.state.ships[0].location.system().unwrap();
    let mut holders = [None; 3];
    holders[system.0 as usize - 1] = Some(Faction::Federation);
    session.economy.factions = claimed(holders).economy.factions;
    let contact = hailed(&mut session, fleet, trader);
    let response = choose(&mut session, contact, EncounterChoice::AutoResolve);
    let factions = &session.economy.factions;
    assert_eq!(
        factions.wanted_runs(Faction::Federation),
        FEDERATION_WANTED_RUNS
    );
    assert!(wanted_events(&response).contains(&(Faction::Federation, FEDERATION_WANTED_RUNS)));
    if session.state.campaign.ended.is_some() {
        return; // The trader won; nobody is left to trade.
    }
    let ship = session.state.ships[0].clone();
    let quote = session
        .economy
        .quote(&ship, Commodity::Ore, 1, TradeSide::Buy, 0);
    assert_eq!(quote.map(|_| ()), Err(EconomyError::PortClosed));
    let catalog = &session.economy.rules.faction_catalog;
    let base = session
        .economy
        .factions
        .fine_cents(Faction::Federation, catalog);
    assert_eq!(base, 40 * 1_500, "a run's fine for each wanted run");
    let fine = police::fine(&session.economy, Faction::Federation);
    assert!(fine.0 >= base, "fines rise with prices");
    session.economy.credits.0 = fine.0 + 1;
    let market = session.economy.current_market(&ship).unwrap().id;
    let treasury = session.economy.market(market).unwrap().credits.0;
    let response = session.handle(ClientCommand::PayFine {
        ship: ship.id,
        expected_revision: session.state.revision,
    });
    assert!(wanted_events(&response).contains(&(Faction::Federation, 0)));
    assert_eq!(session.economy.credits.0, 1);
    assert_eq!(
        session.economy.market(market).unwrap().credits.0,
        treasury + fine.0
    );
    let ship = session.state.ships[0].clone();
    let after = session
        .economy
        .quote(&ship, Commodity::Ore, 1, TradeSide::Buy, 0);
    assert_ne!(after.map(|_| ()), Err(EconomyError::PortClosed));
    // Nothing left to pay.
    let response = session.handle(ClientCommand::PayFine {
        ship: ship.id,
        expected_revision: session.state.revision,
    });
    assert!(matches!(
        response,
        ServerMessage::CommandRejected(CommandError::Economy(EconomyError::NotWanted))
    ));
}

#[test]
fn a_wanted_fleet_is_stopped_and_complying_pays_the_fine() {
    let mut session = policed(Session::default(), FleetId(1), 30);
    let contact = stopped(&mut session);
    // The pending contact saves like any other.
    let mut session = reload(&session);
    let catalog = &session.economy.rules.faction_catalog;
    let base = session
        .economy
        .factions
        .fine_cents(Faction::Federation, catalog);
    assert_eq!(base, 29 * 1_500, "one wanted run already ran down");
    let fine = police::fine(&session.economy, Faction::Federation);
    let before = session.economy.credits.0;
    let response = choose(&mut session, contact, EncounterChoice::Comply);
    assert!(wanted_events(&response).contains(&(Faction::Federation, 0)));
    assert_eq!(session.economy.credits.0, before - fine.0);
    assert!(session.state.navigation.encounters.is_empty());
    assert_eq!(
        session.state.navigation.police.patrols[0].cooldown,
        COOLDOWN_RUNS
    );
    // Lawful again: the patrol lets the fleet be.
    for _ in 0..5 {
        assert!(!run(&mut session).iter().any(|event| matches!(
            event,
            NavigationEvent::EncounterFound(found) if found.kind == EncounterKind::Police
        )));
    }
    reload(&session);
}

#[test]
fn escaping_police_after_a_seizure_advances_pirate_progress() {
    let mut session = policed(Session::default(), FleetId(1), 30);
    session.state.navigation.piracies = 1;
    session.state.navigation.pirate_seizures = 1;
    session.state.navigation.pirate_cargo = 8;
    let contact = stopped(&mut session);
    let escaped = (0..100).find_map(|seed| {
        let mut attempt = session.clone();
        attempt.state.navigation.rng_state = seed;
        choose(&mut attempt, contact, EncounterChoice::Evade);
        attempt
            .state
            .navigation
            .encounters
            .is_empty()
            .then_some(attempt)
    });
    let escaped = escaped.expect("at least one deterministic seed evades the patrol");
    assert_eq!(escaped.state.navigation.pirate_escapes, 1);
    assert_eq!(reload(&escaped).state.navigation.pirate_escapes, 1);
}

#[test]
fn complying_short_of_the_fine_impounds_the_cargo() {
    let mut session = policed(Session::default(), FleetId(1), 30);
    session.state.ships[0].cargo[Commodity::Ore.index()] = 5;
    session.economy.credits.0 = 100;
    let contact = stopped(&mut session);
    choose(&mut session, contact, EncounterChoice::Comply);
    assert_eq!(session.economy.credits.0, 0);
    assert_eq!(session.state.ships[0].cargo, [0; 6]);
    assert!(!session.economy.factions.is_wanted(Faction::Federation));
}

#[test]
fn resisting_arrest_is_a_crime_and_destroyed_police_cost_standing() {
    let mut session = policed(Session::default(), FleetId(1), 30);
    session.state.ships[0].kind = game_core::fleet::ShipKind::Patrol;
    let contact = stopped(&mut session);
    let response = choose(&mut session, contact, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    assert_eq!(report.police, Some(Faction::Federation));
    assert_eq!(report.bounty.0, 0, "no bounty for police");
    let destroyed = report
        .side(game_core::combat::Side::Hostile)
        .filter(|ship| ship.destroyed)
        .count() as i32;
    let factions = &session.economy.factions;
    assert_eq!(factions.standing(Faction::Federation), -25 * destroyed);
    if destroyed > 0 {
        assert!(standing_events(&response).contains(&(Faction::Federation, -25 * destroyed)));
    }
    if report.outcome != game_core::combat::CombatOutcome::Surrendered {
        assert_eq!(
            factions.wanted_runs(Faction::Federation),
            29 + FEDERATION_WANTED_RUNS,
            "fighting adds a crime"
        );
    }
    let patrols = &session.state.navigation.police;
    let survivors = patrols
        .patrols
        .first()
        .map_or(0, |patrol| patrol.ships.len());
    assert_eq!(survivors + destroyed as usize, 2);
    assert_eq!(patrols.respawns.len(), usize::from(survivors == 0));
    reload(&session);
}

#[test]
fn police_leave_lawful_fleets_be_and_engage_raiders_hunting_them() {
    let mut session = policed(Session::default(), FleetId(1), 0);
    let patrol = session.state.navigation.police.patrols[0].clone();
    // A band 140 million km down the line from the fleet: it closes on the
    // fleet, and the patrol between them meets it.
    let mut band = Session::default().state.navigation.bands[0].clone();
    assert_eq!(band.system, SystemId(2));
    band.position = LocalPosition {
        z_km: patrol.position.z_km - 138_000_000,
        ..patrol.position
    };
    band.heading = band.position;
    session.state.navigation.bands = vec![band.clone()];
    session.state.navigation.police.patrols[0].position = LocalPosition {
        z_km: patrol.position.z_km - 60_000_000,
        ..patrol.position
    };
    let events = run(&mut session);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, NavigationEvent::EncounterFound(_)))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NavigationEvent::PoliceNote(note) if note.contains("Federation police"))),
        "{events:?}"
    );
    let bands = &session.state.navigation.bands;
    assert!(bands.is_empty() || bands[0].cooldown > 0 && bands[0].hunting.is_none());
    let police = &session.state.navigation.police;
    assert!(
        police.patrols.is_empty()
            || police.patrols[0].cooldown == BAND_FIGHT_COOLDOWN_RUNS
                && police.patrols[0].answering.is_none()
    );
    reload(&session);
}

#[test]
fn wanted_status_runs_down_and_its_end_is_announced() {
    let mut session = claimed([None, Some(Faction::Federation), None]);
    session
        .state
        .spawn_police(&session.economy.factions, &session.systems);
    session.economy.factions.wanted[Faction::Federation.index()] = 2;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    let quiet = |session: &mut Session| {
        session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision,
        })
    };
    assert!(
        wanted_events(&quiet(&mut session)).is_empty(),
        "2 → 1 is not news"
    );
    assert_eq!(
        wanted_events(&quiet(&mut session)),
        vec![(Faction::Federation, 0)]
    );
}

#[test]
fn secondary_fleets_submit_to_the_police() {
    let mut session = two_fleets();
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    let mut session = policed(session, FleetId(2), 30);
    let before = session.economy.credits.0;
    let events = run(&mut session);
    assert!(events.iter().any(|event| matches!(
        event,
        NavigationEvent::EncounterResolved { fleet: FleetId(2), outcome } if outcome.contains("fine")
    )));
    assert!(session.state.navigation.encounters.is_empty());
    assert!(session.economy.credits.0 < before);
    assert!(!session.economy.factions.is_wanted(Faction::Federation));
}
