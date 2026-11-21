//! Speed profiles for the main menu's ships, in five phases along the route:
//! creep out at `start_speed` for `creep_out` units, speed up over `ramp`,
//! cruise, brake evenly over `brake`, then creep in at `end_speed` for the last
//! `creep_in` units. Docking ships brake to a crawl before their final approach
//! into the station's bay; departing ships creep out and then pull away.
//! Distances are world units and speeds world units per second.

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Motion {
    pub cruise: f32,
    pub start_speed: f32,
    pub end_speed: f32,
    pub creep_out: f32,
    pub ramp: f32,
    pub brake: f32,
    pub creep_in: f32,
}

/// Index of the braking phase, in route order (creep out, ramp, cruise,
/// brake, creep in).
const BRAKE: usize = 3;

/// One phase: distance, and speed at its start and end (even acceleration).
#[derive(Clone, Copy)]
struct Phase {
    distance: f32,
    from: f32,
    to: f32,
}

impl Phase {
    fn seconds(&self) -> f32 {
        if self.distance <= 0.0 {
            0.0
        } else {
            2.0 * self.distance / (self.from + self.to).max(f32::EPSILON)
        }
    }
}

impl Motion {
    /// Constant speed for the whole route.
    pub fn steady(cruise: f32) -> Self {
        Self {
            cruise,
            start_speed: cruise,
            end_speed: cruise,
            creep_out: 0.0,
            ramp: 0.0,
            brake: 0.0,
            creep_in: 0.0,
        }
    }

    /// The five phases for a route of `length` units. When the route is shorter
    /// than the creeps, ramp and brake together, they shrink in proportion.
    fn phases(&self, length: f32) -> [Phase; 5] {
        let fixed = self.creep_out + self.ramp + self.brake + self.creep_in;
        let fit = (length / fixed.max(f32::EPSILON)).min(1.0);
        let (start, end, cruise) = (self.start_speed, self.end_speed, self.cruise);
        let phase = |distance: f32, from, to| Phase {
            distance: distance * fit,
            from,
            to,
        };
        [
            phase(self.creep_out, start, start),
            phase(self.ramp, start, cruise),
            Phase {
                distance: (length - fixed * fit).max(0.0),
                from: cruise,
                to: cruise,
            },
            phase(self.brake, cruise, end),
            phase(self.creep_in, end, end),
        ]
    }

    /// Seconds each phase starts at on a route of `length` units, and the end.
    fn phase_starts(&self, length: f32) -> [f32; 6] {
        let mut starts = [0.0; 6];
        for (index, phase) in self.phases(length).iter().enumerate() {
            starts[index + 1] = starts[index] + phase.seconds();
        }
        starts
    }

    /// When braking starts and when the final creep starts, in seconds.
    pub fn braking(&self, length: f32) -> (f32, f32) {
        let starts = self.phase_starts(length);
        (starts[BRAKE], starts[BRAKE + 1])
    }

    /// Seconds to fly a route of `length` units.
    pub fn duration(&self, length: f32) -> f32 {
        self.phase_starts(length)[5]
    }

    /// Distance flown and speed after `time` seconds on a route of `length`.
    fn state_at(&self, time: f32, length: f32) -> (f32, f32) {
        let mut covered = 0.0;
        let mut time = time.max(0.0);
        let mut speed = self.start_speed;
        for phase in self.phases(length) {
            let seconds = phase.seconds();
            if time < seconds {
                let accel = (phase.to - phase.from) / seconds;
                let distance = phase.from * time + accel * time * time / 2.0;
                return ((covered + distance).min(length), phase.from + accel * time);
            }
            covered += phase.distance;
            time -= seconds;
            speed = phase.to;
        }
        (length, speed)
    }

    /// Distance flown after `time` seconds on a route of `length` units.
    pub fn distance_at(&self, time: f32, length: f32) -> f32 {
        self.state_at(time, length).0
    }

    /// Speed after `time` seconds on a route of `length` units.
    pub fn speed_at(&self, time: f32, length: f32) -> f32 {
        self.state_at(time, length).1
    }

    /// The same profile with cruise speed raised so the route takes at most
    /// `seconds` (creeps keep their speeds, so they cannot be hurried).
    pub fn fitted(mut self, length: f32, seconds: f32) -> Self {
        for _ in 0..12 {
            let starts = self.phase_starts(length);
            let duration = starts[5];
            if duration <= seconds {
                break;
            }
            let creeping = (starts[1] - starts[0]) + (starts[5] - starts[4]);
            let flexible = (duration - creeping).max(f32::EPSILON);
            let wanted = (seconds - creeping).max(flexible * 0.25);
            self.cruise *= flexible / wanted;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_docking_ship_brakes_then_creeps_in_and_covers_the_whole_route() {
        let motion = Motion {
            end_speed: 1.5,
            brake: 40.0,
            creep_in: 4.5,
            ..Motion::steady(12.0)
        };
        let length = 200.0;
        let end = motion.duration(length);
        assert!((motion.distance_at(end, length) - length).abs() < 1e-3);
        assert_eq!(motion.speed_at(0.0, length), 12.0);
        let (brake, creep) = motion.braking(length);
        assert!((motion.speed_at(creep + 0.1, length) - 1.5).abs() < 1e-3);
        assert!(brake < creep, "brakes before the final approach");
        assert!((end - creep - 3.0).abs() < 1e-3, "a 3 s final approach");
        let mut previous = 0.0;
        for step in 0..=200 {
            let distance = motion.distance_at(end * step as f32 / 200.0, length);
            assert!(distance >= previous, "never runs backwards");
            previous = distance;
        }
    }

    #[test]
    fn fitting_speeds_up_the_cruise_but_not_the_creeps() {
        let motion = Motion {
            start_speed: 1.0,
            creep_out: 3.0,
            ramp: 20.0,
            ..Motion::steady(5.0)
        };
        let fitted = motion.fitted(300.0, 20.0);
        assert!(fitted.duration(300.0) <= 20.1);
        assert!(fitted.cruise > 5.0);
        assert_eq!(fitted.start_speed, 1.0);
    }
}
