//! What the acting ship can do: move within its movement points (around
//! asteroids), fire one volley along a clear line of fire, withdraw from its
//! own edge, or (for the player) surrender. Each action validates first and
//! changes nothing when rejected.
use super::{Battle, BattleError, COVER_PERCENT, Hex, Unit};
use crate::combat::{
    CombatCatalog, WeaponSpec,
    engagement::{ShotRecord, Side, adjusted_hit_chance, apply_hit},
    rng::CombatRng,
};
use crate::combat::{
    WeaponEffect,
    gear::{
        BULKHEAD_ARMOR_POINTS, EVASION_JETS_PERCENT, Gear, POINT_DEFENSE_PERCENT, TARGETING_PERCENT,
    },
};

pub(super) fn defense_profile(unit: &Unit) -> crate::combat::CombatProfile {
    let mut profile = unit.profile.clone();
    if unit.has(Gear::ArmoredBulkheads) {
        profile.armor = profile.armor.saturating_add(BULKHEAD_ARMOR_POINTS);
    }
    profile
}

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

    /// Weapons `unit` could fire from `from` at a ship on `to`: in range, and
    /// with a clear line of fire unless guided.
    pub fn volley_from(
        &self,
        unit: usize,
        from: Hex,
        to: Hex,
        weapons: &CombatCatalog,
    ) -> Vec<WeaponSpec> {
        let distance = from.distance(to);
        let sight = self.line_of_fire(from, to);
        self.units[unit]
            .profile
            .loadout
            .iter()
            .filter_map(|id| weapons.weapon(id))
            .filter(|weapon| {
                weapon.range >= distance && (sight || weapon.effect == WeaponEffect::Guided)
            })
            .cloned()
            .collect()
    }

    /// Enemies `unit` could fire on from where it stands: some weapon reaches
    /// them, in a clear line of fire unless it is guided.
    pub fn targets(&self, unit: usize, weapons: &CombatCatalog) -> Vec<usize> {
        let from = self.units[unit].at;
        (0..self.units.len())
            .filter(|&other| {
                let target = &self.units[other];
                target.side != self.units[unit].side
                    && target.active()
                    && !self.volley_from(unit, from, target.at, weapons).is_empty()
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
    /// where they stand now: cover (unless piercing), point defense (double
    /// against guided weapons) and range (unless steady).
    pub fn shot_chance(&self, attacker: usize, target: usize, weapon: &WeaponSpec) -> u32 {
        let (from, to) = (&self.units[attacker], &self.units[target]);
        let cover = if self.cover(to.at) && weapon.effect != WeaponEffect::Piercing {
            -COVER_PERCENT
        } else {
            0
        };
        let defense = match (to.has(Gear::PointDefense), weapon.effect) {
            (false, _) => 0,
            (true, WeaponEffect::Guided) => -2 * POINT_DEFENSE_PERCENT,
            (true, _) => -POINT_DEFENSE_PERCENT,
        };
        let distance = match weapon.effect {
            WeaponEffect::Steady => 1,
            _ => from.at.distance(to.at),
        };
        let targeting = if from.has(Gear::PatrolTargeting) {
            TARGETING_PERCENT
        } else {
            0
        };
        let jets = if to.has(Gear::EvasionJets) {
            -EVASION_JETS_PERCENT
        } else {
            0
        };
        adjusted_hit_chance(
            weapon,
            to.profile.engine,
            distance,
            cover + defense + targeting + jets,
        )
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
        let volley = self.volley_from(me, from, to, weapons);
        if volley.is_empty() {
            let reach = self.max_range(me, weapons);
            return Err(if from.distance(to) <= reach {
                BattleError::NoLineOfFire
            } else {
                BattleError::OutOfRange
            });
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
                    let profile = defense_profile(unit);
                    apply_hit(&mut unit.shields, &mut unit.damage, &profile, weapon)
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
                if hit {
                    self.after_hit(me, target, weapon);
                }
            }
        }
        self.rng = rng.state();
        self.fired = true;
        self.acted = true;
        Ok(())
    }

    /// A hit's weapon effect: a disruptor jams the target's engines; a ripple
    /// deals half damage (at least 1) to each active enemy beside the target.
    fn after_hit(&mut self, me: usize, target: usize, weapon: &WeaponSpec) {
        match weapon.effect {
            WeaponEffect::Disrupt if self.units[target].alive() => {
                self.units[target].jammed = true;
            }
            WeaponEffect::Ripple => {
                let centre = self.units[target].at;
                let side = self.units[target].side;
                let beside: Vec<usize> = (0..self.units.len())
                    .filter(|&other| {
                        let unit = &self.units[other];
                        other != target
                            && unit.side == side
                            && unit.active()
                            && unit.at.distance(centre) == 1
                    })
                    .collect();
                let wave = WeaponSpec {
                    damage: (weapon.damage / 2).max(1),
                    ..weapon.clone()
                };
                for other in beside {
                    let unit = &mut self.units[other];
                    let profile = defense_profile(unit);
                    let (shield_damage, hull_damage) =
                        apply_hit(&mut unit.shields, &mut unit.damage, &profile, &wave);
                    let destroyed = hull_damage > 0 && !self.units[other].alive();
                    self.units[me].hull_dealt += hull_damage;
                    self.current_round().push(ShotRecord {
                        attacker: me as u8,
                        target: other as u8,
                        weapon: weapon.id.clone(),
                        hit: true,
                        shield_damage,
                        hull_damage,
                        destroyed,
                    });
                }
            }
            _ => {}
        }
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
