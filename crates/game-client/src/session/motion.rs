//! Run motion: ships glide from where they were before a navigation run to where the
//! run left them, so a run reads as movement instead of a jump. Display only: the
//! authoritative state, saves and camera-follow logic never see these positions.
//! Courses are straight within a leg and a run never starts a second leg, so a
//! straight glide between the two positions stays on the plotted path.
use super::LocalSession;
use game_core::{
    StarSystem,
    movement::SpacePosition,
    npc::Trader,
    travel::{ShipId, TravelState},
};

/// Seconds one run's movement takes to play out at the default pace. Equal to
/// the advance interval (both follow Settings → Run speed), so the runs of an
/// advance join into one continuous glide.
#[cfg(test)]
pub(super) const GLIDE_SECONDS: f64 = super::advance::RUN_INTERVAL_SECONDS;

/// The latest run's glide.
pub(super) struct Motion {
    /// Own state before the run: where each ship's glide starts.
    before: TravelState,
    /// NPC traders before the run, matched to current traders by ID.
    traders_before: Vec<Trader>,
    /// UI clock (seconds) when the glide began; set on the first frame after the run.
    started_at: Option<f64>,
}

impl Motion {
    pub(super) fn new(before: TravelState, traders_before: Vec<Trader>) -> Self {
        Self {
            before,
            traders_before,
            started_at: None,
        }
    }
}

impl LocalSession {
    /// Per-frame clock: steps an advance, then starts or finishes the latest glide.
    /// `animate` is false when the UI has animations off (e.g. captures), which shows
    /// final positions at once. Returns how soon to repaint without new input.
    pub fn frame(&mut self, now: f64, animate: bool) -> Option<f64> {
        self.clock = now;
        self.glide_seconds = if animate { self.run_seconds() } else { 0.0 };
        let next_run = self.drive_advance(now);
        let gliding = self.update_motion(now).then_some(0.0);
        match (next_run, gliding) {
            (Some(run), Some(glide)) => Some(run.min(glide)),
            (run, glide) => run.or(glide),
        }
    }

    /// Galaxy-space drawing position of an own ship; glides after each run.
    pub fn display_position(&self, ship: ShipId, systems: &[StarSystem]) -> Option<SpacePosition> {
        let target = self.state.navigation_position(ship, systems)?;
        let start = self
            .motion
            .as_ref()
            .and_then(|motion| motion.before.navigation_position(ship, systems));
        Some(glide(start, target, self.glide_progress()))
    }

    /// Drawing position of an NPC trader's ship, in light-years; glides like own ships.
    pub fn trader_display_position(
        &self,
        trader: &Trader,
        systems: &[StarSystem],
    ) -> Option<glam::DVec3> {
        self.trader_display_km(trader, systems)
            .map(super::light_years)
    }

    /// The same drawing position in galaxy-space kilometres.
    pub(super) fn trader_display_km(
        &self,
        trader: &Trader,
        systems: &[StarSystem],
    ) -> Option<SpacePosition> {
        let ship = trader.state.ships.first()?.id;
        let target = trader.state.navigation_position(ship, systems)?;
        let start = self
            .motion
            .as_ref()
            .and_then(|motion| {
                motion
                    .traders_before
                    .iter()
                    .find(|before| before.id == trader.id)
            })
            .and_then(|before| before.state.navigation_position(ship, systems));
        Some(glide(start, target, self.glide_progress()))
    }

    /// True while the latest run's glide is still playing out.
    /// (tick before the latest run, current tick, glide progress 0 to 1): what
    /// the map needs to draw bodies moving along their orbits.
    pub fn orbit_clock(&self) -> (u64, u64, f64) {
        let from = self
            .motion
            .as_ref()
            .map_or(self.state.tick, |motion| motion.before.tick);
        (from, self.state.tick, self.glide_progress())
    }

    pub fn gliding(&self) -> bool {
        self.glide_progress() < 1.0
    }

    /// Fraction of the latest glide shown this frame: 0 until its first frame, 1 when done.
    fn glide_progress(&self) -> f64 {
        let Some(motion) = &self.motion else {
            return 1.0;
        };
        let Some(start) = motion.started_at else {
            return 0.0;
        };
        if self.glide_seconds <= 0.0 {
            return 1.0;
        }
        ((self.clock - start) / self.glide_seconds).clamp(0.0, 1.0)
    }

    /// Starts the glide clock on its first frame and drops it once played out.
    /// Returns true while more frames are needed.
    fn update_motion(&mut self, now: f64) -> bool {
        let Some(motion) = &mut self.motion else {
            return false;
        };
        let start = *motion.started_at.get_or_insert(now);
        // A hair of tolerance: UI clocks add up in floating point.
        if self.glide_seconds <= 0.0 || now - start >= self.glide_seconds - 1e-9 {
            self.motion = None;
            return false;
        }
        true
    }
}

/// Linear blend in kilometers; exact at both ends. Linear speed keeps consecutive
/// runs of an advance continuous instead of easing to a stop at each run boundary.
fn glide(start: Option<SpacePosition>, target: SpacePosition, progress: f64) -> SpacePosition {
    let Some(start) = start else {
        return target;
    };
    if progress >= 1.0 {
        return target;
    }
    // Galaxy positions stay within ±1e15 km, so differences fit i64 and f64 is ample.
    let blend = |a: i64, b: i64| a + ((b - a) as f64 * progress).round() as i64;
    SpacePosition {
        x_km: blend(start.x_km, target.x_km),
        y_km: blend(start.y_km, target.y_km),
        z_km: blend(start.z_km, target.z_km),
    }
}
