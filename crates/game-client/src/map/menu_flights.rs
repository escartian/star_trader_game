//! Generated routes for the main menu's ships. Each of `LANES` lanes flies one
//! flight per `SLOT_SECONDS` slot, drawn from a generator seeded by the launch,
//! the lane and the slot: the ship type, which way it crosses, the depth and
//! height at each end (so the angle varies), an arc and a bank.
//!
//! Station lanes add traffic to the trade station: ships arriving and slowing
//! into its bay, leaving it, or descending from it to the planet.
//!
//! Crossing routes run sideways far past the screen (`REACH` world units each side of
//! the scene), so every ship enters and leaves off screen whatever the window
//! shape, and resizing never moves a route. Scene space is `menu_scene`'s.
use super::menu_scene::{PLANET_CENTRE, PLANET_RADIUS, STATION};
use game_core::fleet::ShipKind;
use glam::{Mat3, Vec3};
use std::f32::consts::PI;

const LANES: u64 = 8;
/// Seconds each lane's flight slot lasts; a flight fits inside one slot.
const SLOT_SECONDS: f32 = 36.0;
/// Routes start and end this far left or right of the scene centre: past the
/// screen edge at every depth used, for windows up to about 2.6:1 (portrait
/// screens pull the camera back, see `menu_scene::eye`).
const REACH: f32 = 220.0;
/// Seconds lanes are shifted against each other, so flights interleave.
const LANE_STAGGER: f32 = 4.7;
/// Lanes of station traffic, each flying one trip per `STATION_SLOT_SECONDS`.
const STATION_LANES: u64 = 3;
const STATION_SLOT_SECONDS: f32 = 22.0;
/// Share of an arrival or departure spent passing through the bay doors,
/// where the ship shrinks into (or grows out of) the station.
const BAY_SHARE: f32 = 0.12;
/// Share of a descent spent entering the atmosphere, shrinking to nothing.
const ENTRY_SHARE: f32 = 0.3;

/// What a flight does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Leg {
    /// Crosses the whole view at an even speed.
    Cross,
    /// Comes in from off screen and slows into the station's bay.
    Arrive,
    /// Eases out of the bay and speeds off screen.
    Depart,
    /// Drops from the station down into the planet's atmosphere.
    Descend,
}

pub(super) struct Flight {
    pub kind: ShipKind,
    pub leg: Leg,
    from: Vec3,
    to: Vec3,
    /// Sideways bow of the route at its middle, world units (horizontal).
    arc: f32,
    /// Ship length in world units.
    pub size: f32,
    pub bank: f32,
    /// A per-flight number for trail flicker, so trails do not pulse together.
    pub phase: f32,
}

impl Flight {
    /// Route position at `progress` (0 at the start, 1 at the end, linear in
    /// time). Station legs ease: arrivals brake, departures accelerate.
    pub fn position(&self, progress: f32) -> Vec3 {
        let travelled = match self.leg {
            Leg::Cross => progress,
            Leg::Arrive => 1.0 - (1.0 - progress).powi(2),
            Leg::Depart => progress.powi(2),
            Leg::Descend => progress * progress * (3.0 - 2.0 * progress),
        };
        let across = (self.to - self.from).cross(Vec3::Z).normalize_or_zero();
        self.from.lerp(self.to, travelled) + across * self.arc * (travelled * PI).sin()
    }

    /// Ship length at `progress`, in world units: shrinking into the bay or the
    /// atmosphere, growing out of the bay.
    pub fn size_at(&self, progress: f32) -> f32 {
        let factor = match self.leg {
            Leg::Cross => 1.0,
            Leg::Arrive => (1.0 - progress) / BAY_SHARE,
            Leg::Depart => progress / BAY_SHARE,
            Leg::Descend => (1.0 - progress) / ENTRY_SHARE,
        };
        self.size * factor.clamp(0.0, 1.0)
    }

    /// Route length in world units (the arc's extra is small).
    pub fn length(&self) -> f32 {
        self.from.distance(self.to)
    }

    /// Model rotation at `progress`: nose along the route, rolled by `bank`.
    pub fn rotation(&self, progress: f32) -> Mat3 {
        let ahead = self.position((progress + 0.01).min(1.0));
        let behind = self.position((progress - 0.01).max(0.0));
        orientation(ahead - behind, self.bank)
    }
}

