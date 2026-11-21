//! Routes for the main menu's ships. A flight follows straight legs between
//! waypoints; its size can shrink over the final stretch (sliding into the
//! station's hull or below the planet's surface) and grow over the first (out
//! of the station), so ships never pop in or out.
//!
//! Crossing traffic: each of `LANES` lanes flies one flight per `SLOT_SECONDS`
//! slot, drawn from a generator seeded by the launch, the lane and the slot:
//! the ship type, which way it crosses, the depth and height at each end (so
//! the angle varies), an arc and a bank. Crossing routes run sideways far past
//! the screen (`REACH` world units each side), so ships enter and leave off
//! screen whatever the window shape or camera swing. Station traffic is in
//! `menu_docking`. Scene space is `menu_scene`'s.
use super::menu_motion::Motion;
use game_core::fleet::ShipKind;
use glam::{Mat3, Vec3};
use std::f32::consts::PI;

const LANES: u64 = 4;
/// Seconds each lane's flight slot lasts; a flight fits inside one slot.
const SLOT_SECONDS: f32 = 36.0;
/// Routes start and end this far left or right of the scene centre: past the
/// screen edge at every depth used, for windows up to about 2.6:1 (portrait
/// screens pull the camera back, see `menu_scene::camera`).
const REACH: f32 = 220.0;
/// Seconds lanes are shifted against each other, so flights interleave.
const LANE_STAGGER: f32 = 9.1;

/// Seconds a ship takes to turn end over end.
const FLIP_SECONDS: f32 = 1.8;

/// What a ship's engines are doing at a moment, each 0 (off) to 1 (full).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Engines {
    /// Main engine pushing the ship along its route (the cruise trail).
    pub main: f32,
    /// Main engine firing toward the destination while flipped, to brake.
    pub burn: f32,
    /// Small thrusters on the nose braking the final approach.
    pub retro: f32,
}

/// What a flight does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Leg {
    /// Crosses the whole view at an even speed.
    Cross,
    /// Comes in from off screen, brakes and slides into the station's bay.
    Arrive,
    /// Eases out of the bay and speeds off screen.
    Depart,
    /// Leaves the bay and drops into the planet's atmosphere.
    Descend,
}

#[derive(Clone)]
pub(super) struct Flight {
    pub kind: ShipKind,
    pub leg: Leg,
    /// Waypoints in scene space; at least two.
    pub points: Vec<Vec3>,
    /// Sideways bow of a two-point route at its middle, world units.
    pub arc: f32,
    /// Full ship length in world units.
    pub size: f32,
    pub bank: f32,
    /// A per-flight number for trail flicker, so trails do not pulse together.
    pub phase: f32,
    /// World units over which the ship grows at the start (0: full size from
    /// the start) and shrinks at the end (0: full size to the end).
    pub grow_over: f32,
    pub shrink_over: f32,
    /// How fast it flies along the route over time.
    pub motion: Motion,
}

