//! Story quests: each great power has a short chain of hand-written steps
//! (`config/quests.json`). When your standing with a faction reaches a step's
//! tier (Friendly, or Honored for the last), the host offers that step: a job
//! whose task is drafted from the faction's port nearest your flagship. Quest
//! offers can be accepted anywhere, count against no job limit, and start
//! their deadline when accepted. Completing a chain's treaty step wins the
//! Diplomat road; a failed step returns after `retry_runs`.
//!
//! The **Free Captains** (the pirate chain) work the same way, but their
//! steps open with your record as a pirate (successful seizures, and police
//! escapes where factions hold territory) and are drafted from the lawless
//! port nearest your flagship. Completing their chain wins the Pirate road.
use super::board::{self, Board};
use super::{Job, JobKind};
use crate::StarSystem;
use crate::economy::{Commodity, Credits, Economy, Market};
use crate::factions::{Faction, Relation, Tier};
use crate::travel::TravelState;

mod catalog;
pub use catalog::{Chain, Destination, PirateChain, QuestCatalog, QuestRef, Step};

/// Steps completed and the chain's length for `patron` (`None`: the Free
/// Captains); (0, 0) without a chain.
pub fn chain_progress(
    state: &TravelState,
    economy: &Economy,
    patron: Option<Faction>,
) -> (u32, u32) {
    let total = economy.rules.quest_catalog.steps(patron).len() as u32;
    (state.jobs.steps_done(patron).min(total), total)
}

/// Whether the Free Captains deal with you in this game: there are traders
/// to rob and a pirate chain to follow.
pub fn pirates_active(economy: &Economy) -> bool {
    !economy.traders.is_empty() && !economy.rules.quest_catalog.steps(None).is_empty()
}

/// What the Free Captains' next step still waits for, if anything: e.g.
/// "2 successful seizures (you have 1)". `None` when it can be offered.
pub fn pirate_needs(state: &TravelState, economy: &Economy) -> Option<String> {
    let step = economy
        .rules
        .quest_catalog
        .steps(None)
        .get(state.jobs.pirate_steps as usize)?;
    let record = &state.navigation;
    let mut needs = Vec::new();
    if record.pirate_seizures < step.seizures {
        needs.push(format!(
            "{} successful seizures (you have {})",
            step.seizures, record.pirate_seizures
        ));
    }
    if economy.factions.is_active() && record.pirate_escapes < step.escapes {
        needs.push(format!(
            "{} escapes from police after a seizure (you have {})",
            step.escapes, record.pirate_escapes
        ));
    }
    (!needs.is_empty()).then(|| needs.join(" and "))
}

/// Whether `step` of `patron`'s chain is open to you now.
fn unlocked(state: &TravelState, economy: &Economy, patron: Option<Faction>, step: &Step) -> bool {
    match patron {
        Some(faction) => economy.factions.tier(faction) >= step.unlock.unwrap_or(Tier::Friendly),
        None => pirate_needs(state, economy).is_none(),
    }
}

/// Offers each patron's next step where your standing (or pirate record)
/// allows it and none is already offered or under way. Returns the new
/// offers' ids.
pub fn offer(state: &mut TravelState, economy: &Economy, systems: &[StarSystem]) -> Vec<u64> {
    let mut patrons: Vec<Option<Faction>> =
        economy.factions.in_play.iter().copied().map(Some).collect();
    if pirates_active(economy) {
        patrons.push(None);
    }
    let run = state.navigation.run;
    let mut offered = Vec::new();
    for patron in patrons {
        let busy = state
            .jobs
            .quest_offers
            .iter()
            .chain(&state.jobs.active)
            .any(|job| {
                job.quest
                    .as_ref()
                    .is_some_and(|quest| quest.faction == patron)
            });
        let retry = match patron {
            Some(faction) => state.jobs.quest_retry[faction.index()],
            None => state.jobs.pirate_retry,
        };
        if busy || run < retry {
            continue;
        }
        let done = state.jobs.steps_done(patron) as usize;
        let Some(step) = economy.rules.quest_catalog.steps(patron).get(done) else {
            continue;
        };
        if !unlocked(state, economy, patron, step) {
            continue;
        }
        if let Some(mut job) = draft(state, economy, systems, patron, done as u32, step) {
            job.id = state.jobs.next_id;
            state.jobs.next_id = state.jobs.next_id.saturating_add(1);
            offered.push(job.id);
            state.jobs.quest_offers.push(job);
        }
    }
    offered
}

