//! Turn-based tactical battles on a 3D grid, with HoMM-style ship initiative.
//! Pure and deterministic: the same seed and the same actions replay a battle.
//!
//! - The battle volume is a [`GridSize`] cube that grows with the fleets (10 cells
//!   a side for a duel). Distance is Chebyshev (one step may change all three
//!   axes), so every range is a cube.
//! - Each round, every active ship acts once in initiative order: engine rating,
//!   high first, ties broken by a seeded roll drawn once per battle.
//! - A turn: optionally move up to [`move_range`] cells to a free cell, optionally
//!   fire every fitted weapon whose range reaches one chosen enemy, then end the
//!   turn. A ship on its own side's edge may withdraw instead, leaving the battle
//!   with its damage so far.
//! - Hits and damage follow `engagement::{hit_chance, apply_hit}`.
//! - The battle is fought to a finish: it ends when a side has no active ships.
//!   It also ends, disengaged, after [`CEASEFIRE_ROUNDS`] whole rounds in which
//!   nobody fired or moved, and [`MAX_ROUNDS`] is a safety backstop normal play
//!   never reaches (every shot can hit and every hit deals at least one point).
//! - Morale (`morale`) drives the AI: badly damaged ships, and every ship of a
//!   broken side, retreat to their edge and withdraw.
mod ai;
mod grid;
pub mod morale;
#[cfg(test)]
mod morale_tests;
#[cfg(test)]
mod tests;
mod validation;

pub use ai::{auto_resolve, finish, play_turn};
pub use grid::{Cell, DUEL_EDGE, GridSize, MAX_EDGE, move_range};

use super::{
    CombatCatalog, CombatProfile,
    engagement::{
        CombatantResult, CombatantSetup, Engagement, MAX_COMBATANTS, ShotRecord, Side, apply_hit,
        hit_chance,
    },
    rng::CombatRng,
};
use crate::{fleet::ShipKind, travel::ShipId};
use serde::{Deserialize, Serialize};

/// Safety backstop that bounds saved shot logs; not a gameplay limit.
pub const MAX_ROUNDS: u32 = 200;
/// Consecutive whole rounds without a shot or a move after which both sides
/// break off.
pub const CEASEFIRE_ROUNDS: u32 = 3;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Unit {
    pub name: String,
    pub side: Side,
    pub ship: Option<ShipId>,
    pub kind: ShipKind,
    pub profile: CombatProfile,
    pub starting_damage: u32,
    /// Hull points lost so far; the ship is destroyed when this reaches its hull.
    pub damage: u32,
    pub shields: u32,
    pub cell: Cell,
    pub withdrawn: bool,
    pub hits: u32,
    pub shots: u32,
    pub hull_dealt: u32,
}

impl Unit {
    pub fn alive(&self) -> bool {
        self.damage < self.profile.hull
    }

    /// Still fighting: alive and not withdrawn.
    pub fn active(&self) -> bool {
        self.alive() && !self.withdrawn
    }

