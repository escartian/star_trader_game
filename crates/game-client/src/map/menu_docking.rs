//! Station traffic behind the main menu: ships arriving and sliding into the
//! trade station's dock, leaving it, or dropping from it into the planet.
//! Each of `LANES` lanes flies one trip per `SLOT_SECONDS`, seeded by the
//! launch, lane and slot. Docking ships slide down the station's axis while
//! shrinking, and descending ships shrink as they sink below the surface, so
//! they disappear into the station or planet rather than popping.
use super::menu_flights::{Flight, Leg, between, pick_ship};
use super::menu_motion::Motion;
use super::menu_scene::{PLANET_CENTRE, PLANET_RADIUS, dock, station_axis};
use glam::Vec3;

const LANES: u64 = 2;
const SLOT_SECONDS: f32 = 34.0;
/// Seconds a ship spends creeping into or out of the station, or sinking into
/// the planet, and the slowest crawl (world units per second).
const CREEP_SECONDS: f32 = 3.0;
const MIN_CRAWL: f32 = 0.6;
/// Length of the descent's straight climb out of the dock before it turns.
const CORRIDOR: f32 = 14.0;
/// Distance over which a ship brakes from cruise to a crawl before docking.
const BRAKE_DISTANCE: f32 = 45.0;
/// Distance over which a departing ship speeds up from a crawl.
const PULL_AWAY_DISTANCE: f32 = 35.0;
/// Ships near the station fly at this share of their size, so it reads as large.
const SIZE_SHARE: f32 = 0.65;
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
        let flight = plan(&mut random);
        let duration = flight.duration();
        let start = between(&mut random, 0.0, SLOT_SECONDS - duration);
        let progress = (into_slot - start) / duration;
        if (0.0..=1.0).contains(&progress) {
            flying.push((flight, progress));
        }
    }
    flying
}

/// One station trip, fitted to finish within a slot.
fn plan(random: &mut fastrand::Rng) -> Flight {
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
    // Growing and shrinking span the slide through the hull and a little open
    // space outside it, so it takes a few seconds at a crawl.
    let doorway = slide + size;
    // Ships use a straight corridor along the station's axis: arrivals turn
    // into it (above the screen) before braking, so the flip, the burn and the
    // creep all happen in a straight line; departures climb out along it.
    let corridor = |length: f32| dock() + station_axis() * length;
    let arrival = corridor(BRAKE_DISTANCE + doorway + 6.0);
    let departure = corridor(PULL_AWAY_DISTANCE + doorway);
    let descent = corridor(CORRIDOR);
    let (points, grow_over, shrink_over) = match leg {
        Leg::Arrive => (vec![far, arrival, dock(), inside], 0.0, doorway),
        Leg::Depart => (vec![inside, dock(), departure, far], doorway, 0.0),
        // Shrinking starts a little above the surface and ends deep below it.
        _ => (
            vec![inside, dock(), descent, surface, sunk],
            doorway,
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
        motion: Motion::steady(speed),
    };
    // Final approaches and the first moments out of the hull are slow and
    // steady, about `CREEP_SECONDS` long, while the ship shrinks or grows.
    let creep = |distance: f32| (distance / CREEP_SECONDS).max(MIN_CRAWL);
    let motion = match leg {
        // Flip, burn to a crawl at the doorway, flip back, creep in.
        Leg::Arrive => Motion {
            end_speed: creep(doorway),
            brake: BRAKE_DISTANCE,
            creep_in: doorway,
            ..flight.motion
        },
        // Creep out of the hull, then pull away.
        Leg::Depart => Motion {
            start_speed: creep(doorway),
            creep_out: doorway,
            ramp: PULL_AWAY_DISTANCE,
            ..flight.motion
        },
        // Creep out, cruise down, flip, burn and sink tail-first.
        _ => Motion {
            start_speed: creep(doorway),
            creep_out: doorway,
            ramp: PULL_AWAY_DISTANCE * 0.5,
            end_speed: creep(shrink_over),
            brake: BRAKE_DISTANCE * 0.6,
            creep_in: shrink_over,
            ..flight.motion
        },
    };
    let length = flight.length();
    Flight {
        motion: motion.fitted(length, SLOT_SECONDS - 1.0),
        ..flight
    }
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
                        assert!(flight.size_at(1.0) < 1e-4);
                    }
                    Leg::Depart => {
                        assert!(start.distance(dock()) > 0.5, "starts inside the hull");
                        assert!(flight.size_at(0.0) < 1e-4);
                    }
                    Leg::Descend => {
                        assert!(
                            end.distance(PLANET_CENTRE) < PLANET_RADIUS,
                            "ends below the surface"
                        );
                        assert!(flight.size_at(0.0) < 1e-4);
                        assert!(flight.size_at(1.0) < 1e-4);
                    }
                    Leg::Cross => unreachable!("station lanes never cross"),
                }
                // Size changes are gradual: under a fifth of the ship per
                // tenth of a second, so it takes at least half a second.
                let steps = (flight.duration() * 10.0).ceil() as usize;
                for step in 0..steps {
                    let (a, b) = (step as f32 / steps as f32, (step + 1) as f32 / steps as f32);
                    let change = (flight.size_at(a) - flight.size_at(b)).abs();
                    assert!(change < flight.size * 0.2, "{:?} jumps at {a}", flight.leg);
                }
            }
        }
        assert_eq!(legs.len(), 3, "arrivals, departures and descents all occur");
    }

    #[test]
    fn an_arrival_flips_to_burn_then_flips_back_for_the_final_creep() {
        let mut random = fastrand::Rng::with_seed(1);
        let flight = std::iter::repeat_with(|| plan(&mut random))
            .find(|flight| flight.leg == Leg::Arrive)
            .unwrap();
        let mut seen = (false, false, false, false);
        let steps = (flight.duration() * 20.0) as usize;
        for step in 1..steps {
            let progress = step as f32 / steps as f32;
            let travel =
                (flight.position(progress + 0.002) - flight.position(progress - 0.002)).normalize();
            let nose = flight.rotation(progress) * Vec3::Y;
            let engines = flight.engines(progress);
            if engines.main > 0.5 {
                assert!(nose.dot(travel) > 0.95, "cruising nose-first");
                seen.0 = true;
            }
            if engines.burn > 0.99 {
                assert!(nose.dot(travel) < -0.95, "burning tail-first");
                seen.1 = true;
            }
            if engines.retro > 0.99 {
                assert!(nose.dot(travel) > 0.95, "creeping in nose-first");
                seen.2 = true;
            }
            let turning = flight.flip(progress);
            if turning > 0.2 && turning < 0.8 {
                assert_eq!(
                    engines.main + engines.burn + engines.retro,
                    0.0,
                    "coasts while turning"
                );
                seen.3 = true;
            }
        }
        assert_eq!(
            seen,
            (true, true, true, true),
            "cruise, burn, retro and flips all happen"
        );
    }
}
