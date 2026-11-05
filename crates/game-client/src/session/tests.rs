use super::*;
use game_core::travel::ShipId;

#[test]
fn offline_clock_preserves_fractional_ticks_and_arrives_without_player_turns() {
    let mut host = LocalSession::new(game_server::Session::default());
    let start = host.clock;
    let message = host.authority.handle(ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
        expected_revision: 0,
    });
    host.accept(message);
    host.advance_to(start + Duration::from_millis(14_900));
    assert_eq!(host.state.tick, 14);
    assert!(matches!(host.state.ship.location, Location::Travelling(_)));
    host.advance_to(start + Duration::from_secs(15));
    assert_eq!(host.state.ship.location, Location::Docked(SystemId(2)));
    assert_eq!(host.state.tick, 15);
    assert_eq!(host.state.ship.fuel, 25);
    let original = host.state.clone();
    host.advance_to(start + Duration::from_secs(15));
    assert_eq!(host.state, original);
}
