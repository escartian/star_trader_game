//! What the acting ship can do: move and change band within its movement
//! points, fire one volley, withdraw from its own edge, or (for the player)
//! surrender. Each action validates first and changes nothing when rejected.
use super::{Band, Battle, BattleError, Hex, Position, high_ground_percent};
use crate::combat::{
    CombatCatalog, WeaponSpec,
    engagement::{ShotRecord, Side, adjusted_hit_chance, apply_hit},
    rng::CombatRng,
};

impl Battle {
    /// Whether an active ship other than `except` stands on `hex` (any band).
    fn occupied(&self, hex: Hex, except: usize) -> bool {
        self.units
            .iter()
            .enumerate()
            .any(|(index, unit)| index != except && unit.active() && unit.at.hex == hex)
    }

    /// Positions the acting ship can still reach this turn: free hexes (or its
    /// own hex on another band) within its remaining movement points.
    pub fn reachable(&self) -> Vec<Position> {
        let Some(me) = self.active().filter(|_| self.moves_left > 0) else {
            return Vec::new();
        };
        let from = self.units[me].at;
        self.board
            .hexes()
            .filter(|hex| hex.distance(from.hex) <= self.moves_left && !self.occupied(*hex, me))
            .flat_map(|hex| Band::ALL.map(|band| Position::new(hex, band)))
            .filter(|to| *to != from && to.range(from) <= self.moves_left)
            .collect()
    }

    /// Enemies `unit` could fire on from where it stands.
    pub fn targets(&self, unit: usize, weapons: &CombatCatalog) -> Vec<usize> {
        let reach = self.max_range(unit, weapons);
        (0..self.units.len())
            .filter(|&other| {
                let target = &self.units[other];
                target.side != self.units[unit].side
                    && target.active()
                    && target.at.range(self.units[unit].at) <= reach
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
    /// where they stand now, high ground included.
    pub fn shot_chance(&self, attacker: usize, target: usize, weapon: &WeaponSpec) -> u32 {
        let (from, to) = (&self.units[attacker], &self.units[target]);
        adjusted_hit_chance(
            weapon,
            to.profile.engine,
            from.at.range(to.at),
            high_ground_percent(from.at.band, to.at.band),
        )
    }

    /// Moves the acting ship to `to`, paying one point per hex step and per
    /// band crossed. The rest of its movement stays available this turn.
    pub fn move_to(&mut self, to: Position) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.moves_left == 0 {
            return Err(BattleError::AlreadyMoved);
        }
        let from = self.units[me].at;
        if to == from || !self.board.contains(to.hex) || self.occupied(to.hex, me) {
            return Err(BattleError::Blocked);
        }
        let cost = from.range(to);
        if cost > self.moves_left {
            return Err(BattleError::TooFar);
        }
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
        let distance = self.units[me].at.range(self.units[target].at);
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
            self.board.at_home_edge(unit.side, unit.at.hex)
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
