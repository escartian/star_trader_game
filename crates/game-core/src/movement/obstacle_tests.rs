use super::{MoveTarget, MovementError, MovementRules, obstacles};
use crate::{
    BodyId, BodyKind, LocalPosition, SystemId,
    travel::{Location, PlayerId, ShipId, TravelState},
};

const INNER_WORLD: BodyId = BodyId(1);
const MOON: BodyId = BodyId(4);

fn at(x: i64, y: i64, z: i64) -> LocalPosition {
    LocalPosition {
        x_km: x,
        y_km: y,
        z_km: z,
    }
}

/// The starting ship, moved to a free point in Sol.
fn ship_at(position: LocalPosition) -> TravelState {
    let mut state = TravelState::default();
    state.ships[0].location = Location::InSystem {
        system: SystemId(1),
        position,
    };
    state
}

fn local(position: LocalPosition) -> MoveTarget {
    MoveTarget::Local {
        system: SystemId(1),
        position,
    }
}

/// On the line from here to the Moon, the Inner world sits in between.
const BEHIND_INNER_WORLD: (i64, i64, i64) = (40_000_000, 5_500_000, -20_000_000);

#[test]
fn every_body_has_a_volume_and_the_demo_worlds_fit_without_overlaps() {
    let rules = MovementRules::default();
    assert_eq!(rules.body_radii.of(BodyKind::RockyPlanet), 600_000);
    assert!(rules.body_radii.of(BodyKind::GasGiant) > rules.body_radii.of(BodyKind::Moon));
    for seed in 0..32 {
        rules
            .validate_world(&crate::generation::generate_galaxy(seed))
            .unwrap();
    }
    // A moon pushed into its planet, or a planet poking out of the cube, is invalid.
    let mut systems = crate::demo_galaxy();
    systems[0].bodies[3].position = at(45_500_000, 8_000_000, -20_000_000);
    assert_eq!(
        rules.validate_world(&systems),
        Err(MovementError::InvalidRules)
    );
    let mut systems = crate::demo_galaxy();
    systems[0].bodies[0].position.x_km = rules.system_half_extent_km - 100_000;
    assert_eq!(
        rules.validate_world(&systems),
        Err(MovementError::InvalidRules)
    );
    // Older movement definitions without radii load with the defaults.
    let legacy = r#"{"schema_version":1,"galaxy_half_extent_ly":100,
        "system_half_extent_km":200000000,"star_exclusion_radius_km":1000000,
        "local_km_per_tick":50000000,"local_km_per_fuel":100000000}"#;
    assert_eq!(
        MovementRules::from_json(legacy).unwrap().body_radii,
        rules.body_radii
    );
}

#[test]
fn raw_waypoints_may_not_enter_or_cross_a_body() {
    let (x, y, z) = BEHIND_INNER_WORLD;
    let state = ship_at(at(x, y, z));
    let systems = crate::demo_galaxy();
    let plan = |target: &MoveTarget| state.plan_movement(PlayerId(1), ShipId(1), target, &systems);
    let inside = at(45_200_000, 8_000_000, -20_000_000);
    assert_eq!(
        plan(&local(inside)).unwrap_err(),
        MovementError::InsideBody.into()
    );
    let beyond = at(50_000_000, 10_500_000, -20_000_000);
    assert_eq!(
        plan(&local(beyond)).unwrap_err(),
        MovementError::BodyCollision.into()
    );
    let moon = MoveTarget::Body {
        system: SystemId(1),
        body: MOON,
    };
    assert_eq!(
        plan(&moon).unwrap_err(),
        MovementError::BodyCollision.into(),
        "a direct leg to the Moon passes through the Inner world"
    );
    // Leaving a body through its own volume, or arriving at one, is fine.
    let mut orbiting = state.clone();
    orbiting.ships[0].location = Location::Orbiting {
        system: SystemId(1),
        body: INNER_WORLD,
    };
    let away = at(40_000_000, 0, -20_000_000);
    orbiting
        .plan_movement(PlayerId(1), ShipId(1), &local(away), &systems)
        .unwrap();
}

#[test]
fn selected_destinations_route_around_bodies_in_the_way() {
    let (x, y, z) = BEHIND_INNER_WORLD;
    let state = ship_at(at(x, y, z));
    let systems = crate::demo_galaxy();
    let course = state
        .plan_destination(PlayerId(1), ShipId(1), SystemId(1), Some(MOON), &systems)
        .unwrap();
    assert!(course.legs.len() > 1, "a detour waypoint was added");
    let volumes = obstacles::obstacles(&state.movement_rules, &systems[0]);
    let point = |location: &Location| super::local_position(location, SystemId(1), &systems);
    for leg in &course.legs {
        let (start, end) = (
            point(&leg.origin).unwrap(),
            point(&leg.destination).unwrap(),
        );
        assert_eq!(obstacles::blocking(start, end, &volumes), None, "{leg:?}");
    }
    assert_eq!(
        course.legs.last().unwrap().destination,
        Location::Orbiting {
            system: SystemId(1),
            body: MOON
        }
    );
}

#[test]
fn saved_positions_inside_a_body_are_rejected() {
    let systems = crate::demo_galaxy();
    let rules = MovementRules::default();
    let inside = Location::InSystem {
        system: SystemId(1),
        position: at(45_100_000, 8_000_000, -20_000_000),
    };
    assert_eq!(
        rules.validate_location(&inside, &systems),
        Err(MovementError::InsideBody)
    );
}
