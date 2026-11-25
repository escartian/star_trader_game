//! Jobs: contracts the player takes from a port's job board. A port posts
//! a few offers while your ships are there (see `board`), fresh every few runs; the player accepts
//! one while a ship is at that port, then has until its deadline run to do
//! the task. The host settles every active job after each command (see
//! `settle`): a finished task pays credits and standing with the issuing
//! faction exactly once; a missed deadline, a lost trader or abandoning it
//! costs standing instead. Story quests reuse this lifecycle.
//!
//! Tuning and text are in `config/jobs.json` (`JobCatalog`). Credits are in
//! cents; runs are navigation runs.
use crate::economy::{Commodity, Credits, MarketId};
use crate::factions::Faction;
use crate::{BodyId, SystemId};
use serde::{Deserialize, Serialize};

mod board;
mod settle;

pub use board::refresh_offers;
pub use settle::{JobOutcome, abandon, settle};

/// Finished jobs kept for the log, newest last.
const HISTORY: usize = 20;
/// Player-cleared raider bands remembered for hunt jobs.
const CLEARED_BANDS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    Deliver,
    Hunt,
    Escort,
    Survey,
}

impl JobKind {
    pub const ALL: [JobKind; 4] = [Self::Deliver, Self::Hunt, Self::Escort, Self::Survey];

    pub fn label(self) -> &'static str {
        match self {
            Self::Deliver => "Delivery",
            Self::Hunt => "Bounty",
            Self::Escort => "Escort",
            Self::Survey => "Survey",
        }
    }
}

/// What the job asks for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Task {
    /// Bring `quantity` units of `commodity` to `market` (consumed on arrival).
    Deliver {
        commodity: Commodity,
        quantity: u32,
        market: MarketId,
    },
    /// Wipe out raider band `band` (last seen in `system`).
    Hunt { band: u64, system: SystemId },
    /// Have a fleet in `market`'s system when NPC trader `trader` docks there.
    Escort { trader: u64, market: MarketId },
    /// Put a ship in orbit of, or on, `body` in `system`.
    Survey { system: SystemId, body: BodyId },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Job {
    pub id: u64,
    pub kind: JobKind,
    /// The port that posted it; accepting needs a ship there.
    pub issuer: MarketId,
    /// The port's holder when posted; rewards and penalties use its standing.
    pub faction: Option<Faction>,
    pub task: Task,
    pub reward: Credits,
    /// Standing gained with `faction` on success.
    pub standing: i32,
    /// Standing lost with `faction` on failure.
    pub penalty: i32,
    /// The last run on which the task can still be completed.
    pub deadline: u64,
    pub title: String,
    pub brief: String,
}

/// A finished job, for the log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinishedJob {
    pub title: String,
    pub success: bool,
    pub run: u64,
}

/// The player's jobs: board offers, accepted jobs and a short history.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Jobs {
    /// Ports with a standing board and the run each was posted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boards: Vec<(MarketId, u64)>,
    #[serde(default)]
    pub next_id: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub offers: Vec<Job>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<Job>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub history: Vec<FinishedJob>,
    #[serde(default)]
    pub completed: u32,
    #[serde(default)]
    pub failed: u32,
    /// Raider bands the player's fleets wiped out, newest last (band ids are
    /// never reused).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cleared_bands: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobError {
    UnknownJob,
    NotAtPort,
    TooManyJobs,
    AlreadyTaken,
}

impl JobError {
    pub fn message(self) -> &'static str {
        match self {
            Self::UnknownJob => "That job is no longer on offer.",
            Self::NotAtPort => "Take a job at the port that posted it.",
            Self::TooManyJobs => "You already have as many jobs as you can handle.",
            Self::AlreadyTaken => "You already have a job with that task.",
        }
    }
}

impl Jobs {
    /// True for NPC traders' states and games without any job yet.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Accepts offer `id`; `at` is the market where one of the player's
    /// ships is. `max_active` comes from the catalog.
    pub fn accept(
        &mut self,
        id: u64,
        at: Option<MarketId>,
        max_active: usize,
    ) -> Result<&Job, JobError> {
        let index = self
            .offers
            .iter()
            .position(|job| job.id == id)
            .ok_or(JobError::UnknownJob)?;
        if at != Some(self.offers[index].issuer) {
            return Err(JobError::NotAtPort);
        }
        if self.active.len() >= max_active {
            return Err(JobError::TooManyJobs);
        }
        if self
            .active
            .iter()
            .any(|job| job.task == self.offers[index].task)
        {
            return Err(JobError::AlreadyTaken);
        }
        let job = self.offers.remove(index);
        self.active.push(job);
        Ok(self.active.last().expect("just pushed"))
    }