    fn durability(&self) -> u32 {
        self.profile.hull - self.damage + self.shields
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleError {
    Finished,
    AlreadyMoved,
    AlreadyFired,
    /// The cell is occupied or outside the battle volume.
    Blocked,
    TooFar,
    InvalidTarget,
    OutOfRange,
    NotAtEdge,
    /// Only the player side surrenders, on one of its own turns.
    NotYourTurn,
}

impl std::fmt::Display for BattleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Finished => "The battle is over.",
            Self::AlreadyMoved => "This ship has already moved this turn.",
            Self::AlreadyFired => "This ship has already fired this turn.",
            Self::Blocked => "That cell is occupied or outside the battle.",
            Self::TooFar => "That cell is beyond this ship's engine range.",
            Self::InvalidTarget => "Pick an enemy ship that is still fighting.",
            Self::OutOfRange => "No fitted weapon reaches that target.",
            Self::NotAtEdge => "Ships withdraw from their own edge of the battle.",
            Self::NotYourTurn => "You can surrender only on one of your ships' turns.",
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Battle {
    /// Seed drawn from the navigation RNG; recorded in the combat report.
    pub seed: u64,
    rng: u64,
    /// Battles saved before grids scaled load as the old 9 × 9 × 5 volume.
    #[serde(default)]
    pub size: GridSize,
    pub units: Vec<Unit>,
    /// Initiative order: indices into `units`.
    pub order: Vec<u8>,
    /// Current round, starting at 1.
    pub round: u32,
    /// Position in `order` of the acting ship.
    pub turn: usize,
    pub moved: bool,
    pub fired: bool,
    /// Shots fired, one list per round played.
    pub rounds: Vec<Vec<ShotRecord>>,
    /// Someone moved, fired or withdrew in the current round.
    #[serde(default)]
    pub acted: bool,
    /// Whole rounds in a row, just played, in which nobody acted.
    #[serde(default)]
    pub quiet_rounds: u32,
    /// The player side gave up; the battle is over.
    #[serde(default)]
    pub surrendered: bool,
}

impl Battle {
    pub fn new(
        setups: &[CombatantSetup],
        weapons: &CombatCatalog,
        seed: u64,
    ) -> Result<Self, String> {
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
        let size = GridSize::for_ships(setups.len());
        let mut placed = [0_usize; 2];
        let units: Vec<Unit> = setups
            .iter()
            .map(|setup| {
                let rank = &mut placed[usize::from(setup.side == Side::Hostile)];
                let cell = size.deployment_cell(setup.side, *rank);
                *rank += 1;
                Unit {
                    name: setup.name.clone(),
                    side: setup.side,
                    ship: setup.ship,
                    kind: setup.kind,
                    profile: setup.profile.clone(),
                    starting_damage: setup.hull_damage,
                    damage: setup.hull_damage,
                    shields: setup.profile.shields,
                    cell,
                    withdrawn: false,
                    hits: 0,
                    shots: 0,
                    hull_dealt: 0,
                }
            })
            .collect();
        let rolls: Vec<u64> = units.iter().map(|_| rng.next_u64()).collect();
        let mut order: Vec<u8> = (0..units.len() as u8).collect();
        order.sort_by_key(|&index| {
            let index = usize::from(index);
            (
                std::cmp::Reverse(units[index].profile.engine),
                rolls[index],
                index,
            )
        });
        Ok(Self {
            seed,
            rng: rng.state(),
            size,
            units,
            order,
            round: 1,
            turn: 0,
            moved: false,
            fired: false,
            rounds: vec![Vec::new()],
            acted: false,
            quiet_rounds: 0,
            surrendered: false,
        })
    }

    /// The acting ship, or `None` once the battle is over.
    pub fn active(&self) -> Option<usize> {
        (!self.finished()).then(|| usize::from(self.order[self.turn]))
    }

    pub fn finished(&self) -> bool {
        self.surrendered
            || self.round > MAX_ROUNDS
            || self.quiet_rounds >= CEASEFIRE_ROUNDS
            || !self.side_active(Side::Player)
            || !self.side_active(Side::Hostile)
    }

    /// Once finished, `Some(winner)`:
    /// - the player wins when no raider is left fighting (destroyed or fled);
    /// - raiders win only when every player ship was destroyed;
    /// - `None` (disengaged) when player ships escaped by withdrawing, or both
    ///   sides still fight (ceasefire or the safety cap).
    pub fn outcome(&self) -> Option<Option<Side>> {
        if !self.finished() {
            return None;
        }
        let player_alive = self
            .units
            .iter()
            .any(|unit| unit.side == Side::Player && unit.alive());
        Some(
            match (
                self.side_active(Side::Player),
                self.side_active(Side::Hostile),
            ) {
                (true, false) => Some(Side::Player),
                (false, _) if !player_alive => Some(Side::Hostile),
                _ => None,
            },
        )
    }

    fn side_active(&self, side: Side) -> bool {
        self.units
            .iter()
            .any(|unit| unit.side == side && unit.active())
    }

    fn occupied(&self, cell: Cell) -> bool {
        self.units
            .iter()
            .any(|unit| unit.active() && unit.cell == cell)
    }

    /// Free cells the acting ship can still move to this turn.
    pub fn reachable(&self) -> Vec<Cell> {
        let Some(me) = self.active().filter(|_| !self.moved) else {
            return Vec::new();
        };
        let from = self.units[me].cell;
        let reach = move_range(self.units[me].profile.engine) as i8;
        let mut cells = Vec::new();
        for x in from.x - reach..=from.x + reach {
            for y in from.y - reach..=from.y + reach {
                for z in from.z - reach..=from.z + reach {
                    let cell = Cell { x, y, z };
                    if cell != from && self.size.contains(cell) && !self.occupied(cell) {
                        cells.push(cell);
                    }
                }
            }
        }
        cells
    }

    /// Enemies the acting ship (or `unit`) could fire on from where it stands.
    pub fn targets(&self, unit: usize, weapons: &CombatCatalog) -> Vec<usize> {
        let reach = self.max_range(unit, weapons);
        (0..self.units.len())
            .filter(|&other| {
                let target = &self.units[other];
                target.side != self.units[unit].side
                    && target.active()
                    && target.cell.distance(self.units[unit].cell) <= reach
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

    pub fn move_to(&mut self, cell: Cell) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.moved {
            return Err(BattleError::AlreadyMoved);
        }
        if !self.size.contains(cell) || self.occupied(cell) {
            return Err(BattleError::Blocked);
        }
        if cell.distance(self.units[me].cell) > move_range(self.units[me].profile.engine) {
            return Err(BattleError::TooFar);
        }
        self.units[me].cell = cell;
        self.moved = true;
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
        let distance = self.units[me].cell.distance(self.units[target].cell);
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
            for _ in 0..weapon.shots {
                if !self.units[target].alive() {
                    break;
                }
                let chance = hit_chance(weapon, self.units[target].profile.engine, distance);
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

    /// Leaves the battle from the acting ship's own edge; ends its turn.
    pub fn withdraw(&mut self) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.units[me].cell.x != self.size.home_edge(self.units[me].side) {
            return Err(BattleError::NotAtEdge);
        }
        self.units[me].withdrawn = true;
        self.acted = true;
        if !self.finished() {
            self.advance();
        }
        Ok(())
    }

    /// Passes to the next active ship, starting a new round after the last one.
    pub fn end_turn(&mut self) -> Result<(), BattleError> {
        if self.finished() {
            return Err(BattleError::Finished);
        }
        self.advance();
        Ok(())
    }

    fn advance(&mut self) {
        self.moved = false;
        self.fired = false;
        loop {
            self.turn += 1;
            if self.turn == self.order.len() {
                self.turn = 0;
                self.round += 1;
                self.quiet_rounds = if self.acted { 0 } else { self.quiet_rounds + 1 };
                self.acted = false;
                if self.round > MAX_ROUNDS {
                    return;
                }
                self.rounds.push(Vec::new());
            }
            if self.finished() || self.units[usize::from(self.order[self.turn])].active() {
                return;
            }
        }
    }

    fn current_round(&mut self) -> &mut Vec<ShotRecord> {
        let index = self.rounds.len() - 1;
        &mut self.rounds[index]
    }

    /// Results in the shape the combat report and fleet updates expect.
    pub fn engagement(&self) -> Engagement {
        let combatants = self
            .units
            .iter()
            .map(|unit| CombatantResult {
                name: unit.name.clone(),
                side: unit.side,
                ship: unit.ship,
                kind: unit.kind,
                hull: unit.profile.hull,
                starting_damage: unit.starting_damage,
                final_damage: unit.damage,
                destroyed: !unit.alive(),
                hits: unit.hits,
                shots: unit.shots,
                hull_dealt: unit.hull_dealt,
            })
            .collect();
        Engagement {
            combatants,
            rounds: self.rounds.clone(),
            winner: self.outcome().flatten(),
        }
    }
}