/// The step as a job, drafted from the patron's port nearest your first
/// ship: one of the faction's own, or for the Free Captains a lawless one
/// (any port when every system is held).
fn draft(
    state: &TravelState,
    economy: &Economy,
    systems: &[StarSystem],
    patron: Option<Faction>,
    index: u32,
    step: &Step,
) -> Option<Job> {
    let factions = &economy.factions;
    let held = |market: &&Market| factions.owner(market.id.system) == patron;
    let here = state.ships.first().and_then(|ship| ship.location.system());
    let from = |origin: Option<crate::SystemId>| {
        move |market: &&Market| {
            let find = |id| systems.iter().find(|system| Some(system.id) == id);
            find(origin)
                .zip(find(Some(market.id.system)))
                .map_or(i64::MAX, |(a, b)| board::distance_squared(a, b))
        }
    };
    let issuer = economy
        .markets
        .iter()
        .filter(held)
        .min_by_key(from(here))
        .or_else(|| {
            patron
                .is_none()
                .then(|| economy.markets.iter().min_by_key(from(here)))
                .flatten()
        })?;
    let reach = board::reach(
        issuer,
        economy.rules.job_catalog.reach_systems as usize,
        systems,
    );
    let patron_bits = patron.map_or(7, |faction| faction.index() as u64);
    let seed = state.navigation.rng_state ^ (u64::from(index) << 40) ^ (patron_bits << 48);
    let mut board = Board::new(state, economy, systems, issuer, &reach, seed);
    let rival = patron.and_then(|faction| {
        factions
            .in_play
            .iter()
            .copied()
            .filter(|other| *other != faction)
            .min_by_key(|other| factions.relation(faction, *other) != Relation::Rival)
    });
    let nearest_of = |owner: Option<Faction>| {
        economy
            .markets
            .iter()
            .filter(|market| market.id != issuer.id)
            .filter(|market| match owner {
                Some(faction) => factions.owner(market.id.system) == Some(faction),
                None => factions
                    .owner(market.id.system)
                    .is_some_and(|f| Some(f) != patron),
            })
            .min_by_key(from(Some(issuer.id.system)))
    };
    let in_reach_other = || {
        economy.markets.iter().filter(held).find(|market| {
            market.id != issuer.id && reach.iter().any(|system| system.id == market.id.system)
        })
    };
    let commodity = step
        .cargo
        .unwrap_or(Commodity::ALL[index as usize % Commodity::ALL.len()]);
    let units = step.units.unwrap_or(8);
    let draft = match step.kind {
        JobKind::Deliver => {
            let target = match (step.destination, rival) {
                (Destination::Rival, Some(rival)) => nearest_of(Some(rival)),
                _ => in_reach_other(),
            }
            .unwrap_or(issuer);
            board.deliver_to(target, commodity, units)
        }
        JobKind::Smuggle => {
            let target = nearest_of(rival)
                .or_else(|| nearest_of(None))
                .or_else(in_reach_other)
                .unwrap_or(issuer);
            board.smuggle_to(target, commodity, units)
        }
        JobKind::Raid => board.raid_of(units)?,
        kind => board.draft(kind)?,
    };
    let mut fields = draft.fields;
    if let Some(faction) = patron {
        fields.push((
            "faction",
            economy.rules.faction_catalog.get(faction).name.clone(),
        ));
    }
    fields.push((
        "rival",
        rival.map_or("its rivals", Faction::label).to_owned(),
    ));
    fields.push(("runs", step.runs.to_string()));
    Some(Job {
        id: 0,
        kind: step.kind,
        issuer: issuer.id,
        faction: patron,
        task: draft.task,
        reward: Credits(step.reward.0.saturating_add(draft.goods.0)),
        standing: step.standing,
        penalty: step.penalty,
        deadline: state.navigation.run.saturating_add(step.runs),
        title: board::fill(&step.title, &fields),
        brief: board::fill(&step.brief, &fields),
        quest: Some(QuestRef {
            faction: patron,
            step: index,
            runs: step.runs,
            intro: board::fill(&step.intro, &fields),
            treaty: step.treaty,
        }),
    })
}
