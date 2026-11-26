//! Fleet software: upgrades installed once per fleet (not per ship) at a
//! market, paid by the fleet's lead ship.
//!
//! - **Pursuit suite**: while the fleet pursues a trader or a raider band, its
//!   courses take [`PURSUIT_COURSE_PERCENT`]% of the usual time. Ships
//!   otherwise all fly at one speed, so a chase that starts a run behind
//!   tends to stay a run behind.
//!
//! Software is copied to both fleets when one splits, and a merged fleet
//! keeps whatever either side had.
use crate::economy::Credits;
use serde::{Deserialize, Serialize};

/// Course time (percent of normal) for a pursuing fleet with the pursuit suite.
pub const PURSUIT_COURSE_PERCENT: u64 = 65;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FleetSoftware {
    PursuitSuite,
}

impl FleetSoftware {
    pub const ALL: [FleetSoftware; 1] = [FleetSoftware::PursuitSuite];

    pub fn label(self) -> &'static str {
        match self {
            Self::PursuitSuite => "Pursuit suite",
        }
    }

    pub fn summary(self) -> &'static str {
        match self {
            Self::PursuitSuite => "while pursuing, the fleet flies its courses in 65% of the time",
        }
    }
}

/// Prices at a new game's price level (credits in cents).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwarePrices {
    pub pursuit_suite: Credits,
}

impl Default for SoftwarePrices {
    fn default() -> Self {
        Self {
            pursuit_suite: Credits(90_000),
        }
    }
}

impl SoftwarePrices {
    pub fn of(&self, software: FleetSoftware) -> Credits {
        match software {
            FleetSoftware::PursuitSuite => self.pursuit_suite,
        }
    }

    pub fn valid(&self) -> bool {
        FleetSoftware::ALL
            .into_iter()
            .all(|software| (1..=1_000_000_000).contains(&self.of(software).0))
    }
}

/// Whether `software` lists each item at most once.
pub fn valid_install(software: &[FleetSoftware]) -> bool {
    software
        .iter()
        .enumerate()
        .all(|(index, item)| !software[..index].contains(item))
}
