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
pub(super) fn plot(session: &mut Session, ship: ShipId, target: MoveTarget) {
    succeed(
        session,
        ClientCommand::Move {
            ship,
            target,
            expected_revision: session.state.revision,
        },
    );
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
fn unassigned_fleets_need_an_explicit_wait_that_lasts_until_a_new_course() {
    let mut session = two_fleets();
    plot(&mut session, ShipId(1), body(1));
    let before = session.snapshot(Vec::new());
    assert_eq!(
        session.handle(ClientCommand::ExecuteNavigationRun {
            expected_revision: session.state.revision
        }),
        ServerMessage::CommandRejected(NavigationError::NeedsOrders(FleetId(2)).into())
    );
    assert_eq!(session.snapshot(Vec::new()), before);
    wait(&mut session, FleetId(2));
    run(&mut session);
    assert_eq!(
        session.state.ships[1].location,
        Location::Docked(SystemId(1))
    );
    assert_eq!(session.state.ships[1].fuel, 50);
    // Fleet 1 arrived and has no orders, so it needs attention; Fleet 2's Wait stands.
    assert_eq!(session.state.navigation.holds, [FleetId(2)]);
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(1))
    );
    wait(&mut session, FleetId(1));
    run(&mut session);
    run(&mut session);
    assert_eq!(session.state.navigation.run, 3);
    plot(&mut session, ShipId(2), body(4));
    assert_eq!(session.state.navigation.holds, [FleetId(1)]);
    run(&mut session);
    // After arriving, Fleet 2 needs a deliberate new order rather than waiting by default.
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(2))
    );
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
    // The split-off fleet starts without orders; the others keep their Waits.
    assert_eq!(session.state.navigation.holds, [FleetId(1), FleetId(2)]);
    assert_eq!(
        session.state.navigation_attention(LOCAL_PLAYER),
        Some(FleetId(3))
    );
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
                y_km: 10_000_000,
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
