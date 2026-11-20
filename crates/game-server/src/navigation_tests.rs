use super::{LOCAL_PLAYER, Session};
use game_core::{
    BodyId, SystemId,
    fleet::{FleetId, ShipKind},
    movement::MoveTarget,
    navigation::NavigationError,
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

pub(super) fn succeed(session: &mut Session, command: ClientCommand) {
    let response = session.handle(command);
    assert!(
        matches!(response, ServerMessage::SessionSnapshot { .. }),
        "{response:?}"
    );
}
/// Plots a course for `ship`'s fleet, first making it the flagship (only the
/// flagship takes direct orders).
pub(super) fn plot(session: &mut Session, ship: ShipId, target: MoveTarget) {
    command_fleet_of(session, ship);
    succeed(
        session,
        ClientCommand::Move {
            ship,
            target,
            expected_revision: session.state.revision,
        },
    );
}
/// Makes `ship`'s fleet the flagship, if it is not already.
pub(super) fn command_fleet_of(session: &mut Session, ship: ShipId) {
    let fleet = session.state.fleet_for_ship(ship).unwrap().id;
    if session.state.flagship(crate::LOCAL_PLAYER) != Some(fleet) {
        succeed(
            session,
            ClientCommand::SetFlagship {
                fleet,
                expected_revision: session.state.revision,
            },
        );
    }
}

pub(super) fn run(session: &mut Session) {
    succeed(
        session,
        ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision,
        },
    );
}
pub(super) fn two_fleets() -> Session {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    succeed(
        &mut session,
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: 0,
        },
    );
    succeed(
        &mut session,
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(2)],
            expected_revision: 1,
        },
    );
    session
}
fn body(id: u64) -> MoveTarget {
    MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(id),
    }
}

#[test]
fn two_fleets_plot_independently_and_advance_on_one_shared_run() {
    let mut session = two_fleets();
    let original = session.economy.clone();
    plot(&mut session, ShipId(1), body(1));
    plot(&mut session, ShipId(2), body(4));
    assert_eq!(session.state.tick, 0);
    assert!(
        session
            .state
            .ships
            .iter()
            .all(|ship| ship.location == Location::Docked(SystemId(1)))
    );
    assert_eq!(session.state.ships[0].fuel, 40);
    assert_eq!(session.state.ships[1].fuel, 50);
    assert_eq!(session.economy, original);
    let revision = session.state.revision;
    run(&mut session);
    assert_eq!(session.state.tick, 4);
    assert_eq!(session.state.navigation.run, 1);
    assert_eq!(session.state.revision, revision + 1);
    assert_eq!(
        session.state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
    assert_eq!(
        session.state.ships[1].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(4)
        }
    );
    assert_eq!(session.state.ships[0].fuel, 39);
    assert_eq!(session.state.ships[1].fuel, 49);
    let mut expected = original;
    expected.advance(4);
    expected.advance_traders(&session.systems).unwrap();
    assert_eq!(session.economy, expected);
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
}

fn wait(session: &mut Session, fleet: FleetId) {
    succeed(
        session,
        ClientCommand::HoldFleet {
            fleet,
            expected_revision: session.state.revision,
        },
    );
}

#[test]
fn only_the_flagship_needs_orders_and_secondary_fleets_take_none_directly() {
    let mut session = two_fleets();
    assert_eq!(session.state.flagship(LOCAL_PLAYER), Some(FleetId(1)));
    // A secondary fleet takes no direct orders, and nothing changes.
    let before = session.snapshot(Vec::new());
    for command in [
        ClientCommand::Move {
            ship: ShipId(2),
            target: body(1),
            expected_revision: session.state.revision,
        },
        ClientCommand::Land {
            fleet: FleetId(2),
            expected_revision: session.state.revision,
        },
    ] {
        assert_eq!(
            session.handle(command),
            ServerMessage::CommandRejected(NavigationError::NotFlagship.into())
        );
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    // The flagship needs a course or Wait; idle Fleet 2 just waits.
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(1))
    );
    plot(&mut session, ShipId(1), body(1));
    assert_eq!(session.state.navigation_attention(LOCAL_PLAYER), None);
    run(&mut session);
    assert_eq!(
        session.state.ships[1].location,
        Location::Docked(SystemId(1)),
        "the secondary fleet stayed put"
    );
    // Taking command of Fleet 2: it now needs orders; Fleet 1 never asks again.
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SetFlagship {
            fleet: FleetId(2),
            expected_revision: revision,
        },
    );
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(2))
    );
    wait(&mut session, FleetId(2));
    run(&mut session);
    assert_eq!(session.state.navigation.run, 2);
}

