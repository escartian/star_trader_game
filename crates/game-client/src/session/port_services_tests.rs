use super::FleetService;
use crate::session::LocalSession;
use game_core::{BodyId, SystemId, fleet::ShipKind, movement::MoveTarget, travel::ShipId};
use game_protocol::ClientCommand;

#[test]
fn fleet_refuel_fills_every_ship_docked_here() {
    let mut host = LocalSession::new(game_server::Session::default());
    host.command(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    });
    host.command(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
        expected_revision: host.state.revision,
    });
    host.request_execute_run();
    assert_eq!(host.ships_here(ShipId(1)), vec![ShipId(1), ShipId(2)]);
    assert!(host.state.ships.iter().all(|s| s.fuel < s.fuel_capacity));
    host.run_fleet_service(ShipId(1), FleetService::Refuel);
    assert!(host.state.ships.iter().all(|s| s.fuel == s.fuel_capacity));
    // Nothing is damaged, so a fleet repair sends nothing and changes nothing.
    let before = host.state.clone();
    host.run_fleet_service(ShipId(1), FleetService::Repair);
    assert_eq!(host.state, before);
}
