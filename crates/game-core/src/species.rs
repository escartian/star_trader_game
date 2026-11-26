//! Species are crew identity, independent of political territory or standing.
use crate::combat::CombatProfile;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Species {
    #[default]
    Human,
    Miri,
    Auralith,
    Kethari,
}

impl Species {
    pub const ALL: [Self; 4] = [Self::Human, Self::Miri, Self::Auralith, Self::Kethari];

    pub fn label(self) -> &'static str {
        match self {
            Self::Human => "Human",
            Self::Miri => "Miri",
            Self::Auralith => "Auralith",
            Self::Kethari => "Kethari",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Human => {
                "Adaptable crews from the settled worlds. Human captains start with balanced fleet handling."
            }
            Self::Miri => {
                "Small green explorers with keen spatial sense. Their fleets evade well, but carry lighter shields."
            }
            Self::Auralith => {
                "Luminous minds that can join in a shared choir. Their shields are strong, but their ships react slowly."
            }
            Self::Kethari => {
                "Insectoid colony-born navigators. Their coordinated formations protect ships, but turn slowly."
            }
        }
    }
}

/// Experience points are earned by this fleet's captain and never duplicated
/// when ships split away. A new split fleet receives a novice captain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetCaptain {
    pub species: Species,
    /// 0–100; +1 per navigation run and +5 for a battle victory.
    pub experience: u32,
}

impl FleetCaptain {
    pub const MAX_EXPERIENCE: u32 = 100;

    pub fn gain(&mut self, points: u32) {
        self.experience = self
            .experience
            .saturating_add(points)
            .min(Self::MAX_EXPERIENCE);
    }

    /// Apply the captain's fleet-wide traits to a battle profile. Engine is
    /// clamped to its 1–10 rating; shields and armor use abstract points.
    pub fn combat_profile(self, profile: &mut CombatProfile) {
        match self.species {
            Species::Human => {}
            Species::Miri => {
                profile.engine = (profile.engine + 1).min(10);
                profile.shields = profile.shields.saturating_sub(2);
            }
            Species::Auralith => {
                profile.shields = profile.shields.saturating_add(4);
                profile.engine = profile.engine.saturating_sub(1).max(1);
            }
            Species::Kethari => {
                profile.armor = profile.armor.saturating_add(2);
                profile.engine = profile.engine.saturating_sub(1).max(1);
            }
        }
        profile.armor = profile.armor.saturating_add(self.experience / 25);
    }

    /// Percentage-point change to fleet escape chance.
    pub fn evasion_points(self) -> i32 {
        let species = match self.species {
            Species::Human => 0,
            Species::Miri => 10,
            Species::Auralith => -10,
            Species::Kethari => -5,
        };
        species + (self.experience / 20) as i32
    }

    pub fn trait_summary(self) -> &'static str {
        match self.species {
            Species::Human => "balanced handling",
            Species::Miri => "+1 engine, −2 shields, +10% escape",
            Species::Auralith => "+4 shields, −1 engine, −10% escape",
            Species::Kethari => "+2 armor, −1 engine, −5% escape",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::HullSize;

    fn profile() -> CombatProfile {
        CombatProfile {
            size: HullSize::Small,
            hull: 10,
            shields: 3,
            armor: 1,
            engine: 4,
            weapon_slots: 0,
            loadout: Vec::new(),
        }
    }

    #[test]
    fn species_tradeoffs_and_experience_affect_combat() {
        let mut miri = profile();
        FleetCaptain {
            species: Species::Miri,
            experience: 50,
        }
        .combat_profile(&mut miri);
        assert_eq!((miri.engine, miri.shields, miri.armor), (5, 1, 3));
        let mut auralith = profile();
        FleetCaptain {
            species: Species::Auralith,
            experience: 0,
        }
        .combat_profile(&mut auralith);
        assert_eq!((auralith.engine, auralith.shields), (3, 7));
        let mut human = profile();
        FleetCaptain::default().combat_profile(&mut human);
        assert_eq!(human.armor, 1);
        let mut kethari = profile();
        FleetCaptain {
            species: Species::Kethari,
            experience: 0,
        }
        .combat_profile(&mut kethari);
        assert_eq!((kethari.armor, kethari.engine), (3, 3));
    }
}