/// Smooth 0-to-1 ramp of `x` (clamped).
fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Flight {
    /// Seconds the whole route takes.
    pub fn duration(&self) -> f32 {
        self.motion.duration(self.length())
    }

    /// Distance flown at `progress` (0-1, linear in time), following `motion`.
    fn travelled(&self, progress: f32) -> f32 {
        let time = progress.clamp(0.0, 1.0) * self.duration();
        self.motion.distance_at(time, self.length())
    }

    /// Current speed as a share of cruise speed (0-1): low while braking or
    /// pulling away, so the engine trail can shorten.
    pub fn throttle(&self, progress: f32) -> f32 {
        let time = progress.clamp(0.0, 1.0) * self.duration();
        (self.motion.speed_at(time, self.length()) / self.motion.cruise).clamp(0.0, 1.0)
    }

    /// Total route length in world units (the arc's extra is small).
    pub fn length(&self) -> f32 {
        self.points
            .windows(2)
            .map(|leg| leg[0].distance(leg[1]))
            .sum()
    }

    /// Route position at `progress` (0 at the start, 1 at the end).
    pub fn position(&self, progress: f32) -> Vec3 {
        let mut left = self.travelled(progress);
        let mut at = self.points[0];
        for leg in self.points.windows(2) {
            let span = leg[0].distance(leg[1]);
            if left <= span || span == 0.0 {
                at = leg[0].lerp(leg[1], (left / span.max(f32::EPSILON)).min(1.0));
                break;
            }
            left -= span;
            at = leg[1];
        }
        if self.points.len() == 2 {
            let (from, to) = (self.points[0], self.points[1]);
            let across = (to - from).cross(Vec3::Z).normalize_or_zero();
            let share = self.travelled(progress) / self.length().max(f32::EPSILON);
            at += across * self.arc * (share * PI).sin();
        }
        at
    }

    /// Ship length at `progress`, in world units.
    pub fn size_at(&self, progress: f32) -> f32 {
        let flown = self.travelled(progress);
        let remaining = self.length() - flown;
        let grow = if self.grow_over > 0.0 {
            smooth(flown / self.grow_over)
        } else {
            1.0
        };
        let shrink = if self.shrink_over > 0.0 {
            smooth(remaining / self.shrink_over)
        } else {
            1.0
        };
        self.size * grow * shrink
    }

    /// Model rotation at `progress`: nose along the route, rolled by `bank`,
    /// turned end over end while flipped to brake.
    pub fn rotation(&self, progress: f32) -> Mat3 {
        let ahead = self.position((progress + 0.01).min(1.0));
        let behind = self.position((progress - 0.01).max(0.0));
        let flip = Mat3::from_rotation_z(PI * smooth(self.flip(progress)));
        orientation(ahead - behind, self.bank) * flip
    }

    /// Seconds into the route at `progress`, when braking starts, when the
    /// final creep starts, and the flip time (shorter for brief brakes).
    fn timeline(&self, progress: f32) -> (f32, f32, f32, f32) {
        let time = progress.clamp(0.0, 1.0) * self.duration();
        let (brake, creep) = self.motion.braking(self.length());
        (time, brake, creep, FLIP_SECONDS.min((creep - brake) * 0.4))
    }

    /// How far the ship has turned around to brake: 0 nose-first, 1 tail-first.
    /// Arrivals and descents flip while coasting just before braking; arrivals
    /// flip back just before the final creep; descents land tail-first.
    pub fn flip(&self, progress: f32) -> f32 {
        if !matches!(self.leg, Leg::Arrive | Leg::Descend) {
            return 0.0;
        }
        let (time, brake, creep, flip) = self.timeline(progress);
        if flip <= 0.0 {
            return 0.0;
        }
        let turned = ((time - (brake - flip)) / flip).clamp(0.0, 1.0);
        if self.leg == Leg::Descend {
            return turned;
        }
        turned - ((time - (creep - flip)) / flip).clamp(0.0, 1.0)
    }

    /// Engine output at `progress`: cruising on the main engine, coasting
    /// through flips, burning while tail-first, retro thrusters on the final
    /// creep into the station.
    pub fn engines(&self, progress: f32) -> Engines {
        let cruising = Engines {
            main: self.throttle(progress),
            ..Engines::default()
        };
        if !matches!(self.leg, Leg::Arrive | Leg::Descend) {
            return cruising;
        }
        let (time, brake, creep, flip) = self.timeline(progress);
        if flip <= 0.0 || time < brake - flip {
            return cruising;
        }
        let turning = self.flip(progress);
        // Engines are off while the ship turns; they fire once it has settled.
        let settled = |amount: f32| smooth((amount - 0.85) / 0.15);
        if self.leg == Leg::Descend || time < creep - flip {
            return Engines {
                burn: settled(turning),
                ..Engines::default()
            };
        }
        Engines {
            retro: settled(1.0 - turning),
            ..Engines::default()
        }
    }
}

