//! Side morale and the AI's retreat rule. Morale is derived from the battle
//! state (nothing extra is saved), so it always agrees with what happened.
//!
//! Morale (0–100) = 100
//!   − 50 × share of the side's hull lost in this battle
//!   − 30 × share of the side's ships destroyed
//!   − 20 × share of the side's ships withdrawn
//!   + 15 × share of enemy ships destroyed.
//!
//! The AI retreats a ship when its hull is at or below [`RETREAT_HULL_PERCENT`],
//! and every ship of a side whose morale fell below [`BREAK_MORALE`] (a rout).
//! Retreating ships take parting shots, head for their own edge and withdraw.
use super::Battle;
use crate::combat::Side;

/// Below this, a side breaks and the AI pulls every one of its ships out.
pub const BREAK_MORALE: u32 = 50;
/// A ship with this share of its hull or less retreats on its own.
pub const RETREAT_HULL_PERCENT: u32 = 30;

/// The side's morale, 0–100.
pub fn morale(battle: &Battle, side: Side) -> u32 {
    let own = || battle.units.iter().filter(move |unit| unit.side == side);
    let enemies = || battle.units.iter().filter(move |unit| unit.side != side);
    let ships = own().count().max(1) as i64;
    let enemy_ships = enemies().count().max(1) as i64;
    let at_start: i64 = own()
        .map(|unit| i64::from(unit.profile.hull - unit.starting_damage))
        .sum();
    let lost: i64 = own()
        .map(|unit| i64::from(unit.damage - unit.starting_damage))
        .sum();
    let destroyed = own().filter(|unit| !unit.alive()).count() as i64;
    let withdrawn = own().filter(|unit| unit.withdrawn).count() as i64;
    let kills = enemies().filter(|unit| !unit.alive()).count() as i64;
    let morale =
        100 - 50 * lost / at_start.max(1) - 30 * destroyed / ships - 20 * withdrawn / ships
            + 15 * kills / enemy_ships;
    morale.clamp(0, 100) as u32
}

/// Whether the AI would pull `unit` out: badly damaged, or its side is broken.
pub fn retreating(battle: &Battle, unit: usize) -> bool {
    let ship = &battle.units[unit];
    let left = u64::from(ship.profile.hull - ship.damage) * 100;
    left <= u64::from(RETREAT_HULL_PERCENT) * u64::from(ship.profile.hull)
        || morale(battle, ship.side) < BREAK_MORALE
}
