//! Raider bands crossing between systems (see `raiders`).
//!
//! - **Pursuit:** a band whose hunted fleet has left its system follows it to
//!   the next system on the fleet's course, if that lies within
//!   [`PURSUIT_RANGE_LY`]. It keeps hunting the fleet, so the fleet's card warns.
//!   A fleet that slipped out of detection range inside the system, or that has
//!   no course into another system, is lost.
//! - **Roaming:** a band with nothing to chase and not backing off moves on one
//!   run in [`ROAM_ODDS`], to a random system within [`ROAM_RANGE_LY`].
//! - Never into the home (first) system, and never into a system that already
//!   holds [`MAX_BANDS_PER_SYSTEM`] bands, counting bands on their way there.
//! - The crossing takes one run per [`BAND_LY_PER_RUN`] light-years (half a
//!   fleet's deep-space speed), at least one. Meanwhile the band neither hunts
//!   nor strikes, and `system` already names its destination. It arrives at
//!   the destination's cube face on the side of its origin, then hunts or
//!   patrols as usual from the next run.
//!
//! Rolls come from the bands' own random stream (`band_roll`).
use super::{RaiderBand, band_motion::local_in};
use crate::{
    GalaxyPosition, LocalPosition, StarSystem, SystemId,
    fleet::FleetId,
    movement::MoveTarget,
    travel::{PlayerId, TravelState},
};
use serde::{Deserialize, Serialize};

/// Light-years a band crosses per run.
pub const BAND_LY_PER_RUN: u64 = 2;
/// Farthest system a band follows a fleet to, in light-years.
pub const PURSUIT_RANGE_LY: u64 = 40;
/// Farthest system a roaming band moves on to, in light-years.
pub const ROAM_RANGE_LY: u64 = 30;
/// An idle band moves on one run in this many.
pub const ROAM_ODDS: u64 = 40;
/// Most bands one system holds, counting bands on their way there.
pub const MAX_BANDS_PER_SYSTEM: usize = 2;
/// Longest crossing, in runs: the longer range at [`BAND_LY_PER_RUN`].
pub const MAX_TRANSIT_RUNS: u32 = PURSUIT_RANGE_LY.div_ceil(BAND_LY_PER_RUN) as u32;

/// A band on its way to another system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BandTransit {
    /// The system it left.
    pub from: SystemId,
    /// Runs the whole crossing takes (1..=[`MAX_TRANSIT_RUNS`]).
    pub total_runs: u32,
    /// Runs until it arrives (1..=`total_runs`).
    pub runs_left: u32,
}

impl BandTransit {
    /// How far along the crossing is, from 0 (just left) to 1 (arriving).
    pub fn progress(&self) -> f32 {
        (self.total_runs - self.runs_left) as f32 / self.total_runs as f32
    }
}

impl TravelState {
    /// Whether `system` can take another band.
    pub(super) fn band_room(&self, system: SystemId) -> bool {
        let present = self
            .navigation
            .bands
            .iter()
            .filter(|band| band.system == system)
            .count();
        present < MAX_BANDS_PER_SYSTEM
    }

    /// Spends one run of a band's crossing; true if it was in transit (the
    /// arrival run included: it acts again from the next run).
    pub(super) fn advance_transit(&mut self, index: usize) -> bool {
        let record = &mut self.navigation.bands[index];
        let Some(transit) = record.transit.as_mut() else {
            return false;
        };
        transit.runs_left = transit.runs_left.saturating_sub(1);
        if transit.runs_left == 0 {
            record.transit = None;
        }
        true
    }

    /// Where a band with no quarry this run crosses to, if anywhere: after
    /// the fleet it hunted last run, or roaming.
    pub(super) fn crossing(
        &mut self,
        band: &RaiderBand,
        player: PlayerId,
        systems: &[StarSystem],
    ) -> Option<SystemId> {
        if let Some(fleet) = band.hunting {
            return self.pursuit(band, fleet, player, systems);
        }
        if band.stalking.is_some() || band.cooldown > 0 {
            return None;
        }
        if !self.band_roll().is_multiple_of(ROAM_ODDS) {
            return None;
        }
        let from = systems.iter().find(|world| world.id == band.system)?;
        let choices: Vec<SystemId> = systems
            .iter()
            .skip(1)
            .filter(|world| world.id != band.system && self.band_room(world.id))
            .filter(|world| light_years(from.position, world.position) <= ROAM_RANGE_LY)
            .map(|world| world.id)
            .collect();
        if choices.is_empty() {
            return None;
        }
        let pick = self.band_roll() % choices.len() as u64;
        choices.get(pick as usize).copied()
    }

