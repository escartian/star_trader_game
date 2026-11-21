//! Wanted status: the law's memory of your crimes in a faction's space.
//!
//! - Each crime there (an act of piracy, or fighting its police) adds the
//!   faction's `wanted_runs` to your wanted status with it, up to
//!   [`MAX_WANTED_RUNS`]. Every navigation run takes one off.
//! - While wanted, the faction's police patrols pursue your fleets
//!   (`crate::police`) and its ports refuse your trade (`terms`).
//! - Paying the fine clears it: `fine_per_run` for every run left, but never
//!   less than [`MIN_FINE_RUNS`] runs' worth. Hostile standing alone also
//!   sets the police on you; complying then costs that minimum fine.
use super::{Faction, FactionCatalog, Factions, Tier};

/// The longest wanted status, in runs.
pub const MAX_WANTED_RUNS: u32 = 200;
/// The smallest fine, in runs of `fine_per_run`.
pub const MIN_FINE_RUNS: u32 = 10;

pub(super) fn default_wanted_runs() -> u32 {
    30
}

pub(super) fn default_fine_per_run() -> u64 {
    1_000
}

impl Factions {
    /// Runs left on your wanted status with `faction` (0: not wanted).
    pub fn wanted_runs(&self, faction: Faction) -> u32 {
        self.wanted[faction.index()]
    }

    pub fn is_wanted(&self, faction: Faction) -> bool {
        self.wanted_runs(faction) > 0
    }

    /// Whether `faction`'s police pursue you: you are wanted, or your
    /// standing with it is Hostile.
    pub fn pursues_you(&self, faction: Faction) -> bool {
        self.in_play.contains(&faction)
            && (self.is_wanted(faction) || self.tier(faction) == Tier::Hostile)
    }

    /// Records a crime in `faction`'s space: `count` times its `wanted_runs`.
    pub fn charge(&mut self, faction: Faction, count: u32, catalog: &FactionCatalog) {
        if !self.in_play.contains(&faction) || count == 0 {
            return;
        }
        let added = catalog.get(faction).wanted_runs.saturating_mul(count);
        let slot = &mut self.wanted[faction.index()];
        *slot = slot.saturating_add(added).min(MAX_WANTED_RUNS);
    }

    /// The fine that clears your record with `faction`, in cents at a new
    /// game's price level (scale it by the price level before charging).
    pub fn fine_cents(&self, faction: Faction, catalog: &FactionCatalog) -> u64 {
        let runs = self.wanted_runs(faction).max(MIN_FINE_RUNS);
        catalog
            .get(faction)
            .fine_per_run
            .saturating_mul(u64::from(runs))
    }

    /// Your fine is paid: `faction` no longer wants you.
    pub fn clear_wanted(&mut self, faction: Faction) {
        self.wanted[faction.index()] = 0;
    }

    /// One navigation run passes: every wanted status runs down by one.
    pub fn pass_run(&mut self) {
        for runs in &mut self.wanted {
            *runs = runs.saturating_sub(1);
        }
    }
}
