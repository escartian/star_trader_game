//! Deterministic round-based fleet engagement. Pure: no session state, no clock.
//!
//! Rules (first-pass balance, abstract points):
//! - At most [`MAX_ROUNDS`]; if both sides still have ships, they disengage.
//! - Each round, living ships act by engine rating (high first). Ties are broken
//!   by a seeded initiative roll drawn once per engagement, so neither side
//!   always fires first. A ship destroyed earlier in the round does not act.
//! - Every fitted weapon fires `shots` times. Each shot targets the living enemy
//!   with the least remaining hull + shields (ties: lowest index), so fire focuses.
//! - Hit chance = accuracy − [`EVASION_PER_ENGINE`] × target engine, at least
//!   [`MIN_HIT_PERCENT`].
//! - A hit converts damage to shield points at `shield_efficiency_percent`. Any
//!   damage left after shields fail converts back at the same rate, then loses
//!   `max(armor − armor_penetration, 0)` and deals at least 1 hull point.
use super::{CombatCatalog, CombatProfile, rng::CombatRng};
use crate::{fleet::ShipKind, travel::ShipId};
use serde::{Deserialize, Serialize};

pub const MAX_ROUNDS: u32 = 8;
pub const EVASION_PER_ENGINE: u32 = 3;
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
    /// `None` when both sides survive the round limit.
    pub winner: Option<Side>,
}

struct Live<'a> {
    setup: &'a CombatantSetup,
    shields: u32,
    damage: u32,
}

impl Live<'_> {
    fn alive(&self) -> bool {
        self.damage < self.setup.profile.hull
    }
    fn durability(&self) -> u32 {
        self.setup.profile.hull - self.damage + self.shields
    }
}

pub fn resolve(
    setups: &[CombatantSetup],
    weapons: &CombatCatalog,
    seed: u64,
) -> Result<Engagement, String> {
    if setups.is_empty() || setups.len() > MAX_COMBATANTS {
        return Err("Unsupported engagement size".into());
    }
    for setup in setups {
        setup.profile.validate(weapons)?;
        if setup.hull_damage >= setup.profile.hull {
            return Err(format!("{} cannot fight while destroyed", setup.name));
        }
    }
    let mut rng = CombatRng::new(seed);
    let mut live: Vec<Live> = setups
        .iter()
        .map(|setup| Live {
            setup,
            shields: setup.profile.shields,
            damage: setup.hull_damage,
        })
        .collect();
    let mut tallies = vec![(0_u32, 0_u32, 0_u32); setups.len()];
    let initiative: Vec<u64> = setups.iter().map(|_| rng.next_u64()).collect();
    let mut order: Vec<usize> = (0..setups.len()).collect();
    order.sort_by_key(|index| {
        (
            std::cmp::Reverse(setups[*index].profile.engine),
            initiative[*index],
            *index,
        )
    });
    let mut rounds = Vec::new();
    while rounds.len() < MAX_ROUNDS as usize && winner(&live).is_none() {
        let mut shots = Vec::new();
        for &attacker in &order {
            if !live[attacker].alive() {
                continue;
            }
            for weapon_id in &setups[attacker].profile.loadout {
                let weapon = weapons.weapon(weapon_id).ok_or("Unknown weapon")?;
                for _ in 0..weapon.shots {
                    let Some(target) = pick_target(&live, setups[attacker].side) else {
                        break;
                    };
                    let evasion = EVASION_PER_ENGINE * setups[target].profile.engine;
                    let chance = weapon
                        .accuracy_percent
                        .saturating_sub(evasion)
                        .max(MIN_HIT_PERCENT);
                    let hit = rng.percent() < chance;
                    let (shield_damage, hull_damage) = if hit {
                        apply_hit(&mut live[target], weapon)
                    } else {
                        (0, 0)
                    };
                    let tally = &mut tallies[attacker];
                    tally.0 += u32::from(hit);
                    tally.1 += 1;
                    tally.2 += hull_damage;
                    shots.push(ShotRecord {
                        attacker: attacker as u8,
                        target: target as u8,
                        weapon: weapon.id.clone(),
                        hit,
                        shield_damage,
                        hull_damage,
                        destroyed: hull_damage > 0 && !live[target].alive(),
                    });
                }
            }
        }
        rounds.push(shots);
    }
    let combatants = live
        .iter()
        .zip(&tallies)
        .map(|(entry, (hits, shots, dealt))| CombatantResult {
            name: entry.setup.name.clone(),
            side: entry.setup.side,
            ship: entry.setup.ship,
            kind: entry.setup.kind,
            hull: entry.setup.profile.hull,
            starting_damage: entry.setup.hull_damage,
            final_damage: entry.damage,
            destroyed: !entry.alive(),
            hits: *hits,
            shots: *shots,
            hull_dealt: *dealt,
        })
        .collect();
    Ok(Engagement {
        combatants,
        rounds,
        winner: winner(&live),
    })
}

/// The side left standing, if exactly one is.
fn winner(live: &[Live]) -> Option<Side> {
    let standing = |side| {
        live.iter()
            .any(|ship| ship.setup.side == side && ship.alive())
    };
    match (standing(Side::Player), standing(Side::Hostile)) {
        (true, false) => Some(Side::Player),
        (false, true) => Some(Side::Hostile),
        _ => None,
    }
}

fn pick_target(live: &[Live], attacker: Side) -> Option<usize> {
    live.iter()
        .enumerate()
        .filter(|(_, ship)| ship.setup.side != attacker && ship.alive())
        .min_by_key(|(index, ship)| (ship.durability(), *index))
        .map(|(index, _)| index)
}

/// Returns (shield points removed, hull points dealt). Damage is capped at the
/// remaining hull so `final_damage` never exceeds the class hull.
fn apply_hit(target: &mut Live, weapon: &super::WeaponSpec) -> (u32, u32) {
    let efficiency = u64::from(weapon.shield_efficiency_percent);
    let damage = u64::from(weapon.damage);
    let shields = u64::from(target.shields);
    // Raw damage that reaches the hull after shields absorb what they can.
    let (shield_loss, passed) = if efficiency == 0 || shields == 0 {
        (0, damage)
    } else {
        let to_break = (shields * 100).div_ceil(efficiency);
        if damage < to_break {
            ((damage * efficiency / 100).min(shields), 0)
        } else {
            (shields, damage - to_break)
        }
    };
    target.shields -= shield_loss as u32;
    if passed == 0 {
        return (shield_loss as u32, 0);
    }
    let armor = target
        .setup
        .profile
        .armor
        .saturating_sub(weapon.armor_penetration);
    let remaining = target.setup.profile.hull - target.damage;
    let hull = (passed as u32).saturating_sub(armor).max(1).min(remaining);
    target.damage += hull;
    (shield_loss as u32, hull)
}