    /// Removes active job `id` (to fail it, see `settle::abandon`).
    pub(crate) fn take_active(&mut self, id: u64) -> Option<Job> {
        let index = self.active.iter().position(|job| job.id == id)?;
        Some(self.active.remove(index))
    }

    pub(crate) fn record(&mut self, title: String, success: bool, run: u64) {
        if success {
            self.completed = self.completed.saturating_add(1);
        } else {
            self.failed = self.failed.saturating_add(1);
        }
        self.history.push(FinishedJob {
            title,
            success,
            run,
        });
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
    }

    /// Save consistency: bounded lists, ids below `next_id`, sane rewards.
    pub fn validate(&self, catalog: &JobCatalog) -> Result<(), String> {
        let jobs = self.offers.iter().chain(&self.active);
        let mut ids = std::collections::HashSet::new();
        for job in jobs {
            if job.id >= self.next_id
                || !ids.insert(job.id)
                || job.reward.0 > 1_000_000_000_000
                || !(0..=50).contains(&job.standing)
                || !(0..=50).contains(&job.penalty)
                || job.title.len() > 400
                || job.brief.len() > 1_000
            {
                return Err("Invalid saved job".into());
            }
        }
        if self.active.len() > catalog.max_active as usize
            || self.offers.len() > 4_096
            || self.boards.len() > 1_024
            || self.history.len() > HISTORY
            || self.cleared_bands.len() > CLEARED_BANDS
        {
            return Err("Too many saved jobs".into());
        }
        Ok(())
    }

    /// Remembers a band the player's fleets wiped out.
    pub fn record_cleared_band(&mut self, band: u64) {
        self.cleared_bands.push(band);
        if self.cleared_bands.len() > CLEARED_BANDS {
            self.cleared_bands.remove(0);
        }
    }
}

/// One job kind's tuning and text. `reward` is the base pay; `reward_step`
/// adds per unit delivered, per raider in the band, or per light-year of
/// distance (escort, survey). A delivery also pays the goods' asking price
/// at the destination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRules {
    pub kind: JobKind,
    /// Relative chance of this kind on a board.
    pub weight: u32,
    pub deadline_runs: u64,
    pub reward: Credits,
    pub reward_step: Credits,
    pub standing: i32,
    pub penalty: i32,
    /// Templates: {port} {system} {cargo} {units} {ships} {trader} {body} {deadline}.
    pub titles: Vec<String>,
    pub briefs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobCatalog {
    pub schema_version: u32,
    pub offers_per_port: u32,
    /// Runs between board refreshes (accepted jobs stay).
    pub refresh_runs: u64,
    pub max_active: u32,
    /// How many nearest systems (the port's own included) a job may send you to.
    pub reach_systems: u32,
    pub kinds: Vec<KindRules>,
}

impl Default for JobCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../config/jobs.json"))
            .expect("the embedded job definitions must pass validation")
    }
}

impl JobCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 {
            return Err("Unsupported job catalog schema version".into());
        }
        if self.offers_per_port > 8
            || !(1..=1_000).contains(&self.refresh_runs)
            || !(1..=20).contains(&self.max_active)
            || !(1..=64).contains(&self.reach_systems)
        {
            return Err("Invalid job board limits".into());
        }
        for kind in JobKind::ALL {
            let Some(rules) = self.kinds.iter().find(|rules| rules.kind == kind) else {
                return Err(format!("No job rules for {}", kind.label()));
            };
            if rules.weight > 100
                || !(1..=1_000).contains(&rules.deadline_runs)
                || rules.reward.0 > 100_000_000
                || rules.reward_step.0 > 10_000_000
                || !(0..=50).contains(&rules.standing)
                || !(0..=50).contains(&rules.penalty)
                || rules.titles.is_empty()
                || rules.briefs.is_empty()
                || rules
                    .titles
                    .iter()
                    .chain(&rules.briefs)
                    .any(|text| text.len() > 400)
            {
                return Err(format!("Invalid {} job rules", kind.label()));
            }
        }
        if self.kinds.len() != JobKind::ALL.len() {
            return Err("Each job kind needs exactly one rule set".into());
        }
        Ok(())
    }

    pub fn rules(&self, kind: JobKind) -> &KindRules {
        self.kinds
            .iter()
            .find(|rules| rules.kind == kind)
            .expect("validated: every kind has rules")
    }
}

#[cfg(test)]
mod tests;
