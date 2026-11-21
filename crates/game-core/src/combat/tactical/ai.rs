//! Ship AI for raiders and for Auto-resolve: focus the weakest enemy, move to
//! a hex with a clear line of fire from which every weapon reaches it (in
//! cover when there is one), fire, end the turn. Badly damaged ships, and
//! every ship of a broken side (see `morale`), retreat instead. Deterministic:
//! ties break by hex.
use super::{Battle, BattleError, Hex, morale};
use crate::combat::{
    CombatCatalog, WeaponEffect,
    engagement::{CombatantSetup, Engagement},
};
use std::cmp::Reverse;

/// Path steps the AI looks across when closing in: the whole board.
const PATH_LIMIT: u32 = 64;

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
        let shortest = min_range(battle, me, weapons);
        // A good hex puts every weapon in reach, with a clear line of fire
        // unless every weapon is guided, so the whole volley fires.
        let sighted = battle.units[me]
            .profile
            .loadout
            .iter()
            .filter_map(|id| weapons.weapon(id))
            .any(|weapon| weapon.effect != WeaponEffect::Guided);
        let good =
            |at: Hex| at.distance(goal) <= shortest && (!sighted || battle.line_of_fire(at, goal));
        // Path steps toward the target, around asteroids.
        let toward = battle.steps_from(goal, PATH_LIMIT);
        let steps = |at: Hex| toward.get(&at).copied().unwrap_or(u32::MAX);
        let here = battle.units[me].at;
        let score = |at: Hex| {
            let fits = good(at);
            (
                !fits,
                if fits { 0 } else { steps(at) },
                !battle.cover(at),
                at != here,
                at.q,
                at.r,
            )
        };
        let best = battle
            .reachable()
            .into_iter()
            .chain(std::iter::once(here))
            .min_by_key(|at| score(*at));
        if let Some(to) = best
            && to != here
        {
            battle.move_to(to)?;
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
    let to_edge = |at: Hex| (at.q - edge).abs();
    let here = battle.units[me].at;
    let nearest_enemy = |at: Hex| {
        battle
            .units
            .iter()
            .filter(|unit| unit.side != side && unit.active())
            .map(|unit| unit.at.distance(at))
            .min()
            .unwrap_or(u32::MAX)
    };
    if let Some(to) = battle
        .reachable()
        .into_iter()
        .min_by_key(|to| (to_edge(*to), Reverse(nearest_enemy(*to)), to.r, to.q))
        && to_edge(to) < to_edge(here)
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
