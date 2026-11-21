//! Generated routes for the main menu's ships. Each of `LANES` lanes flies one
//! flight per `SLOT_SECONDS` slot, drawn from a generator seeded by the launch,
//! the lane and the slot: the ship type, which way it crosses, the depth and
//! height at each end (so the angle varies), an arc and a bank.
//!
//! Routes run sideways far past the screen (`REACH` world units each side of
//! the scene), so every ship enters and leaves off screen whatever the window
//! shape, and resizing never moves a route. Scene space is `menu_scene`'s.
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

pub(super) struct Flight {
    pub kind: ShipKind,
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
    /// Route position at `progress` (0 at the start, 1 at the end).
    pub fn position(&self, progress: f32) -> Vec3 {
        let across = (self.to - self.from).cross(Vec3::Z).normalize_or_zero();
        self.from.lerp(self.to, progress) + across * self.arc * (progress * PI).sin()
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
    let mut flying = Vec::new();
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
    let between =
        |random: &mut fastrand::Rng, low: f32, high: f32| low + random.f32() * (high - low);
    // Speeds are cruise speeds in world units per second.
    let (kind, size, speed) = match random.u8(0..10) {
        0..=3 => (ShipKind::Freighter, 9.0, 11.0),
        4..=6 => (ShipKind::Patrol, 5.0, 18.0),
        7..=8 => (ShipKind::Scout, 3.5, 30.0),
        _ => (ShipKind::Prospector, 3.0, 14.0),
    };
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
}
