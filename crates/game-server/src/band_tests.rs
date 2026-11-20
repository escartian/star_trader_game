use super::{
    Session,
    combat_tests::{choose, reload},
    navigation_tests::succeed,
};
use game_core::{
    LocalPosition, SystemId,
    fleet::FleetId,
    navigation::{EncounterChoice, NavigationEvent},
    travel::Location,
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};

/// The starting freighter waiting in the open in system 2, `gap` km from the
/// band there.
fn near_band(gap: i64) -> Session {
    let mut session = Session::default();
    let band = session.state.navigation.bands[0].clone();
    assert_eq!(band.system, SystemId(2));
    session.state.ships[0].location = Location::InSystem {
        system: SystemId(2),
        position: LocalPosition {
            x_km: band.position.x_km,
            y_km: band.position.y_km + gap,
            z_km: band.position.z_km,
        },
    };
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    session
}

/// Executes a run and returns its navigation events.
fn run(session: &mut Session) -> Vec<NavigationEvent> {
    let ServerMessage::SessionSnapshot { events, .. } =
        session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision,
        })
    else {
        panic!("run rejected");
    };
    events
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Navigation(event) => Some(event),
            _ => None,
        })
        .collect()
}

fn struck(events: &[NavigationEvent]) -> bool {
    events.iter().any(
        |event| matches!(event, NavigationEvent::EncounterFound(encounter) if encounter.band.is_some()),
    )
}

#[test]
fn every_system_but_home_starts_with_a_band_that_survives_saving() {
    let session = Session::default();
    let systems: Vec<_> = session
        .state
        .navigation
        .bands
        .iter()
        .map(|band| band.system)
        .collect();
    assert_eq!(systems, vec![SystemId(2), SystemId(3)]);
    assert!(
        session
            .state
            .navigation
            .bands
            .iter()
            .all(|band| { !band.ships.is_empty() && band.ships.len() <= 2 && band.cooldown == 0 })
    );
    reload(&session);
}

#[test]
fn a_band_hunts_a_fleet_in_the_open_and_strikes_with_its_own_ships() {
    let mut session = near_band(120_000_000);
    let band = session.state.navigation.bands[0].clone();
    let mut contact = None;
    for _ in 0..4 {
        let events = run(&mut session);
        if struck(&events) {
            contact = session.state.navigation.encounters.first().cloned();
            break;
        }
        assert_eq!(
            session.state.navigation.bands[0].hunting,
            Some(FleetId(1)),
            "the band closes in"
        );
    }
    let contact = contact.expect("the band struck within four runs");
    assert_eq!(contact.band, Some(band.id));
    assert_eq!(contact.hostiles, band.ships);
    // A waiting fleet can be struck; saves accept the band's contact.
    reload(&session);
}

#[test]
fn docked_and_landed_fleets_are_safe_and_peaceful_games_never_strike() {
    let mut docked = near_band(1_000_000);
    docked.state.ships[0].location = Location::Docked(SystemId(2));
    let mut peaceful = near_band(1_000_000);
    peaceful.state.movement_rules.encounter_chance_percent = 0;
    for session in [&mut docked, &mut peaceful] {
        for _ in 0..6 {
            assert!(!struck(&run(session)));
        }
    }
}

#[test]
fn a_band_loses_destroyed_raiders_backs_off_and_returns_after_being_wiped_out() {
    let mut session = near_band(1_000_000);
    // A lethal player: every raider in reach dies in the first volley.
    session.state.ships[0].kind = game_core::fleet::ShipKind::Patrol;
    for _ in 0..4 {
        if struck(&run(&mut session)) {
            break;
        }
    }
    let contact = session.state.navigation.encounters[0].clone();
    choose(&mut session, contact.id, EncounterChoice::AutoResolve);
    let report = session.state.navigation.last_combat.clone().unwrap();
    let destroyed = report
        .side(game_core::combat::Side::Hostile)
        .filter(|ship| ship.destroyed)
        .count();
    let band_id = contact.band.unwrap();
    match session
        .state
        .navigation
        .bands
        .iter()
        .find(|band| band.id == band_id)
    {
        Some(band) => {
            assert_eq!(band.ships.len(), contact.hostiles.len() - destroyed);
            assert_eq!(band.cooldown, 3, "it backs off");
        }
        None => {
            assert_eq!(
                destroyed,
                contact.hostiles.len(),
                "only a wiped-out band vanishes"
            );
            let (system, due) = session.state.navigation.band_respawns[0];
            assert_eq!(system, SystemId(2));
            session.state.ships[0].location = Location::Docked(SystemId(1));
            while session.state.navigation.run < due {
                run(&mut session);
            }
            assert!(
                session
                    .state
                    .navigation
                    .bands
                    .iter()
                    .any(|band| band.system == SystemId(2)),
                "a new band arrives"
            );
        }
    }
    reload(&session);
}

#[test]
fn bands_move_at_most_one_run_of_distance_and_stay_in_their_system() {
    let mut session = Session::default();
    session.state.ships[0].location = Location::Docked(SystemId(1));
    let half = session.state.movement_rules.system_half_extent_km;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    for _ in 0..30 {
        let before = session.state.navigation.bands.clone();
        run(&mut session);
        for (old, new) in before.iter().zip(&session.state.navigation.bands) {
            let gap = [
                (old.position.x_km, new.position.x_km),
                (old.position.y_km, new.position.y_km),
                (old.position.z_km, new.position.z_km),
            ]
            .iter()
            .map(|(a, b)| ((b - a) as f64).powi(2))
            .sum::<f64>()
            .sqrt();
            assert!(gap <= 100_000_001.0, "moved {gap} km in one run");
            assert_eq!(old.system, new.system);
            assert!(
                [new.position.x_km, new.position.y_km, new.position.z_km]
                    .iter()
                    .all(|value| value.abs() <= half)
            );
        }
    }
}
