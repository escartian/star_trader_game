//! Trade routes through the authoritative command interface.
use super::{LOCAL_PLAYER, Session};
use crate::navigation_tests::{run, succeed};
use game_core::{
    SystemId,
    economy::MarketId,
    fleet::FleetId,
    navigation::{NavigationError, NavigationEvent},
};
use game_protocol::{ClientCommand, ServerMessage, SessionEvent};

fn session(encounter_chance_percent: u32) -> Session {
    let mut session = Session::default();
    session.set_encounter_chance(encounter_chance_percent);
    session
}

/// Starts Fleet 1 on a route from Sol's port to Meridian's port.
fn start_route(session: &mut Session) -> MarketId {
    let destination = session
        .economy
        .markets
        .iter()
        .find(|market| market.id.system == SystemId(2) && market.is_station)
        .unwrap()
        .id;
    succeed(
        session,
        ClientCommand::SetTradeRoute {
            fleet: FleetId(1),
            destination,
            expected_revision: session.state.revision,
        },
    );
    destination
}

fn navigation_events(response: ServerMessage) -> Vec<NavigationEvent> {
    let ServerMessage::SessionSnapshot { events, .. } = response else {
        panic!("{response:?}");
    };
    events
        .into_iter()
        .filter_map(|event| match event {
            SessionEvent::Navigation(event) => Some(event),
            _ => None,
        })
        .collect()
}

fn run_events(session: &mut Session) -> Vec<NavigationEvent> {
    let revision = session.state.revision;
    navigation_events(session.handle(ClientCommand::ExecuteNavigationRun {
        expected_revision: revision,
    }))
}

#[test]
fn a_route_loops_through_runs_without_asking_for_orders_and_survives_saving() {
    let mut session = session(0);
    start_route(&mut session);
    let route = session.state.navigation.routes.clone();
    assert_eq!(route.len(), 1);
    let mut stops = 0;
    for _ in 0..60 {
        stops += run_events(&mut session)
            .iter()
            .filter(|event| matches!(event, NavigationEvent::RouteServiced { .. }))
            .count();
        assert_eq!(session.state.navigation_attention(LOCAL_PLAYER), None);
        if stops == 2 {
            break;
        }
    }
    assert_eq!(stops, 2, "traded at Meridian, then back at Sol");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("route.json");
    session.save(&path).unwrap();
    let (loaded, _) = Session::load(&path).unwrap();
    assert_eq!(loaded.state.navigation.routes, route);
}

#[test]
fn new_orders_for_a_route_fleet_end_its_route() {
    let mut session = session(0);
    start_route(&mut session);
    let revision = session.state.revision;
    let events = navigation_events(session.handle(ClientCommand::HoldFleet {
        fleet: FleetId(1),
        expected_revision: revision,
    }));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, NavigationEvent::RouteEnded { .. })),
        "{events:?}"
    );
    assert!(session.state.navigation.routes.is_empty());
    assert_eq!(session.state.navigation.holds, [FleetId(1)]);
}

#[test]
fn cancelling_a_route_lets_the_current_leg_finish_then_asks_for_orders() {
    let mut session = session(0);
    start_route(&mut session);
    run(&mut session);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::CancelTradeRoute {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    assert!(session.state.navigation.routes.is_empty());
    for _ in 0..30 {
        if session.state.navigation.order(FleetId(1)).is_none() {
            break;
        }
        run(&mut session);
    }
    assert_eq!(session.state.ships[0].location.system(), Some(SystemId(2)));
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(1))
    );
}

#[test]
fn route_fleets_meet_contacts_and_wait_for_a_decision_like_any_fleet() {
    let mut session = session(100);
    start_route(&mut session);
    let events = run_events(&mut session);
    assert!(
        events.iter().any(|event| matches!(
            event,
            NavigationEvent::EncounterFound(found) if found.fleet == FleetId(1)
        )),
        "{events:?}"
    );
    assert_eq!(
        session.state.navigation.routes.len(),
        1,
        "the route survives"
    );
    let revision = session.state.revision;
    assert_eq!(
        session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: revision
        }),
        ServerMessage::CommandRejected(NavigationError::EncounterPending(FleetId(1)).into())
    );
}
