use super::{
    Session,
    combat_tests::reload,
    navigation_tests::{plot, run, succeed},
};
use game_core::{
    BodyId, SystemId,
    fleet::FleetId,
    movement::MoveTarget,
    orbits,
    travel::{Location, ShipId},
};
use game_protocol::ClientCommand;

#[test]
fn bodies_move_with_the_clock_and_saves_reload_them_where_they_are() {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    let start = session.systems[0].bodies[0].position;
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    for _ in 0..5 {
        run(&mut session);
    }
    let now = session.systems[0].bodies[0].position;
    assert_ne!(now, start, "the Inner world moved");
    assert_eq!(
        session.systems,
        orbits::placed(&Session::default().systems, session.state.tick),
        "positions are a function of the tick"
    );
    let restored = reload(&session);
    assert_eq!(restored.systems, session.systems);
}

#[test]
fn a_course_to_the_moving_moon_arrives_and_survives_reload_mid_flight() {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    // Let the system turn a while first, so nothing lines up with tick 0.
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::HoldFleet {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    for _ in 0..7 {
        run(&mut session);
    }
    let moon = MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(4),
    };
    plot(&mut session, ShipId(1), moon);
    let mut runs = 0;
    while session.state.navigation.order(FleetId(1)).is_some() {
        run(&mut session);
        session = reload(&session);
        runs += 1;
        assert!(runs < 20, "the course never ended");
    }
    assert_eq!(
        session.state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(4)
        }
    );
}