    /// The next system on `fleet`'s course, if the fleet has left the band's
    /// system and that system can be chased into.
    fn pursuit(
        &self,
        band: &RaiderBand,
        fleet: FleetId,
        player: PlayerId,
        systems: &[StarSystem],
    ) -> Option<SystemId> {
        let ship = *self
            .fleets
            .iter()
            .find(|known| known.id == fleet && known.owner == player)?
            .ships
            .first()?;
        let world = systems.iter().find(|known| known.id == band.system)?;
        let at = self.navigation_position(ship, systems)?;
        let half = self.movement_rules.system_half_extent_km;
        if local_in(world, at, half).is_some() {
            return None;
        }
        let order = self.navigation.order(fleet)?;
        let next = std::iter::once(&order.target)
            .chain(&order.remaining_waypoints)
            .find_map(|target| match target {
                MoveTarget::Body { system, .. }
                | MoveTarget::Local { system, .. }
                | MoveTarget::EnterSystem(system)
                    if *system != band.system =>
                {
                    Some(*system)
                }
                _ => None,
            })?;
        let home = systems.first()?.id;
        let to = systems.iter().find(|known| known.id == next)?;
        (next != home
            && self.band_room(next)
            && light_years(world.position, to.position) <= PURSUIT_RANGE_LY)
            .then_some(next)
    }

    /// Sends band `index` on its way to `to`.
    pub(super) fn send_band(&mut self, index: usize, to: SystemId, systems: &[StarSystem]) {
        let half = self.movement_rules.system_half_extent_km;
        let record = &mut self.navigation.bands[index];
        let (Some(from), Some(destination)) = (
            systems.iter().find(|world| world.id == record.system),
            systems.iter().find(|world| world.id == to),
        ) else {
            return;
        };
        let gap = light_years(from.position, destination.position);
        let runs = gap
            .div_ceil(BAND_LY_PER_RUN)
            .clamp(1, MAX_TRANSIT_RUNS.into());
        let arrival = entry_point(from.position, destination.position, half);
        record.transit = Some(BandTransit {
            from: from.id,
            total_runs: runs as u32,
            runs_left: runs as u32,
        });
        record.system = to;
        record.position = arrival;
        record.heading = arrival;
        record.stalking = None;
    }
}

/// Whole light-years between two systems (rounded down).
fn light_years(a: GalaxyPosition, b: GalaxyPosition) -> u64 {
    let squares: i128 = [(a.x, b.x), (a.y, b.y), (a.z, b.z)]
        .iter()
        .map(|(a, b)| (i128::from(*a) - i128::from(*b)).pow(2))
        .sum();
    u64::try_from((squares as u128).isqrt()).unwrap_or(u64::MAX)
}

/// Where a band from `from` arrives at `to`: on the plane, on the cube face
/// toward its origin, nine tenths of the way out from the star.
fn entry_point(from: GalaxyPosition, to: GalaxyPosition, half_extent_km: i64) -> LocalPosition {
    let (dx, dz) = (
        i128::from(from.x) - i128::from(to.x),
        i128::from(from.z) - i128::from(to.z),
    );
    let longest = dx.abs().max(dz.abs()).max(1);
    let edge = i128::from(half_extent_km) * 9 / 10;
    let along = |delta: i128| i64::try_from(delta * edge / longest).unwrap_or(0);
    LocalPosition {
        x_km: along(dx),
        y_km: 0,
        z_km: along(dz),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_arrive_on_the_face_toward_their_origin() {
        let to = GalaxyPosition { x: 0, y: 0, z: 0 };
        let east = entry_point(GalaxyPosition { x: 10, y: 0, z: 5 }, to, 200);
        assert_eq!((east.x_km, east.y_km, east.z_km), (180, 0, 90));
        let south = entry_point(GalaxyPosition { x: 0, y: 0, z: -3 }, to, 200);
        assert_eq!((south.x_km, south.z_km), (0, -180));
    }

    #[test]
    fn crossings_take_a_run_per_two_light_years() {
        let a = GalaxyPosition { x: 0, y: 0, z: 0 };
        assert_eq!(light_years(a, GalaxyPosition { x: 3, y: 0, z: 4 }), 5);
        assert_eq!(5_u64.div_ceil(BAND_LY_PER_RUN), 3);
        let progress = BandTransit {
            from: SystemId(1),
            total_runs: 4,
            runs_left: 1,
        }
        .progress();
        assert!((progress - 0.75).abs() < f32::EPSILON);
    }
}
