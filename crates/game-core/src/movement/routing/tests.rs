use super::*;
use crate::fleet::FleetId;

#[test]
fn selecting_a_remote_planet_builds_exit_entry_and_arrival_without_coordinates() {
    let state = TravelState::default();
    let systems = crate::demo_galaxy();
    let course = state
        .plan_destination(
            PlayerId(1),
            ShipId(1),
            SystemId(2),
            Some(BodyId(1)),
            &systems,
        )
        .unwrap();
    assert!(matches!(
        course.legs[0].mode,
        super::super::MovementMode::Exit | super::super::MovementMode::Local
    ));
    assert!(
        course
            .legs
            .iter()
            .any(|leg| leg.mode == super::super::MovementMode::Entry)
    );
    assert_eq!(
        course.legs.last().unwrap().destination,
        Location::Orbiting {
            system: SystemId(2),
            body: BodyId(1)
        }
    );
    assert_eq!(state, TravelState::default());
    let mut state = state;
    state.movement_rules.encounter_chance_percent = 0;
    state
        .plot_destination(
            PlayerId(1),
            ShipId(1),
            SystemId(2),
            Some(BodyId(1)),
            &systems,
        )
        .unwrap();
    for _ in 0..32 {
        if state.navigation.orders.is_empty() {
            break;
        }
        state.execute_navigation_run(PlayerId(1), &systems).unwrap();
        state.validate_navigation(&systems).unwrap();
        let position = state.navigation_position(ShipId(1), &systems).unwrap();
        for system in &systems {
            let center = SpacePosition::from_galaxy(system.position).unwrap();
            assert!(
                position
                    .coordinates()
                    .into_iter()
                    .zip(center.coordinates())
                    .any(|(a, b)| (i128::from(a) - i128::from(b)).abs() > 1_000_000)
            );
        }
    }
    assert!(state.navigation.orders.is_empty());
    assert_eq!(
        state.fleet_position(FleetId(1), PlayerId(1)).unwrap(),
        &Location::Orbiting {
            system: SystemId(2),
            body: BodyId(1)
        }
    );
}

#[test]
fn opposite_side_local_destinations_get_safe_waypoints_around_the_star() {
    let systems = crate::demo_galaxy();
    let mut state = TravelState::default();
    state.ships[0].location = Location::InSystem {
        system: SystemId(1),
        position: LocalPosition {
            x_km: -45_000_000,
            y_km: -8_000_000,
            z_km: 20_000_000,
        },
    };
    assert!(
        state
            .plan_movement(
                PlayerId(1),
                ShipId(1),
                &MoveTarget::Body {
                    system: SystemId(1),
                    body: BodyId(1)
                },
                &systems
            )
            .is_err()
    );
    let course = state
        .plan_destination(
            PlayerId(1),
            ShipId(1),
            SystemId(1),
            Some(BodyId(1)),
            &systems,
        )
        .unwrap();
    assert!(course.legs.len() > 1);
    for leg in &course.legs {
        let start = local_position(&leg.origin, SystemId(1), &systems).unwrap();
        let end = local_position(&leg.destination, SystemId(1), &systems).unwrap();
        assert!(!geometry::crosses_star(
            start,
            end,
            state.movement_rules.star_exclusion_radius_km
        ));
    }
}

#[test]
fn deep_space_segments_crossing_a_stellar_core_reject_without_side_effects() {
    let mut state = TravelState::default();
    let systems = crate::demo_galaxy();
    state.ships[0].location = Location::DeepSpace(
        SpacePosition::from_galaxy(crate::GalaxyPosition { x: -2, y: 0, z: 0 }).unwrap(),
    );
    let initial = state.clone();
    assert!(
        state
            .plan_movement(
                PlayerId(1),
                ShipId(1),
                &MoveTarget::DeepSpace(crate::GalaxyPosition { x: 2, y: 0, z: 0 }),
                &systems
            )
            .is_err()
    );
    assert_eq!(state, initial);
    let target = MoveTarget::LeaveSystem(SystemId(1));
    let state = TravelState::default();
    assert!(
        state
            .plan_movement(PlayerId(1), ShipId(1), &target, &systems)
            .is_ok()
    );
}
