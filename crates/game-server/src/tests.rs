use super::*;
use game_core::{
    BodyId, SystemId,
    movement::MoveTarget,
    travel::{Location, PlayerId, ShipId, TravelError},
};

fn state(session: &mut Session) -> TravelState {
    let ServerMessage::SessionSnapshot { state, .. } = session.handle(ClientCommand::GetSession)
    else {
        panic!("session snapshot missing")
    };
    *state
}

#[test]
fn snapshots_and_previews_do_not_mutate_authority_or_advance_time() {
    let mut session = Session::default();
    let ServerMessage::GalaxySnapshot { mut systems, .. } =
        session.handle(ClientCommand::GetGalaxy)
    else {
        panic!("galaxy snapshot missing")
    };
    systems.clear();
    let mut copy = state(&mut session);
    copy.ships[0].fuel = 0;
    for _ in 0..10 {
        assert!(matches!(
            session.handle(ClientCommand::PreviewMove {
                ship: ShipId(1),
                target: MoveTarget::Body {
                    system: SystemId(1),
                    body: BodyId(1)
                }
            }),
            ServerMessage::MovementPreview(_)
        ));
    }
    assert_eq!(state(&mut session), Session::default().state);
}

#[test]
fn rejected_commands_leave_the_authoritative_state_unchanged() {
    let mut session = Session::default();
    let initial = state(&mut session);
    for (player, ship, revision, error) in [
        (PlayerId(2), ShipId(1), 0, TravelError::NotOwner),
        (LOCAL_PLAYER, ShipId(99), 0, TravelError::UnknownShip),
        (LOCAL_PLAYER, ShipId(1), 1, TravelError::StaleQuote),
    ] {
        assert_eq!(
            session.handle_offline_as(
                player,
                ClientCommand::Move {
                    ship,
                    target: MoveTarget::Body {
                        system: SystemId(1),
                        body: BodyId(1)
                    },
                    expected_revision: revision
                }
            ),
            ServerMessage::CommandRejected(error.into())
        );
        assert_eq!(state(&mut session), initial);
    }
    let response = session.handle(ClientCommand::Move {
        ship: ShipId(1),
        target: MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(1),
        },
        expected_revision: 0,
    });
    assert!(matches!(response, ServerMessage::SessionSnapshot { .. }));
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.state.ships[0].location, initial.ships[0].location);
}

#[test]
fn older_journeys_arrive_only_when_the_host_advances_time() {
    let mut session = Session::default();
    super::test_support::legacy_depart(&mut session);
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.state.ships[0].fuel, 25);
    session.advance(14);
    assert!(matches!(
        state(&mut session).ships[0].location,
        Location::Travelling(_)
    ));
    assert!(
        matches!(session.advance(1), ServerMessage::SessionSnapshot { events, .. } if events == vec![SessionEvent::Arrived { system: SystemId(2), ship: ShipId(1) }])
    );
    assert_eq!(
        state(&mut session).ships[0].location,
        Location::Docked(SystemId(2))
    );
}
