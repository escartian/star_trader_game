//! Structural checks for a battle loaded from a save.
use super::{Battle, CEASEFIRE_ROUNDS, MAX_ROUNDS, move_points, terrain};
use crate::combat::{
    CombatCatalog,
    engagement::{MAX_COMBATANTS, Side},
    report::MAX_SHOTS_PER_ROUND,
};
use std::collections::HashSet;

impl Battle {
    /// Rejects battles no sequence of legal actions could produce: bad indices,
    /// impossible damage, two ships on one hex, a ship on an asteroid, terrain
    /// other than the seed gives, more movement left than the acting ship has,
    /// or a finished battle left open.
    pub fn validate(&self, weapons: &CombatCatalog) -> Result<(), String> {
        let count = self.units.len();
        let mut order: Vec<usize> = self.order.iter().map(|&index| usize::from(index)).collect();
        order.sort_unstable();
        let has_side = |side| self.units.iter().any(|unit| unit.side == side);
        if count == 0
            || count > MAX_COMBATANTS
            || !has_side(Side::Player)
            || !has_side(Side::Hostile)
            || order != (0..count).collect::<Vec<_>>()
            || self.turn >= count
            || !(1..=MAX_ROUNDS).contains(&self.round)
            || self.rounds.len() != self.round as usize
            || !self.board.valid_for(count)
            || self.quiet_rounds >= CEASEFIRE_ROUNDS
            || self.terrain != terrain::generate(self.board, self.seed)
            || self.finished()
        {
            return Err("Invalid battle turn state".into());
        }
        let acting = &self.units[usize::from(self.order[self.turn])];
        if !acting.active() || self.moves_left > move_points(acting.profile.engine) {
            return Err("Invalid battle turn state".into());
        }
        let mut hexes = HashSet::new();
        for unit in &self.units {
            unit.profile.validate(weapons)?;
            if unit.name.is_empty()
                || unit.name.len() > 128
                || unit.starting_damage >= unit.profile.hull
                || unit.damage < unit.starting_damage
                || unit.damage > unit.profile.hull
                || unit.shields > unit.profile.shields
                || unit.hits > unit.shots
                || !self.board.contains(unit.at)
                || self.blocked(unit.at)
                || (unit.side == Side::Hostile) == unit.ship.is_some()
                || (unit.active() && !hexes.insert(unit.at))
            {
                return Err(format!("Invalid battle state for {}", unit.name));
            }
        }
        let bad_shot = self.rounds.iter().any(|shots| {
            shots.len() > MAX_SHOTS_PER_ROUND
                || shots.iter().any(|shot| {
                    usize::from(shot.attacker) >= count
                        || usize::from(shot.target) >= count
                        || (!shot.hit && (shot.hull_damage > 0 || shot.shield_damage > 0))
                })
        });
        if bad_shot {
            return Err("Invalid battle shot log".into());
        }
        Ok(())
    }
}
