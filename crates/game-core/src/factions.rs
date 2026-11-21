//! The four great powers: who holds which systems, how they regard each other
//! and how they regard you.
//!
//! - Definitions (names, colours, temperaments) are JSON content
//!   (`config/factions.json`, carried in `EconomyRules::faction_catalog`).
//! - Territory, relations and your standing are world state
//!   (`Economy::factions`), set at new-game setup (`territory`). A world
//!   without factions in play behaves as before: every system independent.
//! - Standing runs from [`MIN_STANDING`] to [`MAX_STANDING`] and falls into
//!   [`Tier`]s that set your terms at the faction's ports (`terms`).
//! - Every change to standing with one faction spreads half as far (rounded
//!   toward zero) to its allies, and the same amount against you to its rivals.
mod terms;
mod territory;
mod wanted;
pub use terms::{PortTerms, Tier};
pub use territory::{Coverage, FactionSetup, RelationMode};
pub use wanted::{MAX_WANTED_RUNS, MIN_FINE_RUNS};

use crate::SystemId;
use serde::{Deserialize, Serialize};

pub const MIN_STANDING: i32 = -100;
pub const MAX_STANDING: i32 = 100;
const CATALOG_SCHEMA: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Faction {
    Federation,
    Empire,
    Republic,
    Alliance,
}

impl Faction {
    pub const ALL: [Self; 4] = [
        Self::Federation,
        Self::Empire,
        Self::Republic,
        Self::Alliance,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// The short name used across the interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Federation => "Federation",
            Self::Empire => "Empire",
            Self::Republic => "Republic",
            Self::Alliance => "Alliance",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Relation {
    Allied,
    Neutral,
    Rival,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionDefinition {
    pub id: Faction,
    pub name: String,
    pub description: String,
    /// sRGB.
    pub color: [u8; 3],
    /// Standing lost for each act of piracy in its space.
    pub piracy_penalty: u32,
    /// Standing gained for each raider destroyed in its space.
    pub raider_kill_standing: u32,
    /// Sales in its space, in cents, that earn one point of standing.
    pub sale_credits_per_point: u64,
    /// Runs each crime in its space adds to your wanted status (`wanted`).
    #[serde(default = "wanted::default_wanted_runs")]
    pub wanted_runs: u32,
    /// Fine per wanted run left, in cents at a new game's price level.
    #[serde(default = "wanted::default_fine_per_run")]
    pub fine_per_run: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationDefinition {
    pub between: [Faction; 2],
    pub relation: Relation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactionCatalog {
    pub schema_version: u32,
    /// One per faction, in [`Faction::ALL`] order.
    pub factions: Vec<FactionDefinition>,
    /// Relations for [`RelationMode::Classic`]; unlisted pairs are neutral.
    pub classic_relations: Vec<RelationDefinition>,
}

impl Default for FactionCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../config/factions.json"))
            .expect("embedded faction definitions must validate")
    }
}

impl FactionCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn validate(&self) -> Result<(), String> {
        let ordered = self.factions.len() == Faction::ALL.len()
            && self
                .factions
                .iter()
                .zip(Faction::ALL)
                .all(|(definition, id)| definition.id == id);
        let sound = self.factions.iter().all(|definition| {
            (1..=64).contains(&definition.name.len())
                && definition.description.len() <= 256
                && definition.piracy_penalty <= 100
                && definition.raider_kill_standing <= 100
                && (100..=100_000_000).contains(&definition.sale_credits_per_point)
                && (1..=MAX_WANTED_RUNS).contains(&definition.wanted_runs)
                && definition.fine_per_run <= 10_000_000
        });
        let pairs = self
            .classic_relations
            .iter()
            .all(|pair| pair.between[0] != pair.between[1]);
        if self.schema_version != CATALOG_SCHEMA || !ordered || !sound || !pairs {
            return Err("Invalid faction catalog".into());
        }
        Ok(())
    }

    pub fn get(&self, faction: Faction) -> &FactionDefinition {
        &self.factions[faction.index()]
    }
}

/// Territory, relations and your standing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Factions {
    /// Factions in this game, in [`Faction::ALL`] order; empty: none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub in_play: Vec<Faction>,
    /// The faction you declared for at setup, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allegiance: Option<Faction>,
    /// Claimed systems; any other system is independent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub territory: Vec<(SystemId, Faction)>,
    /// Non-neutral pairs between factions in play, lower faction first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<(Faction, Faction, Relation)>,
    /// Your standing with each faction, by [`Faction::index`].
    #[serde(default)]
    pub standing: [i32; 4],
    /// Sales not yet worth a point of standing, in cents, by faction.
    #[serde(default)]
    pub sales_cents: [u64; 4],
    /// Runs left on your wanted status with each faction, by
    /// [`Faction::index`]; 0: not wanted (see `wanted`).
    #[serde(default)]
    pub wanted: [u32; 4],
}

impl Factions {
    pub fn is_active(&self) -> bool {
        !self.in_play.is_empty()
    }

