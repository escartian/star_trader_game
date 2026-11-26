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
mod catalog;
pub mod quests;
mod settle;

pub use board::refresh_offers;
pub use catalog::{JobCatalog, KindRules};
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
    /// Fence work: seize cargo from NPC traders (lawless ports only).
    Raid,
    /// Fence work: run goods into a faction port unseen (lawless ports only).
    Smuggle,
}

impl JobKind {
    pub const ALL: [JobKind; 6] = [
        Self::Deliver,
        Self::Hunt,
        Self::Escort,
        Self::Survey,
        Self::Raid,
        Self::Smuggle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Deliver => "Delivery",
            Self::Hunt => "Bounty",
            Self::Escort => "Escort",
            Self::Survey => "Survey",
            Self::Raid => "Raid",
            Self::Smuggle => "Smuggling",
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
    /// Seize `units` units from NPC traders, by demand or plunder; `taken`
    /// counts those seized since the job was accepted.
    Raid {
        units: u32,
        #[serde(default)]
        taken: u32,
    },
    /// Bring `quantity` `commodity` to `market`, a faction port, without
    /// being stopped by that faction's police on the way (consumed on arrival).
    Smuggle {
        commodity: Commodity,
        quantity: u32,
        market: MarketId,
    },
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
    /// The story quest step this job is (none for board jobs).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quest: Option<quests::QuestRef>,
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
    /// Quest steps offered and not yet accepted (at most one per faction).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quest_offers: Vec<Job>,
    /// Quest steps completed, by [`Faction::index`].
    #[serde(default)]
    pub quest_steps: [u32; 6],
    /// The run from which a faction's failed step is offered again.
    #[serde(default)]
    pub quest_retry: [u64; 6],
    /// The faction whose treaty you carried: the Diplomat road is won.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treaty: Option<Faction>,
    /// Free Captains story steps completed (the pirate chain; see `quests`).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pirate_steps: u32,
    /// The run from which a failed Free Captains step is offered again.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub pirate_retry: u64,
}

fn is_zero<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
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

    /// Accepts board offer `id` (`at`: the market where the accepting ship
    /// is; `max_active` from the catalog) or quest offer `id` (anywhere, no
    /// limit; its deadline starts on `run`).
    pub fn accept(
        &mut self,
        id: u64,
        at: Option<MarketId>,
        max_active: usize,
        run: u64,
    ) -> Result<&Job, JobError> {
        if let Some(index) = self.quest_offers.iter().position(|job| job.id == id) {
            let mut job = self.quest_offers.remove(index);
            let runs = job.quest.as_ref().map_or(0, |quest| quest.runs);
            job.deadline = run.saturating_add(runs);
            self.active.push(job);
            return Ok(self.active.last().expect("just pushed"));
        }
        let index = self
            .offers
            .iter()
            .position(|job| job.id == id)
            .ok_or(JobError::UnknownJob)?;
        if at != Some(self.offers[index].issuer) {
            return Err(JobError::NotAtPort);
        }
        let board_jobs = self.active.iter().filter(|job| job.quest.is_none()).count();
        if board_jobs >= max_active {
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
        let jobs = self
            .offers
            .iter()
            .chain(&self.active)
            .chain(&self.quest_offers);
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
        if self.active.iter().filter(|job| job.quest.is_none()).count()
            > catalog.max_active as usize
            || self.quest_offers.len() > Faction::ALL.len() + 1
            || self.offers.len() > 4_096
            || self.boards.len() > 1_024
            || self.history.len() > HISTORY
            || self.cleared_bands.len() > CLEARED_BANDS
            || self.pirate_steps > 16
            || self.quest_steps.iter().any(|steps| *steps > 16)
        {
            return Err("Too many saved jobs".into());
        }
        Ok(())
    }

    /// Story steps completed for `patron` (`None`: the Free Captains).
    pub fn steps_done(&self, patron: Option<Faction>) -> u32 {
        patron.map_or(self.pirate_steps, |faction| {
            self.quest_steps[faction.index()]
        })
    }

    /// Counts units seized from a trader toward every active raid job.
    pub fn record_seizure(&mut self, units: u32) {
        for job in &mut self.active {
            if let Task::Raid { taken, .. } = &mut job.task {
                *taken = taken.saturating_add(units);
            }
        }
    }

    /// Remembers a band the player's fleets wiped out.
    pub fn record_cleared_band(&mut self, band: u64) {
        self.cleared_bands.push(band);
        if self.cleared_bands.len() > CLEARED_BANDS {
            self.cleared_bands.remove(0);
        }
    }
}

#[cfg(test)]
mod pirate_tests;
#[cfg(test)]
mod quest_tests;
#[cfg(test)]
mod tests;
