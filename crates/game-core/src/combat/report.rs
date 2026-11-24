//! Saved, readable outcome of the most recent engagement.
use super::engagement::{CombatantResult, MAX_COMBATANTS, ShotRecord, Side};
use super::tactical::MAX_ROUNDS;
use crate::fleet::FleetId;
use crate::travel::ShipId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatOutcome {
    /// No hostile ship was left fighting: destroyed or fled.
    Victory,
    /// Every player ship in the fleet was destroyed.
    Defeat,
    /// Both sides still had ships: a side withdrew or both stopped firing.
    Disengaged,
    /// The player gave up: the raiders seized the fleet's cargo and let it go.
    Surrendered,
}

impl CombatOutcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Victory => "Victory",
            Self::Defeat => "Defeat",
            Self::Disengaged => "Disengaged",
            Self::Surrendered => "Surrendered",
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
    /// Cargo units lost with destroyed (or crippled) player ships, or seized
    /// on surrender.
    pub cargo_lost: u32,
    /// The player's last owned ship, destroyed in this battle: the campaign
    /// is lost. (The field name predates game over, when it limped away.)
    pub crippled: Option<ShipId>,
    /// Bounty earned for the raiders destroyed (credits in cents); the host
    /// pays it once, when the battle ends.
    #[serde(default)]
    pub bounty: crate::economy::Credits,
    /// The NPC trader fought (`npc::Trader::id`): one you attacked, or a
    /// corsair whose shakedown you resisted (`shakedown`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trader: Option<u64>,
    /// The trader was a corsair that tried to shake you down (no piracy).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shakedown: bool,
    /// The NPC trader you fought this band for, answering its distress call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rescue: Option<u64>,
    /// What that trader paid you for the rescue (credits in cents).
    #[serde(default, skip_serializing_if = "is_zero_credits")]
    pub rescue_reward: crate::economy::Credits,
    /// The faction whose police fought, for a battle against a patrol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub police: Option<crate::factions::Faction>,
    /// Cargo units taken from a destroyed trader's hold.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub plunder: u32,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// Generous bound for one saved shot list (8 weapons × 20 shots × 16 ships).
pub(crate) const MAX_SHOTS_PER_ROUND: usize = 8 * 20 * MAX_COMBATANTS;

impl CombatReport {
    pub fn validate(&self) -> Result<(), String> {
        let count = self.combatants.len();
        let invalid = count == 0
            || count > MAX_COMBATANTS
            || self.rounds.is_empty()
            || self.rounds.len() > MAX_ROUNDS as usize
            || self.bounty.0 > 1_000_000_000_000
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

fn is_zero_credits(credits: &crate::economy::Credits) -> bool {
    credits.0 == 0
}