/// Model rotation for a ship flying along `forward`, rolled by `bank` radians.
/// Model space has the nose along +y and the top along +z.
pub(super) fn orientation(forward: Vec3, bank: f32) -> Mat3 {
    let forward = forward.normalize();
    let right = forward.cross(Vec3::Z).normalize();
    let up = right.cross(forward);
    let roll = Mat3::from_axis_angle(forward, bank);
    Mat3::from_cols(roll * right, forward, roll * up)
}

/// Flights in the air at `time` seconds, each with its progress (0-1).
pub(super) fn active(time: f32, launch_seed: u64) -> Vec<(Flight, f32)> {
    let mut flying = station_traffic(time, launch_seed);
    for lane in 0..LANES {
        let lane_time = time + lane as f32 * LANE_STAGGER;
        let slot = (lane_time / SLOT_SECONDS).floor().max(0.0) as u64;
        let into_slot = lane_time - slot as f32 * SLOT_SECONDS;
        let mut random = fastrand::Rng::with_seed(launch_seed ^ (lane << 48) ^ slot);
        let (lead, duration, start) = plan(&mut random);
        let progress = (into_slot - start) / duration;
        if !(0.0..=1.0).contains(&progress) {
            continue;
        }
        // Patrols fly as a pair: the wingman trails behind and to one side.
        if lead.kind == ShipKind::Patrol {
            let back = (lead.from - lead.to).normalize() * 7.0;
            let side = Vec3::new(0.0, 5.0, -1.5);
            let wingman = Flight {
                from: lead.from + back + side,
                to: lead.to + back + side,
                phase: lead.phase + 1.7,
                ..lead
            };
            flying.push((wingman, progress));
        }
        flying.push((lead, progress));
    }
    flying
}

/// One flight for a slot: the ship, seconds in the air and seconds into the
/// slot it starts.
fn plan(random: &mut fastrand::Rng) -> (Flight, f32, f32) {
    let (kind, size, speed) = pick_ship(random);
    let direction = if random.bool() { 1.0 } else { -1.0 };
    // Depth (y) from just in front of the planet to far behind it; heights
    // stay within the view so ships leave by the sides.
    let end = |random: &mut fastrand::Rng, x: f32| {
        Vec3::new(
            x,
            between(random, 15.0, 150.0),
            between(random, -18.0, 34.0),
        )
    };
    let from = end(random, -REACH * direction);
    let to = end(random, REACH * direction);
    let flight = Flight {
        kind,
        leg: Leg::Cross,
        from,
        to,
        arc: between(random, -30.0, 30.0),
        size,
        bank: between(random, -0.35, 0.35),
        phase: between(random, 0.0, 100.0),
    };
    let duration = (flight.length() / speed).min(SLOT_SECONDS - 2.0);
    let start = between(random, 0.0, SLOT_SECONDS - duration);
    (flight, duration, start)
}

fn between(random: &mut fastrand::Rng, low: f32, high: f32) -> f32 {
    low + random.f32() * (high - low)
}

/// A ship type with its length and cruise speed (world units, units/second).
fn pick_ship(random: &mut fastrand::Rng) -> (ShipKind, f32, f32) {
    match random.u8(0..10) {
        0..=3 => (ShipKind::Freighter, 9.0, 11.0),
        4..=6 => (ShipKind::Patrol, 5.0, 18.0),
        7..=8 => (ShipKind::Scout, 3.5, 30.0),
        _ => (ShipKind::Prospector, 3.0, 14.0),
    }
}

