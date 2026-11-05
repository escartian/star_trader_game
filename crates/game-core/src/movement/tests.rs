use super::*;
use crate::{
    BodyId, SystemId,
    travel::{Location, PlayerId, ShipId, TravelState},
};

fn plan(state: &TravelState, target: MoveTarget) -> MovementPlan {
    state
        .plan_movement(PlayerId(1), ShipId(1), &target, &crate::demo_galaxy())
        .unwrap()
}

#[test]
fn local_body_movement_and_first_face_exit_use_real_kilometers() {
    let mut state = TravelState::default();
    let target = MoveTarget::Body {
        system: SystemId(1),
        body: BodyId(1),
    };
    let movement = plan(&state, target);
    assert_eq!(movement.mode, MovementMode::Local);
    assert_eq!(movement.fuel_per_ship, 1);
    let duration = movement.duration_ticks;
    state.resolve_movement(PlayerId(1), &movement).unwrap();
    assert_eq!(state.tick, duration);
    assert_eq!(
        state.ships[0].location,
        Location::Orbiting {
            system: SystemId(1),
            body: BodyId(1)
        }
    );
    let movement = plan(
        &state,
        MoveTarget::Local {
            system: SystemId(1),
            position: LocalPosition {
                x_km: 900_000_000,
                y_km: 8_000_000,
                z_km: -20_000_000,
            },
        },
    );
    assert_eq!(movement.mode, MovementMode::Exit);
    assert_eq!(
        movement.destination,
        Location::DeepSpace(SpacePosition {
            x_km: 200_000_001,
            y_km: 8_000_000,
            z_km: -20_000_000
        })
    );
}

#[test]
fn cube_corner_exit_and_near_face_entry_do_not_mix_spaces() {
    let origin = LocalPosition {
        x_km: 0,
        y_km: 0,
        z_km: 0,
    };
    assert_eq!(
        geometry::exit(
            origin,
            LocalPosition {
                x_km: 400,
                y_km: 200,
                z_km: 100
            },
            100
        )
        .unwrap(),
        LocalPosition {
            x_km: 101,
            y_km: 50,
            z_km: 25
        }
    );
    let center = SpacePosition::from_galaxy(GalaxyPosition { x: 8, y: 4, z: 12 }).unwrap();
    let start = SpacePosition {
        x_km: 0,
        y_km: 0,
        z_km: 0,
    };
    let entry = geometry::entry(start, center, 200_000_000).unwrap();
    assert_eq!(entry.z_km, -200_000_000);
    assert_eq!(entry.x_km, -133_333_333);
    assert_eq!(entry.y_km, -66_666_666);
}

#[test]
fn crossing_a_system_never_intercepts_but_exact_target_explicitly_enters() {
    let mut state = TravelState::default();
    state.ships[0].location = Location::DeepSpace(
        SpacePosition::from_galaxy(GalaxyPosition { x: -2, y: 0, z: 0 }).unwrap(),
    );
    let movement = plan(
        &state,
        MoveTarget::DeepSpace(GalaxyPosition { x: 2, y: 0, z: 0 }),
    );
    assert_eq!(movement.mode, MovementMode::DeepSpace);
    state.resolve_movement(PlayerId(1), &movement).unwrap();
    assert!(matches!(state.ships[0].location, Location::DeepSpace(_)));
    let entry = plan(
        &state,
        MoveTarget::DeepSpace(GalaxyPosition { x: 0, y: 0, z: 0 }),
    );
    assert_eq!(entry.mode, MovementMode::Entry);
    assert_eq!(
        entry.destination,
        Location::InSystem {
            system: SystemId(1),
            position: LocalPosition {
                x_km: 200_000_000,
                y_km: 0,
                z_km: 0
            }
        }
    );
}

