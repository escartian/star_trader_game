//! Ship AI for raiders and for Auto-resolve: focus the weakest enemy, close in until
//! every weapon reaches it, take the higher band when that costs nothing, fire,
//! end the turn. Badly damaged ships, and every ship of a broken side (see `morale`),
//! retreat instead. Deterministic: ties break by band, then hex.
use super::{Battle, BattleError, Position, morale};
use crate::combat::{
    CombatCatalog,
    engagement::{CombatantSetup, Engagement},
};
use std::cmp::Reverse;

/// Plays the acting ship's whole turn.
pub fn play_turn(battle: &mut Battle, weapons: &CombatCatalog) -> Result<(), BattleError> {
    let me = battle.active().ok_or(BattleError::Finished)?;
    if morale::retreating(battle, me) {
        return retreat(battle, me, weapons);
    }
    let reach = battle.max_range(me, weapons);
    if reach > 0
        && let Some(target) = weakest_enemy(battle, me, None)
    {
        let goal = battle.units[target].at;
        let here = battle.units[me].at.range(goal);
        // Close until the shortest-ranged weapon reaches too, so the whole
        // volley fires rather than long-range weapons alone.
        if here > min_range(battle, me, weapons) {
            // Close in: the reachable position nearest the target, then the
            // highest band, then the lowest hex so choices are reproducible.
            if let Some(to) = battle
                .reachable()
                .into_iter()
                .min_by_key(|to: &Position| (to.range(goal), Reverse(to.band), to.hex.q, to.hex.r))
                && to.range(goal) < here
            {
                battle.move_to(to)?;
            }
        }
        let in_reach = battle.targets(me, weapons);
        if let Some(target) = weakest_enemy(battle, me, Some(&in_reach)) {
            climb_if_free(battle, me, target, weapons)?;
            battle.fire(target, weapons)?;
        }
    }
    // The last shot may have ended the battle; there is then no turn to pass.
    if battle.finished() {
        return Ok(());
    }
    battle.end_turn()
}

/// Shortest range among the unit's fitted weapons (0 when unarmed).
fn min_range(battle: &Battle, me: usize, weapons: &CombatCatalog) -> u32 {
    battle.units[me]
        .profile
        .loadout
        .iter()
        .filter_map(|id| weapons.weapon(id))
        .map(|weapon| weapon.range)
        .min()
        .unwrap_or(0)
}

/// Climbs one band before firing when it costs nothing: a movement point is
/// spare, the target is not below (so climbing gains the high ground or
/// removes a penalty), and no weapon that reaches the target now would stop
/// reaching it.
fn climb_if_free(
    battle: &mut Battle,
    me: usize,
    target: usize,
    weapons: &CombatCatalog,
) -> Result<(), BattleError> {
    let (from, goal) = (battle.units[me].at, battle.units[target].at);
    let Some(band) = from.band.above() else {
        return Ok(());
    };
    if battle.moves_left == 0 || from.band > goal.band {
        return Ok(());
    }
    let climbed = Position::new(from.hex, band);
    let in_reach = |at: Position| {
        battle.units[me]
            .profile
            .loadout
            .iter()
            .filter_map(|id| weapons.weapon(id))
            .filter(|weapon| weapon.range >= at.range(goal))
            .count()
    };
    if in_reach(climbed) >= in_reach(from) {
        battle.move_to(climbed)?;
    }
    Ok(())
}

/// Parting shot at the weakest enemy in reach, then toward the own edge (ties:
/// away from the nearest enemy) and out of the battle once there.
fn retreat(battle: &mut Battle, me: usize, weapons: &CombatCatalog) -> Result<(), BattleError> {
    let in_reach = battle.targets(me, weapons);
    if let Some(target) = weakest_enemy(battle, me, Some(&in_reach)) {
        battle.fire(target, weapons)?;
        if battle.finished() {
            return Ok(());
        }
    }
    let side = battle.units[me].side;
    let edge = battle.board.home_column(side);
    let to_edge = |at: Position| (at.hex.q - edge).abs();
    let here = battle.units[me].at;
    let nearest_enemy = |at: Position| {
        battle
            .units
            .iter()
            .filter(|unit| unit.side != side && unit.active())
            .map(|unit| unit.at.range(at))
            .min()
            .unwrap_or(u32::MAX)
    };
    if let Some(to) = battle.reachable().into_iter().min_by_key(|to| {
        (
            to_edge(*to),
            Reverse(nearest_enemy(*to)),
            to.band.difference(here.band),
            to.hex.r,
            to.band,
        )
    }) && to_edge(to) < to_edge(here)
    {
        battle.move_to(to)?;
    }
    if battle.can_withdraw() {
        return battle.withdraw();
    }
    battle.end_turn()
}

/// Fights a whole battle with the AI on both sides.
pub fn auto_resolve(
    setups: &[CombatantSetup],
    weapons: &CombatCatalog,
    seed: u64,
) -> Result<Engagement, String> {
    let mut battle = Battle::new(setups, weapons, seed)?;
    finish(&mut battle, weapons).map_err(|error| error.to_string())?;
    Ok(battle.engagement())
}

/// Plays every remaining turn with the AI.
pub fn finish(battle: &mut Battle, weapons: &CombatCatalog) -> Result<(), BattleError> {
    while !battle.finished() {
        play_turn(battle, weapons)?;
    }
    Ok(())
}

/// The active enemy with the least hull + shields left (ties: lowest index),
/// optionally limited to `among`.
fn weakest_enemy(battle: &Battle, me: usize, among: Option<&[usize]>) -> Option<usize> {
    let side = battle.units[me].side;
    (0..battle.units.len())
        .filter(|index| among.is_none_or(|allowed| allowed.contains(index)))
        .filter(|&index| battle.units[index].side != side && battle.units[index].active())
        .min_by_key(|&index| (battle.units[index].durability(), index))
}
