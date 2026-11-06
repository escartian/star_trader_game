use super::{Session, test_support::execute_quiet_run};
use game_core::{
    BodyId, SystemId,
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

#[test]
fn map_destination_commands_plot_a_complete_itinerary_and_survive_each_run() {
    let mut session = Session::default();
    let initial = session.snapshot(Vec::new());
    assert!(
        matches!(session.handle(ClientCommand::PreviewDestination { ship: ShipId(1), system: SystemId(2), body: Some(BodyId(3)) }), ServerMessage::DestinationPreview(course) if course.legs.len() >= 3)
    );
    assert_eq!(session.snapshot(Vec::new()), initial);
    assert!(matches!(
        session.handle(ClientCommand::PlotDestination {
            ship: ShipId(1),
            system: SystemId(2),
            body: Some(BodyId(3)),
            expected_revision: 0
        }),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.state.ships[0].fuel, 40);
    assert!(session.state.navigation.orders[0].remaining_waypoints.len() >= 2);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("route.json");
    for _ in 0..32 {
        if session.state.navigation.orders.is_empty() {
            break;
        }
        execute_quiet_run(&mut session);
        session.save(&path).unwrap();
        let (restored, _) = Session::load(&path).unwrap();
        assert_eq!(restored.state, session.state);
        assert_eq!(restored.economy, session.economy);
    }
    assert!(session.state.navigation.orders.is_empty());
    assert_eq!(
        session.state.ships[0].location,
        Location::Docked(SystemId(2))
    );
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::PlotDestination {
            ship: ShipId(1),
            system: SystemId(1),
            body: Some(BodyId(999)),
            expected_revision: session.state.revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
}