#[test]
fn invalid_coordinates_collisions_and_zero_distance_are_rejected_without_mutation() {
    let state = TravelState::default();
    for target in [
        MoveTarget::Local {
            system: SystemId(1),
            position: LocalPosition {
                x_km: -30_000_000,
                y_km: -10_000_000,
                z_km: -35_000_000,
            },
        },
        MoveTarget::Body {
            system: SystemId(1),
            body: BodyId(3),
        },
        MoveTarget::EnterSystem(SystemId(2)),
        MoveTarget::DeepSpace(GalaxyPosition {
            x: i64::MAX,
            y: 0,
            z: 0,
        }),
    ] {
        assert!(
            state
                .plan_movement(PlayerId(1), ShipId(1), &target, &crate::demo_galaxy())
                .is_err()
        );
        assert_eq!(state, TravelState::default());
    }
    assert!(
        SpacePosition::from_galaxy(GalaxyPosition {
            x: i64::MAX,
            y: 0,
            z: 0
        })
        .is_err()
    );
    assert!(geometry::distance([i64::MIN; 3], [i64::MAX; 3]).is_err());
    let extreme = plan(
        &state,
        MoveTarget::Local {
            system: SystemId(1),
            position: LocalPosition {
                x_km: i64::MAX,
                y_km: 10_000_000,
                z_km: 35_000_000,
            },
        },
    );
    assert_eq!(extreme.mode, MovementMode::Exit);
}

#[test]
fn range_boundary_uses_exact_squared_kilometers_and_checks_every_member() {
    let mut state = TravelState::default();
    let origin = SpacePosition::from_galaxy(GalaxyPosition { x: -1, y: 5, z: 0 }).unwrap();
    state.ships[0].location = Location::DeepSpace(origin);
    state.ships[0].range_ly = 1;
    state.ships[0].fuel = 1;
    let target = MoveTarget::DeepSpace(GalaxyPosition { x: 0, y: 5, z: 0 });
    let movement = plan(&state, target.clone());
    assert_eq!(movement.fuel_per_ship, 1);
    assert_eq!(movement.duration_ticks, 1);
    state.ships[0].location = Location::DeepSpace(SpacePosition {
        x_km: origin.x_km - 1,
        ..origin
    });
    assert!(
        state
            .plan_movement(PlayerId(1), ShipId(1), &target, &crate::demo_galaxy())
            .is_err()
    );
    state.ships[0].location = Location::DeepSpace(origin);
    let mut second = state.ships[0].clone();
    second.id = ShipId(2);
    second.fuel = 0;
    state.ships.push(second);
    state.fleets[0].ships.push(ShipId(2));
    assert!(
        state
            .plan_movement(PlayerId(1), ShipId(1), &target, &crate::demo_galaxy())
            .is_err()
    );
    let before = state.clone();
    assert!(state.resolve_movement(PlayerId(1), &movement).is_err());
    assert_eq!(state, before);
}

#[test]
fn maximum_supported_local_geometry_and_configuration_remain_checked() {
    let start = LocalPosition {
        x_km: 1_000_000_000,
        y_km: 1_000_000_000,
        z_km: 1_000_000_000,
    };
    let end = LocalPosition {
        x_km: -1_000_000_000,
        y_km: -1_000_000_000,
        z_km: -1_000_000_000,
    };
    assert!(geometry::crosses_star(start, end, 1));
    assert!(!geometry::crosses_star(
        start,
        LocalPosition {
            x_km: -1_000_000_000,
            ..start
        },
        1
    ));
    let boundary = LocalPosition {
        x_km: 200_000_000,
        y_km: 0,
        z_km: 0,
    };
    assert_eq!(
        geometry::exit(
            boundary,
            LocalPosition {
                x_km: i64::MAX,
                ..boundary
            },
            200_000_000
        )
        .unwrap()
        .x_km,
        200_000_001
    );
    let mut rules = MovementRules {
        system_half_extent_km: 10,
        ..MovementRules::default()
    };
    assert!(rules.validate_world(&crate::demo_galaxy()).is_err());
    rules.system_half_extent_km = i64::MAX;
    assert!(rules.validate().is_err());
    assert!(MovementRules::from_json("{}").is_err());
}