/// Ships arriving at, leaving or descending from the station at `time`.
fn station_traffic(time: f32, launch_seed: u64) -> Vec<(Flight, f32)> {
    let mut flying = Vec::new();
    for lane in 0..STATION_LANES {
        let lane_time = time + lane as f32 * STATION_SLOT_SECONDS / STATION_LANES as f32;
        let slot = (lane_time / STATION_SLOT_SECONDS).floor().max(0.0) as u64;
        let into_slot = lane_time - slot as f32 * STATION_SLOT_SECONDS;
        let seed = launch_seed.rotate_left(17) ^ (lane << 40) ^ slot ^ 0x57A7_1011;
        let mut random = fastrand::Rng::with_seed(seed);
        let (kind, size, speed) = pick_ship(&mut random);
        let leg = match random.u8(0..10) {
            0..=3 => Leg::Arrive,
            4..=6 => Leg::Depart,
            _ => Leg::Descend,
        };
        // Off-screen ends come in from either side at varied depth and height.
        let side = if random.bool() { 1.0 } else { -1.0 };
        let far = Vec3::new(
            220.0 * side,
            between(&mut random, 20.0, 140.0),
            between(&mut random, -10.0, 34.0),
        );
        // Descents aim at the planet's near face, high on the side facing the station.
        let entry = PLANET_CENTRE + PLANET_RADIUS * Vec3::new(-0.45, -0.75, 0.48).normalize() * 0.9;
        let (from, to) = match leg {
            Leg::Arrive => (far, STATION),
            Leg::Depart => (STATION, far),
            _ => (STATION, entry),
        };
        let flight = Flight {
            kind,
            leg,
            from,
            to,
            // Station legs are short: a slight bow, not a loop.
            arc: between(&mut random, -3.0, 3.0),
            size,
            bank: between(&mut random, -0.2, 0.2),
            phase: between(&mut random, 0.0, 100.0),
        };
        // Easing halves the average speed, so allow double the cruise time.
        let duration = (2.0 * flight.length() / speed).clamp(6.0, STATION_SLOT_SECONDS - 1.0);
        let start = between(&mut random, 0.0, STATION_SLOT_SECONDS - duration);
        let progress = (into_slot - start) / duration;
        if (0.0..=1.0).contains(&progress) {
            flying.push((flight, progress));
        }
    }
    flying
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ship_flying_along_x_keeps_its_nose_forward_and_its_top_up() {
        let rotation = orientation(Vec3::X, 0.0);
        assert!((rotation * Vec3::Y).abs_diff_eq(Vec3::X, 1e-5));
        assert!((rotation * Vec3::Z).abs_diff_eq(Vec3::Z, 1e-5));
    }

    #[test]
    fn flights_vary_start_and_end_far_off_to_the_sides_and_repeat_per_seed() {
        let mut headings = Vec::new();
        for second in (0..600).step_by(3) {
            for (flight, progress) in active(second as f32, 7) {
                assert!((0.0..=1.0).contains(&progress));
                if flight.leg != Leg::Cross {
                    continue;
                }
                assert!(flight.position(0.0).x.abs() > 200.0);
                assert!(flight.position(1.0).x.abs() > 200.0);
                headings.push((flight.to - flight.from).normalize());
            }
        }
        assert!(headings.len() > 50, "lanes keep ships in the air");
        let first = headings[0];
        assert!(
            headings.iter().any(|heading| heading.dot(first) < 0.99),
            "angles vary"
        );
        let again: Vec<_> = active(123.0, 7)
            .iter()
            .map(|(flight, _)| flight.from)
            .collect();
        let before: Vec<_> = active(123.0, 7)
            .iter()
            .map(|(flight, _)| flight.from)
            .collect();
        assert_eq!(again, before, "the same launch replays the same sky");
    }

    #[test]
    fn station_ships_shrink_into_the_bay_and_the_atmosphere() {
        let mut legs = Vec::new();
        for second in 0..900 {
            for (flight, _) in station_traffic(second as f32, 3) {
                if !legs.contains(&flight.leg) {
                    legs.push(flight.leg);
                }
                match flight.leg {
                    Leg::Arrive => {
                        assert!(flight.position(1.0).distance(STATION) < 0.01);
                        assert_eq!(flight.size_at(1.0), 0.0);
                        assert_eq!(flight.size_at(0.5), flight.size);
                    }
                    Leg::Depart => {
                        assert!(flight.position(0.0).distance(STATION) < 0.01);
                        assert_eq!(flight.size_at(0.0), 0.0);
                    }
                    Leg::Descend => {
                        let end = flight.position(1.0);
                        assert!((end.distance(PLANET_CENTRE) - PLANET_RADIUS * 0.9).abs() < 0.01);
                        assert_eq!(flight.size_at(1.0), 0.0);
                    }
                    Leg::Cross => unreachable!("station lanes never cross"),
                }
            }
        }
        assert_eq!(legs.len(), 3, "arrivals, departures and descents all occur");
    }
}
