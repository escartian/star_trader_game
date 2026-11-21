//! Raider bands crossing between systems (`navigation::band_roaming`).
use super::{Session, band_tests::run, combat_tests::reload, navigation_tests::succeed};
use game_core::{
    BodyId, LocalPosition, SystemId, fleet::FleetId, navigation::MAX_BANDS_PER_SYSTEM,
    travel::Location,
};
use game_protocol::ClientCommand;

/// The starting freighter just inside Meridian's (system 2) south-west corner,
/// facing both home and Far Reach, hunted by the band beside it, with a
/// course plotted to `destination`.
/// Peaceful, so nothing strikes and the voyage runs uninterrupted.
fn fleeing_to(destination: SystemId) -> Session {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    let edge = session.state.movement_rules.system_half_extent_km - 10_000_000;
    let at = LocalPosition {
        x_km: -edge,
        y_km: 0,
        z_km: -edge,
    };
    let ship = &mut session.state.ships[0];
    ship.location = Location::InSystem {
        system: SystemId(2),
        position: at,
    };
    // Long range: Far Reach lies 29 ly from Meridian.
    (ship.fuel, ship.fuel_capacity, ship.range_ly) = (100, 100, 40);
    let band = &mut session.state.navigation.bands[0];
    assert_eq!(band.system, SystemId(2));
    band.position = LocalPosition {
        x_km: at.x_km + 20_000_000,
        ..at
    };
    band.heading = at;
    band.hunting = Some(FleetId(1));
    let (revision, ship) = (session.state.revision, session.state.ships[0].id);
    succeed(
        &mut session,
        ClientCommand::PlotDestination {
            ship,
            system: destination,
            body: Some(BodyId(2)),
            expected_revision: revision,
        },
    );
    session
}

#[test]
fn a_band_follows_a_fleet_that_flees_its_system_and_arrives_later() {
    let mut session = fleeing_to(SystemId(3));
    let id = session.state.navigation.bands[0].id;
    let band = |session: &Session| {
        session
            .state
            .navigation
            .bands
            .iter()
            .find(|band| band.id == id)
            .cloned()
            .unwrap()
    };
    for _ in 0..3 {
        if band(&session).transit.is_some() {
            break;
        }
        run(&mut session);
    }
    let chasing = band(&session);
    let transit = chasing.transit.expect("the band gives chase");
    assert_eq!(chasing.system, SystemId(3));
    assert_eq!(transit.from, SystemId(2));
    assert_eq!(transit.total_runs, 15, "29 ly at 2 ly a run");
    assert_eq!(chasing.hunting, Some(FleetId(1)), "the fleet is warned");
    reload(&session);
    // The fleet then waits safely at home while the band crosses.
    session.state.navigation.orders.clear();
    session.state.ships[0].location = Location::Docked(SystemId(1));
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    for _ in 0..transit.runs_left {
        run(&mut session);
    }
    let arrived = band(&session);
    assert_eq!(arrived.system, SystemId(3));
    assert!(arrived.transit.is_none());
    // It arrives on Far Reach's face toward Meridian (north-east of the star).
    assert!(arrived.position.x_km > 0 && arrived.position.z_km > 0);
    reload(&session);
}

#[test]
fn a_band_never_follows_a_fleet_home() {
    let mut session = fleeing_to(SystemId(1));
    let id = session.state.navigation.bands[0].id;
    run(&mut session);
    let band = session
        .state
        .navigation
        .bands
        .iter()
        .find(|band| band.id == id)
        .unwrap();
    assert_ne!(band.system, SystemId(1));
    assert!(band.transit.is_none(), "it loses the trail");
    assert_eq!(band.hunting, None);
}

#[test]
fn idle_bands_roam_but_stay_out_of_home_and_full_systems() {
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
    let mut crossings = 0;
    for _ in 0..400 {
        let before = session.state.navigation.bands.clone();
        run(&mut session);
        let bands = &session.state.navigation.bands;
        for new in bands {
            assert_ne!(new.system, SystemId(1), "home is never raided");
            let crowd = bands.iter().filter(|other| other.system == new.system);
            assert!(crowd.count() <= MAX_BANDS_PER_SYSTEM);
            assert!(
                [new.position.x_km, new.position.y_km, new.position.z_km]
                    .iter()
                    .all(|value| value.abs() <= half)
            );
            let Some(old) = before.iter().find(|old| old.id == new.id) else {
                continue;
            };
            if old.system != new.system {
                crossings += 1;
                let transit = new.transit.expect("crossing bands are in transit");
                assert_eq!(transit.from, old.system);
                assert_eq!(transit.runs_left, transit.total_runs);
            } else if old.transit.is_none() {
                let gap = [
                    (old.position.x_km, new.position.x_km),
                    (old.position.z_km, new.position.z_km),
                ]
                .iter()
                .map(|(a, b)| ((b - a) as f64).powi(2))
                .sum::<f64>()
                .sqrt();
                assert!(gap <= 100_000_001.0, "moved {gap} km in one run");
            }
        }
    }
    assert!(crossings > 0, "bands roam over 400 runs");
    reload(&session);
}
