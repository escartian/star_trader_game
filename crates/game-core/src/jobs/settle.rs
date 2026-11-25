//! Settling active jobs. After every command the host checks each one: a
//! finished task pays its reward and standing once and leaves the list; one
//! that can no longer succeed, or whose deadline run has passed, fails and
//! costs standing. Abandoning a job fails it the same way.
use super::{Job, Task};
use crate::economy::{Credits, Economy};
use crate::travel::{Location, PlayerId, TravelState};

/// What became of a settled job, for the log and the event card.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JobOutcome {
    pub title: String,
    pub success: bool,
    /// Paid on success (credits in cents).
    pub reward: Credits,
    /// Why it ended, in a sentence.
    pub reason: String,
}

enum Verdict {
    Pending,
    Done(String),
    Failed(String),
}

/// Settles every active job of `player`; returns what finished, in order.
pub fn settle(state: &mut TravelState, economy: &mut Economy, player: PlayerId) -> Vec<JobOutcome> {
    let run = state.navigation.run;
    let mut outcomes = Vec::new();
    for job in std::mem::take(&mut state.jobs.active) {
        let verdict = match check(&job, state, economy, player) {
            Verdict::Pending if run > job.deadline => {
                Verdict::Failed(format!("The deadline, run {}, has passed.", job.deadline))
            }
            verdict => verdict,
        };
        match verdict {
            Verdict::Pending => state.jobs.active.push(job),
            Verdict::Done(reason) => {
                if let Task::Deliver { .. } = job.task {
                    hand_over(&job.task, state, economy, player);
                }
                outcomes.push(close(job, true, reason, state, economy));
            }
            Verdict::Failed(reason) => outcomes.push(close(job, false, reason, state, economy)),
        }
    }
    outcomes
}

/// Gives up active job `id`: it fails with its standing penalty.
pub fn abandon(state: &mut TravelState, economy: &mut Economy, id: u64) -> Option<JobOutcome> {
    let job = state.jobs.take_active(id)?;
    Some(close(
        job,
        false,
        "You abandoned it.".into(),
        state,
        economy,
    ))
}

fn close(
    job: Job,
    success: bool,
    reason: String,
    state: &mut TravelState,
    economy: &mut Economy,
) -> JobOutcome {
    let reward = if success { job.reward } else { Credits(0) };
    economy.credits = Credits(economy.credits.0.saturating_add(reward.0));
    if let Some(faction) = job.faction {
        let change = if success { job.standing } else { -job.penalty };
        economy.factions.adjust(faction, change);
    }
    if let Some(quest) = &job.quest {
        let index = quest.faction.index();
        if success {
            state.jobs.quest_steps[index] = state.jobs.quest_steps[index].max(quest.step + 1);
            if quest.treaty && state.jobs.treaty.is_none() {
                state.jobs.treaty = Some(quest.faction);
            }
        } else {
            let retry = economy.rules.quest_catalog.retry_runs;
            state.jobs.quest_retry[index] = state.navigation.run.saturating_add(retry);
        }
    }
    state
        .jobs
        .record(job.title.clone(), success, state.navigation.run);
    JobOutcome {
        title: job.title,
        success,
        reward,
        reason,
    }
}

fn check(job: &Job, state: &TravelState, economy: &Economy, player: PlayerId) -> Verdict {
    let mut ships = state.ships.iter().filter(|ship| ship.owner == player);
    match &job.task {
        Task::Deliver {
            commodity,
            quantity,
            market,
        } => {
            let carried: u64 = ships
                .filter(|ship| {
                    economy
                        .current_market(ship)
                        .is_ok_and(|at| at.id == *market)
                })
                .map(|ship| u64::from(ship.cargo[commodity.index()]))
                .sum();
            if carried >= u64::from(*quantity) {
                Verdict::Done(format!("Delivered {quantity} {}.", commodity.label()))
            } else {
                Verdict::Pending
            }
        }
        Task::Hunt { band, .. } => {
            if state.jobs.cleared_bands.contains(band) {
                Verdict::Done("The band is broken.".into())
            } else if state.navigation.bands.iter().all(|known| known.id != *band) {
                Verdict::Failed("Someone else broke the band first.".into())
            } else {
                Verdict::Pending
            }
        }
        Task::Escort { trader, market } => {
            let Some(npc) = economy.traders.iter().find(|npc| npc.id == *trader) else {
                return Verdict::Failed("The trader was lost on the way.".into());
            };
            let arrived = npc.state.ships.first().is_some_and(|ship| {
                economy
                    .current_market(ship)
                    .is_ok_and(|at| at.id == *market)
            });
            if !arrived {
                Verdict::Pending
            } else if ships.any(|ship| ship.location.system() == Some(market.system)) {
                Verdict::Done(format!("The {} docked safely.", npc.name))
            } else {
                Verdict::Failed(format!(
                    "The {} docked with none of your fleets there.",
                    npc.name
                ))
            }
        }
        Task::Survey { system, body } => {
            let there = ships.any(|ship| {
                matches!(ship.location,
                    Location::Orbiting { system: s, body: b } | Location::Landed { system: s, body: b }
                        if s == *system && b == *body)
            });
            if there {
                Verdict::Done("Survey readings taken.".into())
            } else {
                Verdict::Pending
            }
        }
    }
}

/// Takes a delivery's goods from the player's ships at the market into its stock.
fn hand_over(task: &Task, state: &mut TravelState, economy: &mut Economy, player: PlayerId) {
    let Task::Deliver {
        commodity,
        quantity,
        market,
    } = task
    else {
        return;
    };
    let index = commodity.index();
    let mut left = *quantity;
    for ship in state.ships.iter_mut().filter(|ship| ship.owner == player) {
        if left == 0 {
            break;
        }
        if !economy
            .current_market(ship)
            .is_ok_and(|at| at.id == *market)
        {
            continue;
        }
        let taken = ship.cargo[index].min(left);
        ship.cargo[index] -= taken;
        left -= taken;
    }
    if let Ok(target) = economy.market_mut(*market) {
        let good = &mut target.goods[index];
        good.stock = good.stock.saturating_add(*quantity).min(good.capacity());
    }
}
