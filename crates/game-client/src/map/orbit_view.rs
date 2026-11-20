//! Bodies drawn where their orbits put them. Between runs the map shows the
//! authoritative positions for the current tick; while a run's glide plays, it
//! shows positions part-way between the ticks before and after the run, so
//! planets move smoothly with the ships. Display only: rules always place
//! bodies themselves (`game_core::orbits`).
use game_core::{LocalPosition, StarSystem, orbits};

/// `systems` with bodies placed `progress` (0 to 1) of the way from tick `from`
/// to tick `to`. Positions are placed at the two whole ticks around that point
/// and blended, so curved orbits stay curved.
pub fn display_systems(
    systems: &[StarSystem],
    from: u64,
    to: u64,
    progress: f64,
) -> Vec<StarSystem> {
    if from >= to || progress >= 1.0 {
        return orbits::placed(systems, to);
    }
    let point = from as f64 + (to - from) as f64 * progress.clamp(0.0, 1.0);
    let floor = point.floor() as u64;
    let blend = point - floor as f64;
    let mut low = orbits::placed(systems, floor);
    let high = orbits::placed(systems, floor + 1);
    for (system, next) in low.iter_mut().zip(&high) {
        for (body, ahead) in system.bodies.iter_mut().zip(&next.bodies) {
            body.position = mix(body.position, ahead.position, blend);
        }
    }
    low
}

fn mix(a: LocalPosition, b: LocalPosition, t: f64) -> LocalPosition {
    let lerp = |a: i64, b: i64| a + ((b - a) as f64 * t).round() as i64;
    LocalPosition {
        x_km: lerp(a.x_km, b.x_km),
        y_km: lerp(a.y_km, b.y_km),
        z_km: lerp(a.z_km, b.z_km),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_display_glides_from_the_old_tick_to_the_new_one() {
        let systems = game_core::demo_galaxy();
        let at = |progress| display_systems(&systems, 0, 4, progress)[0].bodies[0].position;
        assert_eq!(at(0.0), orbits::placed(&systems, 0)[0].bodies[0].position);
        assert_eq!(at(1.0), orbits::placed(&systems, 4)[0].bodies[0].position);
        assert_eq!(at(0.5), orbits::placed(&systems, 2)[0].bodies[0].position);
        let quarter = at(0.125);
        assert_ne!(quarter, at(0.0));
        assert_ne!(quarter, at(0.25));
    }
}
