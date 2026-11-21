//! What the acting ship can do: move within its movement points (around
//! asteroids), fire one volley along a clear line of fire, withdraw from its
//! own edge, or (for the player) surrender. Each action validates first and
//! changes nothing when rejected.
use super::{Battle, BattleError, COVER_PERCENT, Hex};
use crate::combat::{
    CombatCatalog, WeaponSpec,
    engagement::{ShotRecord, Side, adjusted_hit_chance, apply_hit},
    rng::CombatRng,
};

impl Battle {
    /// Whether an active ship other than `except` stands on `hex`.
    fn occupied(&self, hex: Hex, except: usize) -> bool {
        self.units
            .iter()
            .enumerate()
            .any(|(index, unit)| index != except && unit.active() && unit.at == hex)
    }

    /// Hexes the acting ship can still reach this turn: free hexes within its
    /// remaining movement points along a path around asteroids, sorted.
    pub fn reachable(&self) -> Vec<Hex> {
        let Some(me) = self.active().filter(|_| self.moves_left > 0) else {
            return Vec::new();
        };
        let from = self.units[me].at;
        let mut hexes: Vec<Hex> = self
            .steps_from(from, self.moves_left)
            .into_keys()
            .filter(|hex| *hex != from && !self.occupied(*hex, me))
            .collect();
        hexes.sort_by_key(|hex| (hex.q, hex.r));
        hexes
    }

    /// Enemies `unit` could fire on from where it stands: in reach of a weapon
    /// and in a clear line of fire.
    pub fn targets(&self, unit: usize, weapons: &CombatCatalog) -> Vec<usize> {
        let reach = self.max_range(unit, weapons);
        (0..self.units.len())
            .filter(|&other| {
                let target = &self.units[other];
                target.side != self.units[unit].side
                    && target.active()
                    && target.at.distance(self.units[unit].at) <= reach
                    && self.line_of_fire(self.units[unit].at, target.at)
            })
            .collect()
    }

    /// Longest range among the unit's fitted weapons (0 when unarmed).
    pub fn max_range(&self, unit: usize, weapons: &CombatCatalog) -> u32 {
        self.units[unit]
            .profile
            .loadout
            .iter()
            .filter_map(|id| weapons.weapon(id))
            .map(|weapon| weapon.range)
            .max()
            .unwrap_or(0)
    }

    /// Percent chance for one shot of `weapon` from `attacker` to hit `target`
    /// where they stand now, cover included.
    pub fn shot_chance(&self, attacker: usize, target: usize, weapon: &WeaponSpec) -> u32 {
        let (from, to) = (&self.units[attacker], &self.units[target]);
        let cover = if self.cover(to.at) { -COVER_PERCENT } else { 0 };
        adjusted_hit_chance(weapon, to.profile.engine, from.at.distance(to.at), cover)
    }

    /// Moves the acting ship to `to`, paying one point per hex step along the
    /// shortest path around asteroids. The rest of its movement stays
    /// available this turn.
    pub fn move_to(&mut self, to: Hex) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.moves_left == 0 {
            return Err(BattleError::AlreadyMoved);
        }
        let from = self.units[me].at;
        if to == from || !self.board.contains(to) || self.blocked(to) || self.occupied(to, me) {
            return Err(BattleError::Blocked);
        }
        let cost = *self
            .steps_from(from, self.moves_left)
            .get(&to)
            .ok_or(BattleError::TooFar)?;
        self.units[me].at = to;
        self.moves_left -= cost;
        self.acted = true;
        Ok(())
    }

    /// Fires every fitted weapon that reaches `target`; a target destroyed
    /// mid-volley takes no further shots.
    pub fn fire(&mut self, target: usize, weapons: &CombatCatalog) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.fired {
            return Err(BattleError::AlreadyFired);
        }
        if !self
            .units
            .get(target)
            .is_some_and(|unit| unit.active() && unit.side != self.units[me].side)
        {
            return Err(BattleError::InvalidTarget);
        }
        let (from, to) = (self.units[me].at, self.units[target].at);
        if !self.line_of_fire(from, to) {
            return Err(BattleError::NoLineOfFire);
        }
        let distance = from.distance(to);
        let volley: Vec<_> = self.units[me]
            .profile
            .loadout
            .iter()
            .filter_map(|id| weapons.weapon(id))
            .filter(|weapon| weapon.range >= distance)
            .cloned()
            .collect();
        if volley.is_empty() {
            return Err(BattleError::OutOfRange);
        }
        let mut rng = CombatRng::new(self.rng);
        for weapon in &volley {
            // Positions do not change mid-volley, so neither does the chance.
            let chance = self.shot_chance(me, target, weapon);
            for _ in 0..weapon.shots {
                if !self.units[target].alive() {
                    break;
                }
                let hit = rng.percent() < chance;
                let (shield_damage, hull_damage) = if hit {
                    let unit = &mut self.units[target];
                    apply_hit(&mut unit.shields, &mut unit.damage, &unit.profile, weapon)
                } else {
                    (0, 0)
                };
                let attacker = &mut self.units[me];
                attacker.shots += 1;
                attacker.hits += u32::from(hit);
                attacker.hull_dealt += hull_damage;
                let destroyed = hull_damage > 0 && !self.units[target].alive();
                self.current_round().push(ShotRecord {
                    attacker: me as u8,
                    target: target as u8,
                    weapon: weapon.id.clone(),
                    hit,
                    shield_damage,
                    hull_damage,
                    destroyed,
                });
            }
        }
        self.rng = rng.state();
        self.fired = true;
        self.acted = true;
        Ok(())
    }

    /// The player side gives up, on one of its own turns; ends the battle.
    pub fn surrender(&mut self) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.units[me].side != Side::Player {
            return Err(BattleError::NotYourTurn);
        }
        self.surrendered = true;
        Ok(())
    }

    /// Whether the acting ship stands on its own edge and may withdraw.
    pub fn can_withdraw(&self) -> bool {
        self.active().is_some_and(|me| {
            let unit = &self.units[me];
            self.board.at_home_edge(unit.side, unit.at)
        })
    }

    /// Leaves the battle from the acting ship's own edge; ends its turn.
    pub fn withdraw(&mut self) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if !self.can_withdraw() {
            return Err(BattleError::NotAtEdge);
        }
        self.units[me].withdrawn = true;
        self.acted = true;
        if !self.finished() {
            self.advance();
        }
        Ok(())
    }
}
