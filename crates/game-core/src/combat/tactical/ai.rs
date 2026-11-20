//! Ship AI for raiders and for Auto-resolve: focus the weakest enemy, close in until
//! weapons reach it, fire, end the turn. Badly damaged ships, and every ship of a
//! broken side (see `morale`), retreat instead. Deterministic: ties break by
//! index/cell.
use super::{Battle, BattleError, Cell, morale};
use crate::combat::{
    CombatCatalog,
    engagement::{CombatantSetup, Engagement},
};

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
        let goal = battle.units[target].cell;
        let here = battle.units[me].cell.distance(goal);
        if here > reach {
            // Close in: the reachable cell nearest the target, preferring the
            // middle layer and then the lowest cell so choices are reproducible.
            let middle = battle.size.height / 2;
            if let Some(cell) = battle.reachable().into_iter().min_by_key(|cell: &Cell| {
                (
                    cell.distance(goal),
                    (cell.z - middle).abs(),
                    cell.x,
                    cell.y,
                    cell.z,
                )
            }) && cell.distance(goal) < here
            {
                battle.move_to(cell)?;
            }
        }
        let in_reach = battle.targets(me, weapons);
        if let Some(target) = weakest_enemy(battle, me, Some(&in_reach)) {
            battle.fire(target, weapons)?;
        }
    }
    // The last shot may have ended the battle; there is then no turn to pass.
    if battle.finished() {
        return Ok(());
    }
    battle.end_turn()
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
    let edge = battle.size.home_edge(side);
    let to_edge = |cell: Cell| (cell.x - edge).abs();
    let here = battle.units[me].cell;
    let nearest_enemy = |cell: Cell| {
        battle
            .units
            .iter()
            .filter(|unit| unit.side != side && unit.active())
            .map(|unit| unit.cell.distance(cell))
            .min()
            .unwrap_or(u32::MAX)
    };
    if let Some(cell) = battle.reachable().into_iter().min_by_key(|cell| {
        (
            to_edge(*cell),
            std::cmp::Reverse(nearest_enemy(*cell)),
            cell.y,
            cell.z,
        )
    }) && to_edge(cell) < to_edge(here)
    {
        battle.move_to(cell)?;
    }
    if battle.units[me].cell.x == edge {
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
