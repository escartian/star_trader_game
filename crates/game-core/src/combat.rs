//! Data-driven combat definitions: weapons and per-hull combat profiles.
//! Units are abstract game points: hull/shield/armor "points", engine is a 1–10
//! rating used for evasion and initiative. Engagements resolve in `engagement`.
pub mod engagement;
pub mod report;
mod rng;
pub use engagement::{CombatantSetup, Side};
pub use report::{CombatOutcome, CombatReport};
use serde::{Deserialize, Serialize};

/// Physical size class; later rules may restrict loadouts or targeting by size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HullSize {
    Small,
    Medium,
    Large,
}

impl HullSize {
    pub fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
        }
    }
}

/// One weapon definition. Each of `shots` rolls against `accuracy_percent`
/// (reduced by target evasion); a hit deals `damage` to shields scaled by
/// `shield_efficiency_percent`, and any remainder reaches the hull after armor,
/// which `armor_penetration` partly ignores. See `engagement` for exact rules.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeaponSpec {
    /// Stable snake_case identity referenced by hull loadouts and saves.
    pub id: String,
    pub name: String,
    pub damage: u32,
    pub shots: u32,
    pub accuracy_percent: u32,
    pub shield_efficiency_percent: u32,
    pub armor_penetration: u32,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatCatalog {
    pub schema_version: u32,
    pub weapons: Vec<WeaponSpec>,
}

impl Default for CombatCatalog {
    fn default() -> Self {
        Self::from_json(include_str!("../../../config/combat.json"))
            .expect("embedded combat definitions must validate")
    }
}

impl CombatCatalog {
    pub fn from_json(json: &str) -> Result<Self, String> {
        let catalog: Self = serde_json::from_str(json).map_err(|error| error.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn weapon(&self, id: &str) -> Option<&WeaponSpec> {
        self.weapons.iter().find(|weapon| weapon.id == id)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 || self.weapons.is_empty() || self.weapons.len() > 64 {
            return Err("Unsupported combat catalog".into());
        }
        for (index, weapon) in self.weapons.iter().enumerate() {
            let readable_id = !weapon.id.is_empty()
                && weapon.id.len() <= 48
                && weapon
                    .id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
            if !readable_id
                || self.weapons[..index]
                    .iter()
                    .any(|other| other.id == weapon.id)
                || weapon.name.trim().is_empty()
                || !(1..=10_000).contains(&weapon.damage)
                || !(1..=20).contains(&weapon.shots)
                || !(1..=100).contains(&weapon.accuracy_percent)
                || weapon.shield_efficiency_percent > 1000
                || weapon.armor_penetration > 10_000
            {
                return Err(format!("Invalid or duplicate weapon {}", weapon.id));
            }
        }
        Ok(())
    }
}

/// Combat statistics for one ship class.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatProfile {
    pub size: HullSize,
    /// Structure points; a ship whose damage reaches this is destroyed.
    pub hull: u32,
    /// Shield points, fully recharged before each engagement.
    pub shields: u32,
    /// Flat reduction applied to each hit that reaches the hull.
    pub armor: u32,
    /// Engine rating 1–10: evasion and initiative.
    pub engine: u32,
    pub weapon_slots: u32,
    /// Weapon IDs fitted to newly built hulls; at most `weapon_slots`.
    pub loadout: Vec<String>,
}

impl CombatProfile {
    pub fn validate(&self, weapons: &CombatCatalog) -> Result<(), String> {
        if !(1..=100_000).contains(&self.hull)
            || self.shields > 100_000
            || self.armor > 10_000
            || !(1..=10).contains(&self.engine)
            || self.weapon_slots > 8
            || self.loadout.len() > self.weapon_slots as usize
        {
            return Err("Invalid hull combat profile".into());
        }
        if let Some(missing) = self.loadout.iter().find(|id| weapons.weapon(id).is_none()) {
            return Err(format!("Loadout references unknown weapon {missing}"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod engagement_tests;
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_carries_the_five_original_weapons() {
        let catalog = CombatCatalog::default();
        for id in [
            "photon_singularity_beam",
            "quantum_entanglement_torpedo",
            "neutron_beam",
            "graviton_pulse",
            "magnetic_resonance_disruptor",
        ] {
            assert!(catalog.weapon(id).is_some(), "{id}");
        }
    }

    #[test]
    fn invalid_weapons_and_loadouts_are_rejected() {
        let mut catalog = CombatCatalog::default();
        catalog.weapons.push(catalog.weapons[0].clone());
        assert!(catalog.validate().is_err(), "duplicate id");
        let mut catalog = CombatCatalog::default();
        catalog.weapons[0].accuracy_percent = 0;
        assert!(catalog.validate().is_err(), "zero accuracy");
        let catalog = CombatCatalog::default();
        let mut profile = CombatProfile {
            size: HullSize::Small,
            hull: 10,
            shields: 0,
            armor: 0,
            engine: 5,
            weapon_slots: 1,
            loadout: vec!["neutron_beam".into()],
        };
        assert!(profile.validate(&catalog).is_ok());
        profile.loadout.push("graviton_pulse".into());
        assert!(
            profile.validate(&catalog).is_err(),
            "more weapons than slots"
        );
        profile.loadout = vec!["plasma_spoon".into()];
        assert!(profile.validate(&catalog).is_err(), "unknown weapon");
    }
}
