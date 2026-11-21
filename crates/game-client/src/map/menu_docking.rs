//! Station traffic behind the main menu: ships arriving and sliding into the
//! trade station's dock, leaving it, or dropping from it into the planet.
//! Each of `LANES` lanes flies one trip per `SLOT_SECONDS`, seeded by the
//! launch, lane and slot. Docking ships slide down the station's axis while
//! shrinking, and descending ships shrink as they sink below the surface, so
//! they disappear into the station or planet rather than popping.
use super::menu_flights::{Flight, Leg, between, pick_ship};
use super::menu_scene::{PLANET_CENTRE, PLANET_RADIUS, dock, station_axis};
use glam::Vec3;

const LANES: u64 = 2;
const SLOT_SECONDS: f32 = 26.0;
/// Ships near the station fly at this share of their size, so it reads as large.
const SIZE_SHARE: f32 = 0.5;
/// How far inside the hull a docking ship slides while shrinking, as a
/// multiple of its length.
const SLIDE_IN_HULLS: f32 = 1.4;
/// Where descents meet the planet: the near face, high on the station's side.
const ENTRY_DIRECTION: Vec3 = Vec3::new(-0.45, -0.75, 0.48);
/// How deep below the surface a descent ends, as a share of the radius.
const SINK_SHARE: f32 = 0.45;

/// Station flights in the air at `time` seconds, each with its progress (0-1).
pub(super) fn traffic(time: f32, launch_seed: u64) -> Vec<(Flight, f32)> {
    let mut flying = Vec::new();
    for lane in 0..LANES {
        let lane_time = time + lane as f32 * SLOT_SECONDS / LANES as f32;
        let slot = (lane_time / SLOT_SECONDS).floor().max(0.0) as u64;
        let into_slot = lane_time - slot as f32 * SLOT_SECONDS;
        let seed = launch_seed.rotate_left(17) ^ (lane << 40) ^ slot ^ 0x57A7_1011;
        let mut random = fastrand::Rng::with_seed(seed);
        let (flight, speed) = plan(&mut random);
        // Easing halves the average speed, so allow double the cruise time.
        let duration = (2.0 * flight.length() / speed).clamp(6.0, SLOT_SECONDS - 1.0);
        let start = between(&mut random, 0.0, SLOT_SECONDS - duration);
        let progress = (into_slot - start) / duration;
        if (0.0..=1.0).contains(&progress) {
            flying.push((flight, progress));
        }
    }
    flying
}

/// One station trip and the ship's cruise speed (world units per second).
fn plan(random: &mut fastrand::Rng) -> (Flight, f32) {
    let (kind, size, speed) = pick_ship(random);
    let size = size * SIZE_SHARE;
    let leg = match random.u8(0..10) {
        0..=3 => Leg::Arrive,
        4..=6 => Leg::Depart,
        _ => Leg::Descend,
    };
    // Off-screen ends come in from either side at varied depth and height.
    let side = if random.bool() { 1.0 } else { -1.0 };
    let far = Vec3::new(
        220.0 * side,
        between(random, 20.0, 140.0),
        between(random, -10.0, 34.0),
    );
    let slide = size * SLIDE_IN_HULLS;
    let inside = dock() - station_axis() * slide;
    let entry_direction = ENTRY_DIRECTION.normalize();
    let surface = PLANET_CENTRE + entry_direction * PLANET_RADIUS;
    let sunk = PLANET_CENTRE + entry_direction * PLANET_RADIUS * (1.0 - SINK_SHARE);
    let (points, grow_over, shrink_over) = match leg {
        Leg::Arrive => (vec![far, dock(), inside], 0.0, slide),
        Leg::Depart => (vec![inside, dock(), far], slide, 0.0),
        // Shrinking starts a little above the surface and ends deep below it.
        _ => (
            vec![inside, dock(), surface, sunk],
            slide,
            PLANET_RADIUS * SINK_SHARE + 3.0,
        ),
    };
    let flight = Flight {
        kind,
        leg,
        points,
        arc: 0.0,
        size,
        bank: between(random, -0.2, 0.2),
        phase: between(random, 0.0, 100.0),
        grow_over,
        shrink_over,
    };
    (flight, speed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn station_ships_slide_into_the_hull_and_sink_into_the_planet() {
        let mut legs = Vec::new();
        for second in 0..900 {
            for (flight, _) in traffic(second as f32, 3) {
                if !legs.contains(&flight.leg) {
                    legs.push(flight.leg);
                }
                let (start, end) = (flight.position(0.0), flight.position(1.0));
                match flight.leg {
                    Leg::Arrive => {
                        assert!(end.distance(dock()) > 0.5, "ends inside, past the dock");
                        assert_eq!(flight.size_at(1.0), 0.0);
                    }
                    Leg::Depart => {
                        assert!(start.distance(dock()) > 0.5, "starts inside the hull");
                        assert_eq!(flight.size_at(0.0), 0.0);
                    }
                    Leg::Descend => {
                        assert!(
                            end.distance(PLANET_CENTRE) < PLANET_RADIUS,
                            "ends below the surface"
                        );
                        assert_eq!(flight.size_at(0.0), 0.0);
                        assert_eq!(flight.size_at(1.0), 0.0);
                    }
                    Leg::Cross => unreachable!("station lanes never cross"),
                }
                // Size changes are gradual: no jump between nearby moments.
                for step in 0..100 {
                    let (a, b) = (step as f32 / 100.0, (step + 1) as f32 / 100.0);
                    assert!((flight.size_at(a) - flight.size_at(b)).abs() < flight.size * 0.35);
                }
            }
        }
        assert_eq!(legs.len(), 3, "arrivals, departures and descents all occur");
    }
}
