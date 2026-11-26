//! Glides along chase curves (`game_core::navigation::chase`): a flagship
//! flying an in-system pursuit, and raider bands and police patrols hunting
//! your fleets, slide through each step of their last run's path instead of
//! straight from start to end. Bands and patrols that did not chase slide
//! straight. Display only, like the rest of `motion`.
use super::LocalSession;
use game_core::{
    LocalPosition, StarSystem, movement::SpacePosition, navigation::RaiderBand,
    police::FactionPatrol, travel::ShipId,
};

/// The point `progress` (0 to 1) of the way along `points`, which are equally
/// spaced in time (one per chase step after the start).
fn along(points: &[[i64; 3]], progress: f64) -> [i64; 3] {
    let Some(last) = points.last() else {
        return [0; 3];
    };
    if points.len() == 1 || progress >= 1.0 {
        return *last;
    }
    let scaled = progress.max(0.0) * (points.len() - 1) as f64;
    let index = (scaled.floor() as usize).min(points.len() - 2);
    let part = scaled - index as f64;
    let (a, b) = (points[index], points[index + 1]);
    [0, 1, 2].map(|axis| a[axis] + ((b[axis] - a[axis]) as f64 * part).round() as i64)
}

fn local(at: LocalPosition) -> [i64; 3] {
    [at.x_km, at.y_km, at.z_km]
}

fn from_local([x_km, y_km, z_km]: [i64; 3]) -> LocalPosition {
    LocalPosition { x_km, y_km, z_km }
}

/// `before`, then each trail point (or just `now` without a trail).
fn path(before: LocalPosition, trail: &[LocalPosition], now: LocalPosition) -> Vec<[i64; 3]> {
    let mut points = vec![local(before)];
    if trail.is_empty() {
        points.push(local(now));
    } else {
        points.extend(trail.iter().copied().map(local));
    }
    points
}

impl LocalSession {
    /// Where an own ship flying this run's chase curve is drawn, if it is one.
    pub(super) fn chase_glide(
        &self,
        ship: ShipId,
        systems: &[StarSystem],
    ) -> Option<SpacePosition> {
        let motion = self.motion.as_ref()?;
        let trail = &self.state.navigation.chase_trail;
        if trail.is_empty() {
            return None;
        }
        let chaser = self
            .state
            .navigation
            .pursuit
            .or(motion.before.navigation.pursuit)?
            .fleet;
        if self.state.fleet_for_ship(ship).ok()?.id != chaser {
            return None;
        }
        let system = self.state.ship(ship).ok()?.location.system()?;
        let world = systems.iter().find(|known| known.id == system)?;
        let centre = SpacePosition::from_galaxy(world.position).ok()?;
        let start = motion.before.navigation_position(ship, systems)?;
        let start = LocalPosition {
            x_km: start.x_km - centre.x_km,
            y_km: start.y_km - centre.y_km,
            z_km: start.z_km - centre.z_km,
        };
        let points = path(start, trail, *trail.last()?);
        let [x, y, z] = along(&points, self.glide_progress());
        Some(SpacePosition {
            x_km: centre.x_km + x,
            y_km: centre.y_km + y,
            z_km: centre.z_km + z,
        })
    }

    /// Where a raider band is drawn on the system map during the latest glide.
    pub fn band_display_position(&self, band: &RaiderBand) -> LocalPosition {
        let before = self.motion.as_ref().and_then(|motion| {
            motion
                .before
                .navigation
                .bands
                .iter()
                .find(|known| known.id == band.id && known.system == band.system)
        });
        match before {
            Some(before) if band.transit.is_none() && before.transit.is_none() => {
                from_local(along(
                    &path(before.position, &band.trail, band.position),
                    self.glide_progress(),
                ))
            }
            _ => band.position,
        }
    }