/// Model rotation for a ship flying along `forward`, rolled by `bank` radians.
/// Model space has the nose along +y and the top along +z.
pub(super) fn orientation(forward: Vec3, bank: f32) -> Mat3 {
    let forward = forward.normalize_or(Vec3::Y);
    let right = forward.cross(Vec3::Z).normalize_or(Vec3::X);
    let up = right.cross(forward);
    let roll = Mat3::from_axis_angle(forward, bank);
    Mat3::from_cols(roll * right, forward, roll * up)
}

/// Flights in the air at `time` seconds, each with its progress (0-1).
pub(super) fn active(time: f32, launch_seed: u64) -> Vec<(Flight, f32)> {
    let mut flying = super::menu_docking::traffic(time, launch_seed);
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
            let back = (lead.points[0] - lead.points[1]).normalize() * 7.0;
            let offset = back + Vec3::new(0.0, 5.0, -1.5);
            let wingman = Flight {
                points: lead.points.iter().map(|point| *point + offset).collect(),
                phase: lead.phase + 1.7,
                ..lead.clone()
            };
            flying.push((wingman, progress));
        }
        flying.push((lead, progress));
    }
    flying
}

/// One crossing flight for a slot: the ship, seconds in the air and seconds
/// into the slot it starts.
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
        points: vec![from, to],
        arc: between(random, -30.0, 30.0),
        size,
        bank: between(random, -0.35, 0.35),
        phase: between(random, 0.0, 100.0),
        grow_over: 0.0,
        shrink_over: 0.0,
        motion: Motion::steady(speed),
    };
    let flight = Flight {
        motion: flight.motion.fitted(flight.length(), SLOT_SECONDS - 2.0),
        ..flight
    };
    let duration = flight.duration();
    let start = between(random, 0.0, SLOT_SECONDS - duration);
    (flight, duration, start)
}

pub(super) fn between(random: &mut fastrand::Rng, low: f32, high: f32) -> f32 {
    low + random.f32() * (high - low)
}

/// A ship type with its length and cruise speed (world units, units/second).
pub(super) fn pick_ship(random: &mut fastrand::Rng) -> (ShipKind, f32, f32) {
    match random.u8(0..10) {
        0..=3 => (ShipKind::Freighter, 9.0, 11.0),
        4..=6 => (ShipKind::Patrol, 5.0, 18.0),
        7..=8 => (ShipKind::Scout, 3.5, 30.0),
        _ => (ShipKind::Prospector, 3.0, 14.0),
    }
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
                headings.push((flight.points[1] - flight.points[0]).normalize());
            }
        }
        assert!(headings.len() > 30, "lanes keep ships in the air");
        let first = headings[0];
        assert!(
            headings.iter().any(|heading| heading.dot(first) < 0.99),
            "angles vary"
        );
        let starts = |time| -> Vec<Vec3> {
            active(time, 7)
                .iter()
                .map(|(flight, _)| flight.points[0])
                .collect()
        };
        assert_eq!(
            starts(123.0),
            starts(123.0),
            "the same launch replays the same sky"
        );
    }

    #[test]
    fn a_waypoint_route_passes_its_corners_and_sizes_change_smoothly() {
        let flight = Flight {
            kind: ShipKind::Scout,
            leg: Leg::Cross,
            points: vec![Vec3::ZERO, Vec3::X * 10.0, Vec3::new(10.0, 10.0, 0.0)],
            arc: 0.0,
            size: 4.0,
            bank: 0.0,
            phase: 0.0,
            grow_over: 5.0,
            shrink_over: 5.0,
            motion: Motion::steady(4.0),
        };
        assert!(flight.position(0.5).abs_diff_eq(Vec3::X * 10.0, 1e-4));
        assert_eq!(flight.size_at(0.0), 0.0);
        assert_eq!(flight.size_at(0.5), 4.0);
        assert_eq!(flight.size_at(1.0), 0.0);
        let mut previous = 0.0;
        for step in 1..=10 {
            let size = flight.size_at(step as f32 * 0.02);
            assert!(size >= previous && size - previous < 1.0, "grows gradually");
            previous = size;
        }
    }
}
