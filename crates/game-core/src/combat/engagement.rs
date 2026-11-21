//! Shared combat results and damage rules. Battles themselves are turn-based on a
//! hex board with altitude bands (`combat::tactical`); this module is pure
//! arithmetic and data.
//!
//! - Hit chance = accuracy − [`EVASION_PER_ENGINE`] × target engine −
//!   [`RANGE_PENALTY_PERCENT`] × (distance − 1) ± the high-ground modifier, at
//!   least [`MIN_HIT_PERCENT`].
//! - A hit converts damage to shield points at `shield_efficiency_percent`. Any
//!   damage left after shields fail converts back at the same rate, then loses
//!   `max(armor − armor_penetration, 0)` and deals at least 1 hull point.
use super::{CombatProfile, WeaponSpec};
use crate::{fleet::ShipKind, travel::ShipId};
use serde::{Deserialize, Serialize};

pub const EVASION_PER_ENGINE: u32 = 3;
/// Accuracy lost per step of range (hex or band) beyond adjacent.
pub const RANGE_PENALTY_PERCENT: u32 = 4;
pub const MIN_HIT_PERCENT: u32 = 5;
/// Upper bound on combatants in one engagement; keeps reports small.
pub const MAX_COMBATANTS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Player,
    Hostile,
}

/// One participant as it enters the engagement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatantSetup {
    pub name: String,
    pub side: Side,
    /// Owned ship identity; `None` for transient hostile contacts.
    pub ship: Option<ShipId>,
    pub kind: ShipKind,
    pub profile: CombatProfile,
    pub hull_damage: u32,
    /// Fitted anti-piracy gear (`combat::gear`).
    pub gear: Vec<super::gear::Gear>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatantResult {
    pub name: String,
    pub side: Side,
    pub ship: Option<ShipId>,
    pub kind: ShipKind,
    pub hull: u32,
    pub starting_damage: u32,
    pub final_damage: u32,
    pub destroyed: bool,
    pub hits: u32,
    pub shots: u32,
    /// Hull points this combatant inflicted on enemies.
    pub hull_dealt: u32,
}

/// One weapon shot; indices refer to `CombatReport::combatants`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShotRecord {
    pub attacker: u8,
    pub target: u8,
    pub weapon: String,
    pub hit: bool,
    pub shield_damage: u32,
    pub hull_damage: u32,
    pub destroyed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Engagement {
    pub combatants: Vec<CombatantResult>,
    pub rounds: Vec<Vec<ShotRecord>>,
    /// `None` when both sides still have ships (withdrawal or ceasefire).
    pub winner: Option<Side>,
}

/// Percent chance for one shot of `weapon` to hit a target with `target_engine`
/// at `distance` (hex steps plus bands crossed).
pub fn hit_chance(weapon: &WeaponSpec, target_engine: u32, distance: u32) -> u32 {
    adjusted_hit_chance(weapon, target_engine, distance, 0)
}

/// [`hit_chance`] shifted by `modifier_percent` points (the high-ground rule)
/// before the [`MIN_HIT_PERCENT`] floor; never above 100.
pub fn adjusted_hit_chance(
    weapon: &WeaponSpec,
    target_engine: u32,
    distance: u32,
    modifier_percent: i32,
) -> u32 {
    let penalty =
        EVASION_PER_ENGINE * target_engine + RANGE_PENALTY_PERCENT * distance.saturating_sub(1);
    let chance =
        i64::from(weapon.accuracy_percent) - i64::from(penalty) + i64::from(modifier_percent);
    chance.clamp(i64::from(MIN_HIT_PERCENT), 100) as u32
}

/// Applies one hit to a target's `shields` and accumulated hull `damage`.
/// Returns (shield points removed, hull points dealt); hull damage is capped at
/// the remaining hull so it never exceeds the class hull.
pub fn apply_hit(
    shields: &mut u32,
    damage: &mut u32,
    profile: &CombatProfile,
    weapon: &WeaponSpec,
) -> (u32, u32) {
    let efficiency = u64::from(weapon.shield_efficiency_percent);
    let raw = u64::from(weapon.damage);
    let held = u64::from(*shields);
    // Raw damage that reaches the hull after shields absorb what they can.
    let (shield_loss, passed) = if efficiency == 0 || held == 0 {
        (0, raw)
    } else {
        let to_break = (held * 100).div_ceil(efficiency);
        if raw < to_break {
            ((raw * efficiency / 100).min(held), 0)
        } else {
            (held, raw - to_break)
        }
    };
    *shields -= shield_loss as u32;
    if passed == 0 {
        return (shield_loss as u32, 0);
    }
    let armor = profile.armor.saturating_sub(weapon.armor_penetration);
    let remaining = profile.hull - *damage;
    let hull = (passed as u32).saturating_sub(armor).max(1).min(remaining);
    *damage += hull;
    (shield_loss as u32, hull)
}
