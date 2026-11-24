//! NPC trader personalities and their tuning (`config/npc.json`).
//!
//! A personality decides how a trader picks hauls (danger, margin, load size,
//! whose space), how hard it is to rob, and how often it slips past raiders.
//! The catalog travels in the economy rules, like the faction catalog, so a
//! save keeps the tuning it was played with.
use serde::{Deserialize, Serialize};

const CATALOG_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Personality {
    Cautious,
    Opportunist,
    BulkHauler,
    Loyalist,
    Smuggler,
    Corsair,
}

impl Personality {
    pub const ALL: [Personality; 6] = [
        Self::Cautious,
        Self::Opportunist,
        Self::BulkHauler,
        Self::Loyalist,
        Self::Smuggler,
        Self::Corsair,
    ];

    pub fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalityDefinition {
    pub id: Personality,
    pub label: String,
    pub description: String,
    /// Skips ports on dangerous, very dangerous or insidious planets.
    pub avoids_danger: bool,
    /// How it values a haul to such a port, in percent of the real profit.
    pub danger_profit_percent: u64,
    /// When it flies for a faction: trades only in its own or allied space.
    pub friendly_only: bool,
    /// Least profit worth a haul, in percent of its cost (goods and fuel).
    pub margin_percent: u64,
    /// Units bought per haul (at most the hold).
    pub haul_units: u32,
    /// Your fleet's strength, in percent of its own, that makes it yield
    /// cargo to a demand (see `piracy`).
    pub yield_percent: u64,
    /// Percent chance to slip past raiders that strike it under way.
    pub raid_escape_percent: u64,
    /// Percent chance per run to approach your flagship when within hailing
    /// range (see `approach`).
    #[serde(default)]
    pub approach_percent: u64,
}

/// How NPC traders deal with you when they approach (see `approach`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproachRules {
    /// A trade offer's price, in percent of the galaxy list price.
    pub offer_price_percent: u64,
    /// A corsair shakes you down only with this much of your fleet's
    /// strength, in percent; otherwise it trades like anyone.
    pub shakedown_strength_percent: u64,
    /// Runs before the same trader approaches you again.
    pub cooldown_runs: u64,
}

impl Default for ApproachRules {
    fn default() -> Self {
        Self {
            offer_price_percent: 85,
            shakedown_strength_percent: 150,
            cooldown_runs: 10,
        }
    }
}

impl ApproachRules {
    fn valid(&self) -> bool {
        (10..=200).contains(&self.offer_price_percent)
            && (100..=1_000).contains(&self.shakedown_strength_percent)
            && self.cooldown_runs <= 1_000
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersonalityCatalog {
    pub schema_version: u32,
    /// One per personality, in [`Personality::ALL`] order.
    pub personalities: Vec<PersonalityDefinition>,
    /// Personalities dealt to new traders in turn (the first trader gets the
    /// first). Independent traders never get a loyalist (see
    /// [`PersonalityCatalog::for_independent`]).
    pub spawn_cycle: Vec<Personality>,
    /// How approaching traders deal with you.
    #[serde(default)]
    pub approaches: ApproachRules,
}

impl Default for PersonalityCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../../config/npc.json"))
            .expect("embedded NPC personalities must validate")
    }
}

impl PersonalityCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        let ordered = self.personalities.len() == Personality::ALL.len()
            && self
                .personalities
                .iter()
                .zip(Personality::ALL)
                .all(|(definition, id)| definition.id == id);
        let sound = self.personalities.iter().all(|definition| {
            (1..=64).contains(&definition.label.len())
                && definition.description.len() <= 256
                && (50..=400).contains(&definition.danger_profit_percent)
                && definition.margin_percent <= 100
                && (1..=1_000).contains(&definition.haul_units)
                && (100..=1_000).contains(&definition.yield_percent)
                && definition.raid_escape_percent <= 100
                && definition.approach_percent <= 100
        });
        if self.schema_version != CATALOG_SCHEMA
            || !ordered
            || !sound
            || !(1..=64).contains(&self.spawn_cycle.len())
            || !self.approaches.valid()
        {
            return Err("Invalid NPC personality catalog".into());
        }
        Ok(())
    }

    pub fn get(&self, personality: Personality) -> &PersonalityDefinition {
        &self.personalities[personality.index()]
    }

    /// The personality the `index`-th new trader gets.
    pub fn dealt(&self, index: usize) -> Personality {
        self.spawn_cycle[index % self.spawn_cycle.len()]
    }

    /// A loyalist with no faction to be loyal to trades as a smuggler.
    pub fn for_independent(personality: Personality) -> Personality {
        match personality {
            Personality::Loyalist => Personality::Smuggler,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_catalog_keeps_the_original_pair_first() {
        let catalog = PersonalityCatalog::default();
        assert_eq!(catalog.dealt(0), Personality::Cautious);
        assert_eq!(catalog.dealt(1), Personality::Opportunist);
        let yields = |p| catalog.get(p).yield_percent;
        assert!(yields(Personality::Opportunist) > yields(Personality::Cautious));
        assert!(yields(Personality::Corsair) >= yields(Personality::Opportunist));
        let mut broken = catalog.clone();
        broken.personalities.swap(0, 1);
        assert!(broken.validate().is_err());
    }
}