    /// Who holds `system`; `None` is independent space.
    pub fn owner(&self, system: SystemId) -> Option<Faction> {
        self.territory
            .iter()
            .find(|(id, _)| *id == system)
            .map(|(_, faction)| *faction)
    }

    pub fn relation(&self, a: Faction, b: Faction) -> Relation {
        let (low, high) = if a <= b { (a, b) } else { (b, a) };
        self.relations
            .iter()
            .find(|(x, y, _)| (*x, *y) == (low, high))
            .map_or(Relation::Neutral, |(_, _, relation)| *relation)
    }

    pub fn standing(&self, faction: Faction) -> i32 {
        self.standing[faction.index()]
    }

    pub fn tier(&self, faction: Faction) -> Tier {
        Tier::of(self.standing(faction))
    }

    /// Systems each faction holds, as a share of all `systems`, in percent.
    pub fn influence_percent(&self, faction: Faction, systems: usize) -> u32 {
        let held = self.territory.iter().filter(|(_, f)| *f == faction).count();
        (held * 100).checked_div(systems).unwrap_or(0) as u32
    }

    /// Changes standing with `faction` by `delta`, spreading to its allies
    /// and rivals; returns each faction's actual change (zeros left out).
    pub fn adjust(&mut self, faction: Faction, delta: i32) -> Vec<(Faction, i32)> {
        if !self.in_play.contains(&faction) || delta == 0 {
            return Vec::new();
        }
        let mut changes = Vec::new();
        for other in self.in_play.clone() {
            let share = if other == faction {
                delta
            } else {
                match self.relation(faction, other) {
                    Relation::Allied => delta / 2,
                    Relation::Rival => -(delta / 2),
                    Relation::Neutral => 0,
                }
            };
            let before = self.standing(other);
            let after = before
                .saturating_add(share)
                .clamp(MIN_STANDING, MAX_STANDING);
            self.standing[other.index()] = after;
            if after != before {
                changes.push((other, after - before));
            }
        }
        changes
    }

    /// Records a sale worth `cents` at a market in `system`; every
    /// `sale_credits_per_point` earns a point of standing with its holder.
    pub fn record_sale(
        &mut self,
        system: SystemId,
        cents: u64,
        catalog: &FactionCatalog,
    ) -> Vec<(Faction, i32)> {
        let Some(faction) = self.owner(system) else {
            return Vec::new();
        };
        let per_point = catalog.get(faction).sale_credits_per_point;
        let total = self.sales_cents[faction.index()].saturating_add(cents);
        let points = i32::try_from(total / per_point).unwrap_or(MAX_STANDING);
        self.sales_cents[faction.index()] = total % per_point;
        self.adjust(faction, points)
    }

    /// Current-format checks against the world's systems.
    pub fn validate(
        &self,
        systems: &[crate::StarSystem],
        catalog: &FactionCatalog,
    ) -> Result<(), String> {
        let known = |id: SystemId| systems.iter().any(|system| system.id == id);
        let mut seen = std::collections::HashSet::new();
        let sorted = self.in_play.windows(2).all(|pair| pair[0] < pair[1]);
        let territory = self.territory.iter().all(|(system, faction)| {
            known(*system) && seen.insert(*system) && self.in_play.contains(faction)
        });
        let relations = self.relations.iter().all(|(a, b, relation)| {
            a < b
                && *relation != Relation::Neutral
                && self.in_play.contains(a)
                && self.in_play.contains(b)
        });
        let standing = Faction::ALL.iter().all(|faction| {
            let value = self.standing(*faction);
            (MIN_STANDING..=MAX_STANDING).contains(&value)
                && (self.in_play.contains(faction) || value == 0)
                && self.sales_cents[faction.index()] < catalog.get(*faction).sale_credits_per_point
                && self.wanted_runs(*faction) <= MAX_WANTED_RUNS
                && (self.in_play.contains(faction) || self.wanted_runs(*faction) == 0)
        });
        let allegiance = self
            .allegiance
            .is_none_or(|faction| self.in_play.contains(&faction));
        if !(sorted && territory && relations && standing && allegiance) {
            return Err("Invalid faction state".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
