use super::{LocalSession, advance::RUN_INTERVAL_SECONDS};
use game_core::{SystemId, fleet::FleetId, travel::ShipId};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

fn session(encounter_chance_percent: u32) -> LocalSession {
    let mut authority = game_server::Session::default();
    authority.set_encounter_chance(encounter_chance_percent);
    LocalSession::new(authority)
}

/// Plots the starting fleet's whole route into system 2: departure, deep space, entry.
fn plot_to_neighbour(host: &mut LocalSession) {
    host.command(ClientCommand::PlotDestination {
        ship: ShipId(1),
        system: SystemId(2),
        body: None,
        expected_revision: host.state.revision,
    });
    assert!(
        host.state.navigation.order(FleetId(1)).is_some(),
        "{}",
        host.notice
    );
}

/// Drives an advance with a simulated UI clock until it stops; returns the runs executed.
fn drive_to_stop(host: &mut LocalSession) -> u64 {
    let start = host.state.navigation.run;
    let mut now = 0.0;
    for _ in 0..1_000 {
        match host.drive_advance(now) {
            Some(delay) => now += delay,
            None => return host.state.navigation.run - start,
        }
    }
    panic!("advance did not stop");
}

#[test]
fn advance_follows_a_multi_leg_route_to_arrival_then_asks_for_orders() {
    let mut host = session(0);
    plot_to_neighbour(&mut host);
    host.toggle_advance();
    assert!(host.is_advancing());
    let runs = drive_to_stop(&mut host);
    assert!(!host.is_advancing());
    assert!(
        runs > 2,
        "a route into another system spans several runs: {runs}"
    );
    assert_eq!(host.state.navigation.order(FleetId(1)), None);
    assert_eq!(host.selected_ship().location.system(), Some(SystemId(2)));
    assert_eq!(
        host.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(1))
    );
    assert!(host.notice.contains("needs orders"), "{}", host.notice);
    assert!(host.take_focus_request());
}

#[test]
fn advance_stops_on_the_run_that_finds_a_contact() {
    let mut host = session(100);
    plot_to_neighbour(&mut host);
    host.toggle_advance();
    assert_eq!(drive_to_stop(&mut host), 1);
    assert_eq!(host.contact_fleets(), [FleetId(1)]);
    assert!(host.notice.contains("decision needed"), "{}", host.notice);
    assert!(host.take_focus_request());
}

#[test]
fn advance_with_every_fleet_waiting_executes_a_single_run() {
    let mut host = session(0);
    host.command(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: host.state.revision,
    });
    host.toggle_advance();
    assert_eq!(drive_to_stop(&mut host), 1);
    assert!(
        host.notice.contains("no fleet is underway"),
        "{}",
        host.notice
    );
    assert_eq!(host.state.navigation.holds, [FleetId(1)]);
}

#[test]
fn advance_does_not_start_while_a_fleet_needs_orders() {
    let mut host = session(0);
    host.toggle_advance();
    assert!(!host.is_advancing());
    assert_eq!(host.drive_advance(1.0), None);
    assert_eq!(host.state.navigation.run, 0);
    assert!(host.notice.contains("Wait order"), "{}", host.notice);
}

#[test]
fn advance_paces_runs_and_stops_on_request() {
    let mut host = session(0);
    plot_to_neighbour(&mut host);
    host.toggle_advance();
    assert!(host.drive_advance(10.0).is_some());
    assert_eq!(host.state.navigation.run, 1);
    assert!(host.drive_advance(10.1).is_some());
    assert_eq!(
        host.state.navigation.run, 1,
        "the next run waits for the interval"
    );
    assert!(host.drive_advance(10.0 + RUN_INTERVAL_SECONDS).is_some());
    assert_eq!(host.state.navigation.run, 2);
    host.toggle_advance();
    assert!(!host.is_advancing());
    assert_eq!(host.drive_advance(100.0), None);
    assert_eq!(host.state.navigation.run, 2);
    assert!(
        host.notice.contains("stopped after 2 runs"),
        "{}",
        host.notice
    );
}

#[test]
fn loading_a_save_cancels_an_advance_in_progress() {
    let directory = tempfile::tempdir().unwrap();
    let mut host = session(0);
    host.set_save_path(directory.path().join("slot.json"));
    host.autosave().unwrap();
    plot_to_neighbour(&mut host);
    host.toggle_advance();
    host.load_slot();
    assert!(!host.is_advancing());
    assert_eq!(host.drive_advance(5.0), None);
    assert_eq!(host.state.navigation.run, 0);
}

#[test]
fn advance_keeps_mining_until_the_holds_fill_then_stops() {
    use game_core::{BodyId, economy::Commodity, movement::MoveTarget};
    let mut host = session(0);
    host.command(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(5),
        },
        expected_revision: host.state.revision,
    });
    host.toggle_advance();
    drive_to_stop(&mut host);
    host.command(ClientCommand::Land {
        fleet: FleetId(1),
        expected_revision: host.state.revision,
    });
    host.command(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: host.state.revision,
    });
    assert_eq!(host.mining_with_room(), vec![FleetId(1)], "{}", host.notice);
    host.toggle_advance();
    let runs = drive_to_stop(&mut host);
    let ship = &host.state.ships[0];
    assert_eq!(
        ship.cargo[Commodity::Ore.index()],
        ship.cargo_capacity,
        "{:?}",
        host.log
    );
    assert_eq!(u64::from(ship.cargo_capacity / 2), runs, "2 ore per run");
    assert!(host.notice.contains("holds full"), "{}", host.notice);
}
