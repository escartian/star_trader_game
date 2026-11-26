//! Story quest definitions (`config/quests.json`): one chain per great power
//! and the Free Captains' pirate chain.
use crate::economy::{Commodity, Credits};
use crate::factions::{Faction, Tier};
use crate::jobs::JobKind;
use serde::{Deserialize, Serialize};

/// Links a job to its quest step.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestRef {
    /// The great power whose chain it is; `None` for the Free Captains.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<Faction>,
    /// Index in the chain.
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
    /// Standing tier a faction step needs before it is offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlock: Option<Tier>,
    /// A pirate step's needs: successful seizures so far, and police
    /// escapes (ignored in games without factions).
    #[serde(default)]
    pub seizures: u64,
    #[serde(default)]
    pub escapes: u64,
    #[serde(default)]
    pub destination: Destination,
    /// A delivery's or smuggling run's good (random when omitted) and the
    /// units to deliver, smuggle or seize (8 when omitted).
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

/// The pirate chain: offered at lawless ports as your record as a pirate
/// grows; completing it wins the Pirate road.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PirateChain {
    pub name: String,
    pub steps: Vec<Step>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestCatalog {
    pub schema_version: u32,
    /// Runs before a failed step is offered again.
    pub retry_runs: u64,
    pub chains: Vec<Chain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pirate: Option<PirateChain>,
}

impl Default for QuestCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../../../config/quests.json"))
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
            if chain.steps.iter().any(|step| step.unlock.is_none()) {
                return Err("Every faction quest step needs a standing tier".into());
            }
            validate_steps(&chain.steps)?;
        }
        if let Some(pirate) = &self.pirate {
            if pirate.name.is_empty() || pirate.name.len() > 60 {
                return Err("Invalid pirate chain name".into());
            }
            if pirate.steps.iter().any(|step| step.treaty) {
                return Err("The pirate chain carries no treaty".into());
            }
            validate_steps(&pirate.steps)?;
        }
        Ok(())
    }

    pub fn chain(&self, faction: Faction) -> Option<&Chain> {
        self.chains.iter().find(|chain| chain.faction == faction)
    }

    /// The steps of `patron`'s chain (`None`: the Free Captains).
    pub fn steps(&self, patron: Option<Faction>) -> &[Step] {
        match patron {
            Some(faction) => self.chain(faction).map_or(&[], |chain| &chain.steps),
            None => self.pirate.as_ref().map_or(&[], |chain| &chain.steps),
        }
    }

    /// Who offers `patron`'s chain, for cards and logs.
    pub fn pirate_name(&self) -> &str {
        self.pirate
            .as_ref()
            .map_or("Free Captains", |chain| &chain.name)
    }
}

fn validate_steps(steps: &[Step]) -> Result<(), String> {
    if steps.is_empty() || steps.len() > 16 {
        return Err("A quest chain needs 1 to 16 steps".into());
    }
    for step in steps {
        if !(1..=1_000).contains(&step.runs)
            || step.reward.0 > 100_000_000
            || !(0..=50).contains(&step.standing)
            || !(0..=50).contains(&step.penalty)
            || step.seizures > 1_000
            || step.escapes > 1_000
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
    Ok(())
}
