use super::*;

fn state(session: &mut Session) -> TravelState {
    let ServerMessage::SessionSnapshot { state, .. } = session.handle(ClientCommand::GetSession)
    else {
        panic!("session snapshot missing")
    };
    state
}

#[test]
fn snapshots_do_not_mutate_authority_or_advance_time() {
    let mut session = Session::default();
    let ServerMessage::GalaxySnapshot { mut systems, .. } =
        session.handle(ClientCommand::GetGalaxy)
    else {
        panic!("galaxy snapshot missing")
    };
    systems.clear();
    let mut copy = state(&mut session);
    copy.ship.fuel = 0;
    assert_eq!(copy.ship.fuel, 0);
    assert_eq!(state(&mut session), TravelState::default());
}

#[test]
fn rejected_commands_leave_the_authoritative_state_unchanged() {
    let mut session = Session::default();
    let initial = state(&mut session);
    for (player, command, error) in [
        (
            PlayerId(2),
            ClientCommand::Travel {
                ship: ShipId(1),
                destination: SystemId(2),
                expected_revision: 0,
            },
            TravelError::NotOwner,
        ),
        (
            LOCAL_PLAYER,
            ClientCommand::Travel {
                ship: ShipId(5),
                destination: SystemId(2),
                expected_revision: 0,
            },
            TravelError::UnknownShip,
        ),
        (
            LOCAL_PLAYER,
            ClientCommand::Travel {
                ship: ShipId(1),
                destination: SystemId(99),
                expected_revision: 0,
            },
            TravelError::UnknownSystem,
        ),
        (
            LOCAL_PLAYER,
            ClientCommand::Travel {
                ship: ShipId(1),
                destination: SystemId(1),
                expected_revision: 0,
            },
            TravelError::AlreadyThere,
        ),
        (
            LOCAL_PLAYER,
            ClientCommand::Travel {
                ship: ShipId(1),
                destination: SystemId(2),
                expected_revision: 1,
            },
            TravelError::StaleQuote,
        ),
    ] {
        assert_eq!(
            session.handle_as(player, command),
            ServerMessage::CommandRejected(error)
        );
        assert_eq!(state(&mut session), initial);
    }
}

#[test]
fn travel_arrives_only_after_the_host_advances_time_and_retries_do_not_charge_twice() {
    let mut session = Session::default();
    let command = ClientCommand::Travel {
        ship: ShipId(1),
        destination: SystemId(2),
        expected_revision: 0,
    };
    assert!(matches!(
        session.handle(command.clone()),
        ServerMessage::SessionSnapshot { .. }
    ));
    let underway = state(&mut session);
    assert_eq!(underway.ship.fuel, 25);
    assert_eq!(underway.tick, 0);
    assert_eq!(
        session.handle(command),
        ServerMessage::CommandRejected(TravelError::StaleQuote)
    );
    assert_eq!(state(&mut session), underway);
    session.advance(14);
    assert!(matches!(
        state(&mut session).ship.location,
        Location::Travelling(_)
    ));
    assert!(
        matches!(session.advance(1), ServerMessage::SessionSnapshot { events, .. }
        if events == vec![SessionEvent::Arrived { system: SystemId(2) }])
    );
    assert_eq!(
        state(&mut session).ship.location,
        Location::Docked(SystemId(2))
    );
    let before = state(&mut session);
    assert_eq!(
        session.handle(ClientCommand::Travel {
            ship: ShipId(1),
            destination: SystemId(3),
            expected_revision: before.revision
        }),
        ServerMessage::CommandRejected(TravelError::OutOfRange)
    );
    assert_eq!(state(&mut session), before);
}
