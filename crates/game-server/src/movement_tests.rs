use super::{LOCAL_PLAYER, Session, test_support::move_fleet};
use game_core::{
    BodyId, GalaxyPosition, LocalPosition, SystemId,
    economy::{Commodity, TradeSide},
    fleet::{FleetId, ShipKind},
    movement::{MoveTarget, MovementMode, SpacePosition},
    travel::{Location, ShipId},
};
use game_protocol::{ClientCommand, ServerMessage};

fn round_trip(session: &Session) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("movement.json");
    session.save(&path).unwrap();
    let (restored, notice) = Session::load(&path).unwrap();
    assert!(notice.is_none());
    assert_eq!(restored.state, session.state);
    assert_eq!(restored.economy, session.economy);
}

#[test]
fn a_complete_trip_preserves_cargo_and_saves_at_every_movement_stage() {
    let mut session = Session::default();
    assert!(matches!(
        session.handle_offline(ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 5,
            side: TradeSide::Buy,
            expected_revision: 0
        }),
        ServerMessage::SessionSnapshot { .. }
    ));
    let credits = session.economy.credits;
    let cargo = session.state.ships[0].cargo;
    for (target, mode) in [
        (
            MoveTarget::Body {
                system: SystemId(1),
                body: BodyId(1),
            },
            MovementMode::Local,
        ),
        (
            MoveTarget::Local {
                system: SystemId(1),
                position: LocalPosition {
                    x_km: 900_000_000,
                    y_km: 8_000_000,
                    z_km: -20_000_000,
                },
            },
            MovementMode::Exit,
        ),
        (
            MoveTarget::DeepSpace(GalaxyPosition { x: 3, y: 2, z: 4 }),
            MovementMode::DeepSpace,
        ),
        (MoveTarget::EnterSystem(SystemId(2)), MovementMode::Entry),
        (
            MoveTarget::Body {
                system: SystemId(2),
                body: BodyId(3),
            },
            MovementMode::Local,
        ),
    ] {
        let original = session.snapshot(Vec::new());
        let plan = session
            .preview_movement(LOCAL_PLAYER, ShipId(1), &target)
            .unwrap();
        assert_eq!(plan.mode, mode);
        assert_eq!(session.snapshot(Vec::new()), original);
        let (tick, fuel, revision) = (
            session.state.tick,
            session.state.ships[0].fuel,
            session.state.revision,
        );
        let mut expected_markets = session.economy.clone();
        expected_markets.advance(plan.duration_ticks);
        move_fleet(&mut session, ShipId(1), target);
        assert_eq!(session.state.tick, tick + plan.duration_ticks);
        assert_eq!(session.state.revision, revision + 1);
        assert_eq!(session.state.ships[0].fuel, fuel - plan.fuel_per_ship);
        assert_eq!(session.state.ships[0].cargo, cargo);
        assert_eq!(session.economy, expected_markets);
        assert_eq!(session.economy.credits, credits);
        round_trip(&session);
    }
    assert_eq!(
        session.state.ships[0].location,
        Location::Docked(SystemId(2))
    );
    assert!(matches!(
        session.handle_offline(ClientCommand::Trade {
            ship: ShipId(1),
            commodity: Commodity::Food,
            quantity: 5,
            side: TradeSide::Sell,
            expected_revision: session.state.revision
        }),
        ServerMessage::SessionSnapshot { .. }
    ));
    assert!(session.economy.credits > credits);
}

#[test]
fn non_market_locations_do_not_allow_remote_trade_or_fleet_merge() {
    let mut session = Session::default();
    session.handle_offline(ClientCommand::BuyShip {
        ship: ShipId(1),
        kind: ShipKind::Scout,
        expected_revision: 0,
    });
    session.handle_offline(ClientCommand::SplitFleet {
        fleet: FleetId(1),
        ships: vec![ShipId(2)],
        expected_revision: 1,
    });
    move_fleet(
        &mut session,
        ShipId(2),
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(4),
        },
    );
    let before = session.snapshot(Vec::new());
    for command in [
        ClientCommand::Trade {
            ship: ShipId(2),
            commodity: Commodity::Food,
            quantity: 1,
            side: TradeSide::Buy,
            expected_revision: session.state.revision,
        },
        ClientCommand::Refuel {
            ship: ShipId(2),
            units: 1,
            expected_revision: session.state.revision,
        },
        ClientCommand::MergeFleets {
            source: FleetId(2),
            target: FleetId(1),
            expected_revision: session.state.revision,
        },
        ClientCommand::TransferCargo {
            source: ShipId(1),
            target: ShipId(2),
            commodity: Commodity::Food,
            quantity: 1,
            expected_revision: session.state.revision,
        },
    ] {
        assert!(matches!(
            session.handle_offline(command),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
}

#[test]
fn stale_collision_bounds_and_overflow_failures_are_atomic() {
    let mut session = Session::default();
    let valid = MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(1),
    };
    let revision = session.state.revision;
    move_fleet(&mut session, ShipId(1), valid.clone());
    let before = session.snapshot(Vec::new());
    assert!(matches!(
        session.handle_offline(ClientCommand::Move {
            ship: ShipId(1),
            target: valid,
            expected_revision: revision
        }),
        ServerMessage::CommandRejected(_)
    ));
    assert_eq!(session.snapshot(Vec::new()), before);
    for (tick, revision) in [(u64::MAX, 0), (0, u64::MAX)] {
        session.state.tick = tick;
        session.state.revision = revision;
        let before = session.snapshot(Vec::new());
        assert!(matches!(
            session.handle_offline(ClientCommand::Move {
                ship: ShipId(1),
                target: MoveTarget::Body {
                    system: SystemId(1),
                    body: BodyId(3)
                },
                expected_revision: revision
            }),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    session = Session::default();
    for target in [
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(999),
        },
        MoveTarget::Local {
            system: SystemId(1),
            position: LocalPosition {
                x_km: 0,
                y_km: 0,
                z_km: 0,
            },
        },
        MoveTarget::Body {
            system: SystemId(2),
            body: BodyId(3),
        },
    ] {
        let before = session.snapshot(Vec::new());
        assert!(matches!(
            session.handle_offline(ClientCommand::Move {
                ship: ShipId(1),
                target,
                expected_revision: 0
            }),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
    session.state.ships[0].location = Location::DeepSpace(SpacePosition {
        x_km: 200_000_001,
        y_km: 0,
        z_km: 0,
    });
    let before = session.snapshot(Vec::new());
    for target in [
        MoveTarget::DeepSpace(GalaxyPosition { x: 101, y: 0, z: 0 }),
        MoveTarget::DeepSpace(GalaxyPosition { x: 30, y: 0, z: 0 }),
        MoveTarget::DeepSpace(GalaxyPosition {
            x: i64::MAX,
            y: 0,
            z: 0,
        }),
    ] {
        assert!(matches!(
            session.handle_offline(ClientCommand::Move {
                ship: ShipId(1),
                target,
                expected_revision: 0
            }),
            ServerMessage::CommandRejected(_)
        ));
        assert_eq!(session.snapshot(Vec::new()), before);
    }
}
