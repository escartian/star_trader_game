//! Story quests: each great power has a short chain of hand-written steps
//! (`config/quests.json`). When your standing with a faction reaches a step's
//! tier (Friendly, or Honored for the last), the host offers that step: a job
//! whose task is drafted from the faction's port nearest your flagship. Quest
//! offers can be accepted anywhere, count against no job limit, and start
//! their deadline when accepted. Completing a chain's treaty step wins the
//! Diplomat road; a failed step returns after `retry_runs`.
use super::board::{self, Board};
use super::{Job, JobKind};
use crate::StarSystem;
use crate::economy::{Commodity, Credits, Economy, Market};
use crate::factions::{Faction, Relation, Tier};
use crate::travel::TravelState;
use serde::{Deserialize, Serialize};

/// Links a job to its quest step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestRef {
    pub faction: Faction,
    /// Index in the faction's chain.
    pub step: u32,
    /// Runs allowed once accepted.
    pub runs: u64,
    /// The story the offer's event card tells.
    pub intro: String,
    /// Completing it wins the Diplomat road.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub treaty: bool,
}

/// Whose port a delivery step goes to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Destination {
    /// One of the quest faction's own ports.
    #[default]
    Own,
    /// A port of the faction's rival (another faction's when it has none).
    Rival,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub kind: JobKind,
    /// Standing tier the step needs before it is offered.
    pub unlock: Tier,
    #[serde(default)]
    pub destination: Destination,
    /// A delivery's good and amount (random when omitted).
    #[serde(default)]
    pub cargo: Option<Commodity>,
    #[serde(default)]
    pub units: Option<u32>,
    #[serde(default)]
    pub treaty: bool,
    /// Templates: {faction} {rival} {port} {system} {cargo} {units} {ships} {body} {trader} {runs}.
    pub title: String,
    pub intro: String,
    pub brief: String,
    pub reward: Credits,
    pub standing: i32,
    pub penalty: i32,
    pub runs: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chain {
    pub faction: Faction,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestCatalog {
    pub schema_version: u32,
    /// Runs before a failed step is offered again.
    pub retry_runs: u64,
    pub chains: Vec<Chain>,
}

impl Default for QuestCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../../config/quests.json"))
            .expect("the embedded quest definitions must pass validation")
    }
}

impl QuestCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.retry_runs > 1_000 {
            return Err("Invalid quest catalog".into());
        }
        for faction in Faction::ALL {
            let chains = self
                .chains
                .iter()
                .filter(|chain| chain.faction == faction)
                .count();
            if chains > 1 {
                return Err(format!(
                    "More than one quest chain for the {}",
                    faction.label()
                ));
            }
        }
        for chain in &self.chains {
            if chain.steps.is_empty() || chain.steps.len() > 16 {
                return Err("A quest chain needs 1 to 16 steps".into());
            }
            for step in &chain.steps {
                if !(1..=1_000).contains(&step.runs)
                    || step.reward.0 > 100_000_000
                    || !(0..=50).contains(&step.standing)
                    || !(0..=50).contains(&step.penalty)
                    || step
                        .units
                        .is_some_and(|units| !(1..=1_000).contains(&units))
                    || [&step.title, &step.intro, &step.brief]
                        .iter()
                        .any(|text| text.is_empty() || text.len() > 1_000)
                {
                    return Err(format!("Invalid quest step: {}", step.title));
                }
            }
        }
        Ok(())
    }

    pub fn chain(&self, faction: Faction) -> Option<&Chain> {
        self.chains.iter().find(|chain| chain.faction == faction)
    }
}

/// Steps completed and the chain's length for `faction` (0, 0 without a chain).
pub fn chain_progress(state: &TravelState, economy: &Economy, faction: Faction) -> (u32, u32) {
    let total = economy
        .rules
        .quest_catalog
        .chain(faction)
        .map_or(0, |chain| chain.steps.len() as u32);
    (state.jobs.quest_steps[faction.index()].min(total), total)
}

