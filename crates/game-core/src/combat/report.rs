//! Saved, readable outcome of the most recent engagement.
use super::engagement::{CombatantResult, MAX_COMBATANTS, MAX_ROUNDS, ShotRecord, Side};
use crate::fleet::FleetId;
use crate::travel::ShipId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatOutcome {
    /// Every hostile ship was destroyed.
    Victory,
    /// Every player ship in the fleet was destroyed (one may survive crippled).
    Defeat,
    /// Both sides still had ships after the round limit.
    Disengaged,
}

impl CombatOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Victory => "Victory",
            Self::Defeat => "Defeat",
            Self::Disengaged => "Disengaged",
        }
    }

    pub fn from_winner(winner: Option<Side>) -> Self {
        match winner {
            Some(Side::Player) => Self::Victory,
            Some(Side::Hostile) => Self::Defeat,
            None => Self::Disengaged,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatReport {
    pub encounter: u64,
    pub fleet: FleetId,
    pub run: u64,
    /// Seed drawn from the navigation RNG; replaying it reproduces the fight.
    pub seed: u64,
    pub outcome: CombatOutcome,
    pub combatants: Vec<CombatantResult>,
    pub rounds: Vec<Vec<ShotRecord>>,
    /// Ore recovered into the fleet's free cargo space from destroyed hostiles.
    pub salvage_ore: u32,
    /// Cargo units lost with destroyed (or crippled) player ships.
    pub cargo_lost: u32,
    /// The last owned ship is never lost; it limps away at 1 hull point.
    pub crippled: Option<ShipId>,
}

/// Generous bound for one saved shot list (8 weapons × 20 shots × 16 ships).
const MAX_SHOTS_PER_ROUND: usize = 8 * 20 * MAX_COMBATANTS;

impl CombatReport {
    pub fn validate(&self) -> Result<(), String> {
        let count = self.combatants.len();
        let invalid = count == 0
            || count > MAX_COMBATANTS
            || self.rounds.is_empty()
            || self.rounds.len() > MAX_ROUNDS as usize
            || self.combatants.iter().any(|ship| {
                ship.final_damage > ship.hull
                    || ship.starting_damage >= ship.hull
                    || ship.final_damage < ship.starting_damage
                    || ship.destroyed != (ship.final_damage == ship.hull)
                    || ship.hits > ship.shots
            })
            || self.rounds.iter().any(|shots| {
                shots.len() > MAX_SHOTS_PER_ROUND
                    || shots.iter().any(|shot| {
                        usize::from(shot.attacker) >= count
                            || usize::from(shot.target) >= count
                            || (!shot.hit && (shot.hull_damage > 0 || shot.shield_damage > 0))
                    })
            });
        if invalid {
            return Err("Invalid saved combat report".into());
        }
        Ok(())
    }

    pub fn side(&self, side: Side) -> impl Iterator<Item = &CombatantResult> {
        self.combatants.iter().filter(move |ship| ship.side == side)
    }
}