    /// Where a police patrol is drawn on the system map during the latest glide.
    pub fn patrol_display_position(&self, patrol: &FactionPatrol) -> LocalPosition {
        let before = self.motion.as_ref().and_then(|motion| {
            motion
                .before
                .navigation
                .police
                .patrol(patrol.id)
                .filter(|known| known.system == patrol.system)
        });
        match before {
            Some(before) => from_local(along(
                &path(before.position, &patrol.trail, patrol.position),
                self.glide_progress(),
            )),
            None => patrol.position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn along_walks_each_point_at_equal_time() {
        let points = [[0, 0, 0], [10, 0, 0], [10, 0, 10]];
        assert_eq!(along(&points, 0.0), [0, 0, 0]);
        assert_eq!(along(&points, 0.25), [5, 0, 0]);
        assert_eq!(along(&points, 0.5), [10, 0, 0]);
        assert_eq!(along(&points, 0.75), [10, 0, 5]);
        assert_eq!(along(&points, 1.0), [10, 0, 10]);
    }

    #[test]
    fn without_a_trail_the_path_is_straight() {
        let a = LocalPosition {
            x_km: 0,
            y_km: 0,
            z_km: 0,
        };
        let b = LocalPosition {
            x_km: 8,
            y_km: 0,
            z_km: 0,
        };
        assert_eq!(path(a, &[], b), vec![[0, 0, 0], [8, 0, 0]]);
        assert_eq!(along(&path(a, &[], b), 0.5), [4, 0, 0]);
    }

    fn at(x_km: i64, z_km: i64) -> LocalPosition {
        LocalPosition {
            x_km,
            y_km: 0,
            z_km,
        }
    }

    /// A host whose latest run moved the flagship from `start` along `trail`.
    fn chased(start: LocalPosition, trail: Vec<LocalPosition>) -> (LocalSession, Vec<StarSystem>) {
        use game_core::{
            SystemId,
            fleet::FleetId,
            pursuit::{Pursuit, Quarry},
            travel::Location,
        };
        let mut host = LocalSession::new(game_server::Session::default());
        let systems = game_core::demo_galaxy();
        let system = SystemId(1);
        let mut before = host.state.clone();
        before.ships[0].location = Location::InSystem {
            system,
            position: start,
        };
        let end = *trail.last().unwrap();
        host.state.ships[0].location = Location::InSystem {
            system,
            position: end,
        };
        host.state.navigation.chase_trail = trail;
        host.state.navigation.pursuit = Some(Pursuit {
            fleet: FleetId(1),
            quarry: Quarry::Trader(1),
            quarry_docked: false,
            seen: Some((system, end)),
        });
        host.motion = Some(super::super::motion::Motion::new(before, Vec::new()));
        host.frame(0.0, true);
        (host, systems)
    }

    #[test]
    fn the_flagship_glides_through_its_chase_curve_not_straight_across() {
        // A bent path: 8 steps east, then 8 steps north.
        let trail: Vec<_> = (1..=16)
            .map(|step| {
                if step <= 8 {
                    at(step * 10_000_000, 0)
                } else {
                    at(80_000_000, (step - 8) * 10_000_000)
                }
            })
            .collect();
        let (mut host, systems) = chased(at(0, 0), trail.clone());
        let centre = SpacePosition::from_galaxy(systems[0].position).unwrap();
        let shown = |host: &LocalSession| {
            let at = host
                .display_position(game_core::travel::ShipId(1), &systems)
                .unwrap();
            [
                at.x_km - centre.x_km,
                at.y_km - centre.y_km,
                at.z_km - centre.z_km,
            ]
        };
        assert_eq!(shown(&host), [0, 0, 0], "starts where it was");
        host.frame(super::super::motion::GLIDE_SECONDS / 2.0, true);
        // Halfway in time is the bend (80, 0), not the straight midpoint (40, 40).
        assert_eq!(shown(&host), [80_000_000, 0, 0]);
        host.frame(super::super::motion::GLIDE_SECONDS, true);
        assert_eq!(shown(&host), [80_000_000, 0, 80_000_000]);
    }

    #[test]
    fn a_hunting_band_glides_along_its_trail_and_a_roaming_one_straight() {
        let mut host = LocalSession::new(game_server::Session::default());
        let mut band = host.state.navigation.bands[0].clone();
        let mut before = host.state.clone();
        before.navigation.bands[0].position = at(0, 0);
        band.position = at(80_000_000, 80_000_000);
        band.transit = None;
        before.navigation.bands[0].transit = None;
        host.motion = Some(super::super::motion::Motion::new(before, Vec::new()));
        host.frame(0.0, true);
        host.frame(super::super::motion::GLIDE_SECONDS / 2.0, true);
        assert_eq!(
            host.band_display_position(&band),
            at(40_000_000, 40_000_000)
        );
        band.trail = (1..=16)
            .map(|step| {
                if step <= 8 {
                    at(step * 10_000_000, 0)
                } else {
                    at(80_000_000, (step - 8) * 10_000_000)
                }
            })
            .collect();
        assert_eq!(host.band_display_position(&band), at(80_000_000, 0));
    }
}
