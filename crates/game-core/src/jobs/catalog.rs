//! Job board tuning and text (`config/jobs.json`).
use super::JobKind;
use crate::economy::Credits;
use serde::{Deserialize, Serialize};

/// One job kind's tuning and text. `reward` is the base pay; `reward_step`
/// adds per unit delivered, per raider in the band, or per light-year of
/// distance (escort, survey), or per unit to seize (raid). A delivery or a
/// smuggling run also pays the goods' asking price at the destination.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KindRules {
    pub kind: JobKind,
    /// Relative chance of this kind on a board.
    pub weight: u32,
    /// Fence work: posted only at ports in systems no faction holds.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lawless: bool,
    pub deadline_runs: u64,
    pub reward: Credits,
    pub reward_step: Credits,
    pub standing: i32,
    pub penalty: i32,
    /// Templates: {port} {system} {faction} {cargo} {units} {ships} {trader} {body} {deadline}.
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
        Self::from_json(include_str!("../../../../config/jobs.json"))
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
