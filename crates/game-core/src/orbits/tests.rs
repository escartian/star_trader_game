use super::*;
use crate::movement::MovementRules;

fn at(x: i64, y: i64, z: i64) -> LocalPosition {
    LocalPosition {
        x_km: x,
        y_km: y,
        z_km: z,
    }
}

fn length(position: LocalPosition) -> f64 {
    let [x, y, z] = [position.x_km, position.y_km, position.z_km].map(|value| value as f64);
    (x * x + y * y + z * z).sqrt()
}

#[test]
fn integer_trigonometry_matches_floating_point() {
    for step in 0..360_u64 {
        let turns = step * TURN / 360;
        let (cos, sin) = cos_sin(turns);
        let angle = step as f64 / 360.0 * std::f64::consts::TAU;
        let scale = (1_u64 << 30) as f64;
        assert!(
            (cos as f64 / scale - angle.cos()).abs() < 2e-9,
            "cos {step}°"
        );
        assert!(
            (sin as f64 / scale - angle.sin()).abs() < 2e-9,
            "sin {step}°"
        );
    }
}

#[test]
fn bodies_circle_at_constant_distance_and_return_after_one_period() {
    let offset = at(45_000_000, 8_000_000, -20_000_000);
    let period = 400;
    let radius = length(offset);
    for tick in (0..period).step_by(7) {
        let moved = rotate(offset, tick, period);
        assert_eq!(moved.y_km, offset.y_km, "orbits stay at their height");
        assert!(
            (length(moved) - radius).abs() < 2.0,
            "tick {tick}: radius drifts"
        );
    }
    assert_eq!(rotate(offset, period, period), offset);
    assert_eq!(rotate(offset, 0, period), offset);
    let quarter = rotate(at(10_000_000, 0, 0), period / 4, period);
    assert_eq!((quarter.x_km, quarter.z_km), (0, 10_000_000));
}

#[test]
fn the_demo_system_moves_with_time_and_keeps_its_moon_beside_its_planet() {
    let systems = crate::demo_galaxy();
    let later = placed(&systems, 120);
    let (inner, moon) = (&later[0].bodies[0], &later[0].bodies[3]);
    assert_ne!(
        inner.position, systems[0].bodies[0].position,
        "planets move"
    );
    let gap = |a: &crate::LocalBody, b: &crate::LocalBody| {
        length(LocalPosition {
            x_km: a.position.x_km - b.position.x_km,
            y_km: a.position.y_km - b.position.y_km,
            z_km: a.position.z_km - b.position.z_km,
        })
    };
    let start_gap = gap(&systems[0].bodies[0], &systems[0].bodies[3]);
    assert!(
        (gap(inner, moon) - start_gap).abs() < 2.0,
        "the moon stays in orbit"
    );
    assert_eq!(
        placed(&systems, 0),
        systems,
        "tick 0 is the authored layout"
    );
    // Farther bodies take longer: inner world < asteroid < gas giant.
    let period = |index: usize| {
        systems[0].bodies[index]
            .orbit
            .as_ref()
            .unwrap()
            .period_ticks
    };
    assert!(period(0) < period(4) && period(4) < period(1));
    assert_eq!(period(3), MOON_PERIOD_TICKS);
    // The layout is valid at every tick, not just at tick 0.
    let rules = MovementRules::default();
    for tick in (0..2_000).step_by(37) {
        rules.validate_world(&placed(&systems, tick)).unwrap();
    }
}

#[test]
fn orbits_that_could_ever_collide_or_nest_are_rejected() {
    let rules = MovementRules::default();
    // Push the trade port's orbit onto the Inner world's ring.
    let mut systems = crate::demo_galaxy();
    let inner = systems[0].bodies[0].orbit.clone().unwrap();
    let port = systems[0].bodies[2].orbit.as_mut().unwrap();
    port.epoch = LocalPosition {
        x_km: -inner.epoch.x_km,
        ..inner.epoch
    };
    assert!(
        rules.validate_world(&systems).is_err(),
        "same ring, opposite sides"
    );
    let mut systems = crate::demo_galaxy();
    systems[0].bodies[3].orbit.as_mut().unwrap().parent = Some(BodyId(4));
    assert!(rules.validate_world(&systems).is_err(), "a moon of itself");
    let mut systems = crate::demo_galaxy();
    systems[0].bodies[4].orbit.as_mut().unwrap().period_ticks = 0;
    assert!(rules.validate_world(&systems).is_err());
}

#[test]
fn fixed_legacy_bodies_on_opposite_sides_of_one_ring_remain_valid() {
    let mut systems = crate::demo_galaxy();
    for body in &mut systems[0].bodies {
        body.orbit = None;
    }
    let inner = systems[0].bodies[0].position;
    systems[0].bodies[2].position = LocalPosition {
        x_km: -inner.x_km,
        ..inner
    };
    MovementRules::default().validate_world(&systems).unwrap();
}
