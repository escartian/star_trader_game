//! Fleet software (`game_core::fleet::software`).
use super::{Session, combat_tests::reload};
use game_core::{
    fleet::{FleetId, ShipKind, software::FleetSoftware},
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

fn install(session: &mut Session, ship: ShipId) -> ServerMessage {
    session.handle(ClientCommand::InstallSoftware {
        ship,
        software: FleetSoftware::PursuitSuite,
        expected_revision: session.state.revision,
    })
}

fn ok(message: ServerMessage) {
    assert!(
        matches!(message, ServerMessage::SessionSnapshot { .. }),
        "{message:?}"
    );
}

#[test]
fn software_is_bought_once_per_fleet_and_both_halves_keep_it_after_a_split() {
    let mut session = Session::default();
    session.economy.credits = game_core::economy::Credits(1_000_000);
    ok(session.handle(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    }));
    let credits = session.economy.credits.0;
    let price = session.economy.software_price(FleetSoftware::PursuitSuite);
    // Any ship of the fleet can order it; the whole fleet gets it.
    ok(install(&mut session, ShipId(2)));
    assert_eq!(
        session.state.fleets[0].software,
        vec![FleetSoftware::PursuitSuite]
    );
    assert_eq!(session.economy.credits.0, credits - price.0);
    assert!(matches!(
        install(&mut session, ShipId(1)),
        ServerMessage::CommandRejected(_)
    ));
    let reloaded = reload(&session);
    assert_eq!(
        reloaded.state.fleets[0].software,
        session.state.fleets[0].software
    );
    ok(session.handle(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: session.state.revision,
    }));
    assert!(
        session
            .state
            .fleets
            .iter()
            .all(|fleet| fleet.software == vec![FleetSoftware::PursuitSuite])
    );
}