#[test]
fn fleet_changes_keep_waits_only_for_fleets_that_still_exist() {
    let mut session = two_fleets();
    wait(&mut session, FleetId(1));
    wait(&mut session, FleetId(2));
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::BuyShip {
            ship: ShipId(1),
            kind: ShipKind::Scout,
            expected_revision: revision,
        },
    );
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(3)],
            expected_revision: revision,
        },
    );
    // The split-off fleet is secondary: it waits without asking; the others
    // keep their Waits.
    assert_eq!(session.state.navigation.holds, [FleetId(1), FleetId(2)]);
    assert_eq!(session.state.navigation_attention(LOCAL_PLAYER), None);
    wait(&mut session, FleetId(3));
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::SellShip {
            ship: ShipId(3),
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.navigation.holds, [FleetId(1), FleetId(2)]);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::MergeFleets {
            source: FleetId(2),
            target: FleetId(1),
            expected_revision: revision,
        },
    );
    assert_eq!(session.state.navigation.holds, [FleetId(1)]);
    session.state.validate_navigation(&session.systems).unwrap();
    run(&mut session);
    assert_eq!(session.state.navigation.holds, [FleetId(1)]);
}

#[test]
fn queued_courses_can_be_replaced_or_cancelled_without_charging_fuel() {
    let mut session = two_fleets();
    plot(&mut session, ShipId(1), body(1));
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle(ClientCommand::SplitFleet {
            fleet: FleetId(1),
            ships: vec![ShipId(1)],
            expected_revision: session.state.revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    plot(&mut session, ShipId(1), body(4));
    assert_eq!(session.state.navigation.orders.len(), 1);
    assert_eq!(session.state.ships[0].fuel, 40);
    let revision = session.state.revision;
    succeed(
        &mut session,
        ClientCommand::CancelCourse {
            fleet: FleetId(1),
            expected_revision: revision,
        },
    );
    assert!(session.state.navigation.orders.is_empty());
    assert_eq!(session.state.tick, 0);
    assert_eq!(session.state.ships[0].fuel, 40);
}

#[test]
fn a_queued_course_replans_when_its_destination_moves_before_departure() {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    plot(&mut session, ShipId(1), body(1));
    let original = session.state.navigation.orders[0].plan.clone();
    let original_position = session.systems[0].bodies[0].position;
    let fuel = session.state.ships[0].fuel;
    assert!(matches!(
        session.advance(4),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert_eq!(session.state.ships[0].fuel, fuel);
    assert_ne!(session.state.tick, original.tick);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("queued-orbit.json");
    session.save(&path).unwrap();
    let (mut session, _) = Session::load(&path).unwrap();
    run(&mut session);
    assert_eq!(session.state.tick, 8);
    assert_eq!(
        session.state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
    assert_ne!(session.systems[0].bodies[0].position, original_position);
}

#[test]
fn longer_courses_progress_over_runs_and_cannot_trade_or_recharge_fuel_in_transit() {
    let mut session = Session::default();
    session.state.movement_rules.encounter_chance_percent = 0;
    super::test_support::move_fleet(
        &mut session,
        ShipId(1),
        MoveTarget::Local {
            system: SystemId(1),
            position: game_core::LocalPosition {
                x_km: 200_000_001,
                y_km: 0,
                z_km: 35_000_000,
            },
        },
    );
    plot(
        &mut session,
        ShipId(1),
        MoveTarget::EnterSystem(SystemId(2)),
    );
    run(&mut session);
    assert!(matches!(
        session.state.ships[0].location,
        Location::InTransit(FleetId(1))
    ));
    assert_eq!(session.state.navigation.orders[0].remaining_ticks, 11);
    let fuel = session.state.ships[0].fuel;
    let before = session.snapshot(Vec::new());
    for command in [
        ClientCommand::CancelCourse {
            fleet: FleetId(1),
            expected_revision: session.state.revision,
        },
        ClientCommand::Refuel {
            ship: ShipId(1),
            units: 1,
            expected_revision: session.state.revision,
        },
    ] {
        assert!(matches!(
            session.handle(command),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    for _ in 0..3 {
        run(&mut session);
        assert_eq!(session.state.ships[0].fuel, fuel);
    }
    assert!(matches!(
        session.state.ships[0].location,
        Location::InSystem {
            system: SystemId(2),
            ..
        }
    ));
    assert!(session.state.navigation.orders.is_empty());
}
