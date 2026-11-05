//! Helpers that exercise real movement commands; legacy fixtures only test migration.
use super::{LOCAL_PLAYER, Session};
use game_core::{BodyId, LocalPosition, SystemId, movement::MoveTarget, travel::ShipId};
use game_protocol::{ClientCommand, ServerMessage};

pub(super) fn move_fleet(session: &mut Session, ship: ShipId, target: MoveTarget) {
    let response = session.handle_offline(ClientCommand::Move {
        ship,
        target,
        expected_revision: session.state.revision,
    });
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}

pub(super) fn travel_to_port(session: &mut Session, destination: SystemId) {
    let origin = session.state.ships[0].location.system().unwrap();
    let start = game_core::movement::local_position(
        &session.state.ships[0].location,
        origin,
        &session.systems,
    )
    .unwrap();
    move_fleet(
        session,
        ShipId(1),
        MoveTarget::Local {
            system: origin,
            position: LocalPosition {
                x_km: session.state.movement_rules.system_half_extent_km + 1,
                ..start
            },
        },
    );
    move_fleet(session, ShipId(1), MoveTarget::EnterSystem(destination));
    move_fleet(
        session,
        ShipId(1),
        MoveTarget::Body {
            system: destination,
            body: BodyId(3),
        },
    );
}

pub(super) fn legacy_depart(session: &mut Session) {
    let quotes = session.state.fleets[0]
        .ships
        .iter()
        .map(|id| {
            session.state.authorize(LOCAL_PLAYER, *id).unwrap();
            session
                .state
                .quote_for(
                    *id,
                    SystemId(2),
                    session.systems[0].position,
                    session.systems[1].position,
                )
                .unwrap()
        })
        .collect::<Vec<_>>();
    session.state.depart_many(&quotes).unwrap();
}