/// Offers each faction's next step where your standing allows it and none
/// is already offered or under way. Returns the new offers' ids.
pub fn offer(state: &mut TravelState, economy: &Economy, systems: &[StarSystem]) -> Vec<u64> {
    let factions = &economy.factions;
    if !factions.is_active() {
        return Vec::new();
    }
    let run = state.navigation.run;
    let mut offered = Vec::new();
    for faction in factions.in_play.clone() {
        let index = faction.index();
        let busy = state
            .jobs
            .quest_offers
            .iter()
            .chain(&state.jobs.active)
            .any(|job| {
                job.quest
                    .as_ref()
                    .is_some_and(|quest| quest.faction == faction)
            });
        if busy || run < state.jobs.quest_retry[index] {
            continue;
        }
        let Some(chain) = economy.rules.quest_catalog.chain(faction) else {
            continue;
        };
        let done = state.jobs.quest_steps[index] as usize;
        let Some(step) = chain.steps.get(done) else {
            continue;
        };
        if factions.tier(faction) < step.unlock {
            continue;
        }
        if let Some(mut job) = draft(state, economy, systems, faction, done as u32, step) {
            job.id = state.jobs.next_id;
            state.jobs.next_id = state.jobs.next_id.saturating_add(1);
            offered.push(job.id);
            state.jobs.quest_offers.push(job);
        }
    }
    offered
}

/// The step as a job, drafted from the faction's port nearest your first ship.
fn draft(
    state: &TravelState,
    economy: &Economy,
    systems: &[StarSystem],
    faction: Faction,
    index: u32,
    step: &Step,
) -> Option<Job> {
    let factions = &economy.factions;
    let held = |market: &&Market| factions.owner(market.id.system) == Some(faction);
    let here = state.ships.first().and_then(|ship| ship.location.system());
    let near = |market: &&Market| {
        let (Some(a), Some(b)) = (
            systems.iter().find(|system| Some(system.id) == here),
            systems.iter().find(|system| system.id == market.id.system),
        ) else {
            return i64::MAX;
        };
        board::distance_squared(a, b)
    };
    let issuer = economy.markets.iter().filter(held).min_by_key(near)?;
    let reach = board::reach(
        issuer,
        economy.rules.job_catalog.reach_systems as usize,
        systems,
    );
    let seed =
        state.navigation.rng_state ^ (u64::from(index) << 40) ^ (faction.index() as u64) << 48;
    let mut board = Board::new(state, economy, systems, issuer, &reach, seed);
    let rival = factions
        .in_play
        .iter()
        .copied()
        .filter(|other| *other != faction)
        .min_by_key(|other| factions.relation(faction, *other) != Relation::Rival);
    let draft = match step.kind {
        JobKind::Deliver => {
            let target = match (step.destination, rival) {
                (Destination::Rival, Some(rival)) => economy
                    .markets
                    .iter()
                    .filter(|market| factions.owner(market.id.system) == Some(rival))
                    .min_by_key(|market| {
                        systems
                            .iter()
                            .find(|system| system.id == issuer.id.system)
                            .zip(systems.iter().find(|system| system.id == market.id.system))
                            .map_or(i64::MAX, |(a, b)| board::distance_squared(a, b))
                    })
                    .unwrap_or(issuer),
                _ => economy
                    .markets
                    .iter()
                    .filter(held)
                    .find(|market| {
                        market.id != issuer.id
                            && reach.iter().any(|system| system.id == market.id.system)
                    })
                    .unwrap_or(issuer),
            };
            let commodity = step
                .cargo
                .unwrap_or(Commodity::ALL[index as usize % Commodity::ALL.len()]);
            board.deliver_to(target, commodity, step.units.unwrap_or(8))
        }
        kind => board.draft(kind)?,
    };
    let mut fields = draft.fields;
    fields.push((
        "faction",
        economy.rules.faction_catalog.get(faction).name.clone(),
    ));
    fields.push((
        "rival",
        rival.map_or("its rivals", Faction::label).to_owned(),
    ));
    fields.push(("runs", step.runs.to_string()));
    Some(Job {
        id: 0,
        kind: step.kind,
        issuer: issuer.id,
        faction: Some(faction),
        task: draft.task,
        reward: Credits(step.reward.0.saturating_add(draft.goods.0)),
        standing: step.standing,
        penalty: step.penalty,
        deadline: state.navigation.run.saturating_add(step.runs),
        title: board::fill(&step.title, &fields),
        brief: board::fill(&step.brief, &fields),
        quest: Some(QuestRef {
            faction,
            step: index,
            runs: step.runs,
            intro: board::fill(&step.intro, &fields),
            treaty: step.treaty,
        }),
    })
}
