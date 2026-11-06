use super::{LOCAL_PLAYER, Session};
use game_core::{
    BodyId, SystemId,
    economy::Commodity,
    fleet::{FleetId, ShipKind},
    movement::MoveTarget,
    travel::{Location, PlayerId, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn apply(session: &mut Session, command: ClientCommand) {
    assert!(matches!(
        session.handle_offline(command),
        ServerMessage::SessionSnapshot { .. }
    ));
}

fn buy(session: &mut Session) {
    apply(
        session,
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: session.state.revision,
        },
    );
}

#[test]
fn fleet_purchase_split_transfer_travel_and_merge_preserve_owned_state() {
    let mut session = Session::default();
    let initial_credits = session.economy.credits.0;
    buy(&mut session);
    assert_eq!(session.economy.credits.0, initial_credits - 45000);
    assert_eq!(session.state.ships.len(), 2);
    session.state.ships[0].cargo[Commodity::Food.index()] = 5;
    apply(
        &mut session,
        ClientCommand::TransferCargo {
            source: ShipId(1),
            target: ShipId(2),
            commodity: Commodity::Food,
            quantity: 3,
            expected_revision: 1,
        },
    );
    assert_eq!(session.state.ships[0].cargo[0], 2);
    assert_eq!(session.state.ships[1].cargo[0], 3);
    apply(
        &mut session,
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(2)],
            expected_revision: 2,
        },
    );
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.state.fleets.len(), 2);
    apply(
        &mut session,
        ClientCommand::Move {
            ship: ShipId(2),
            target: MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
            expected_revision: 3,
        },
    );
    crate::test_support::execute_quiet_run(&mut session);
    assert_eq!(session.state.tick, 4);
    assert_eq!(
        session.state.ships[0].location,
        Location::Docked(SystemId(1))
    );
    assert_eq!(
        session.state.ships[1].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
    let before = session.export_json().unwrap();
    assert!(matches!(
        session.handle_offline(ClientCommand::MergeFleets {
            source: FleetId(2),
            target: FleetId(1),
            expected_revision: session.state.revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.export_json().unwrap(), before);
    let revision = session.state.revision;
    apply(
        &mut session,
        ClientCommand::Move {
            ship: ShipId(1),
            target: MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
            expected_revision: revision,
        },
    );
    crate::test_support::execute_quiet_run(&mut session);
    let revision = session.state.revision;
    apply(
        &mut session,
        ClientCommand::MergeFleets {
            source: FleetId(2),
            target: FleetId(1),
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.fleets[0].ships, [ShipId(1), ShipId(2)]);
    assert_eq!(session.state.tick, 8);
    assert_eq!(session.state.ships[1].cargo[0], 3);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fleet.json");
    session.save(&path).unwrap();
    let (restored, _) = Session::load(&path).unwrap();
    assert_eq!(restored.state, session.state);
    assert_eq!(restored.economy, session.economy);
}

#[test]
fn fleet_failures_and_stale_retries_never_charge_or_advance_ticks() {
    let mut session = Session::default();
    buy(&mut session);
    session.state.ships[1].fuel = 0;
    let commands = [
        ClientCommand::Move {
            ship: ShipId(1),
            target: MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
            expected_revision: 1,
        },
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: 0,
        },
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(1), ShipId(2)],
            expected_revision: 1,
        },
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(2), ShipId(2)],
            expected_revision: 1,
        },
        ClientCommand::TransferCargo {
            source: ShipId(1),
            target: ShipId(2),
            commodity: Commodity::Food,
            quantity: 1,
            expected_revision: 1,
        },
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Patrol,
            expected_revision: 1,
        },
    ];
    let before = session.export_json().unwrap();
    for command in commands {
        assert!(matches!(
            session.handle_offline(command),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.export_json().unwrap(), before);
    }
    assert!(matches!(
        session.handle_offline_as(
            PlayerId(2),
            ClientCommand::SplitFleet {
                fleet: FleetId(1),
                ships: vec![ShipId(2)],
                expected_revision: 1
            }
        ),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.export_json().unwrap(), before);
    session.state.ships[1].fuel = 50;
    apply(
        &mut session,
        ClientCommand::Move {
            ship: ShipId(1),
            target: MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
            expected_revision: 1,
        },
    );
    crate::test_support::execute_quiet_run(&mut session);
    assert_eq!(session.state.tick, 4);
    assert!(session.state.ships.iter().all(|ship| ship.location
        == Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }));
    assert_eq!(
        session
            .state
            .fleet_position(FleetId(1), LOCAL_PLAYER)
            .unwrap(),
        &Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
}
