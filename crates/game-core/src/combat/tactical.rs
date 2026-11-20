//! Turn-based tactical battles on a 3D grid, with HoMM-style ship initiative.
//! Pure and deterministic: the same seed and the same actions replay a battle.
//!
//! - The battle volume is [`WIDTH`] × [`DEPTH`] × [`HEIGHT`] cells. Distance is
//!   Chebyshev (one step may change all three axes), so every range is a cube.
//! - Each round, every active ship acts once in initiative order: engine rating,
//!   high first, ties broken by a seeded roll drawn once per battle.
//! - A turn: optionally move up to [`move_range`] cells to a free cell, optionally
//!   fire every fitted weapon whose range reaches one chosen enemy, then end the
//!   turn. A ship on its own side's edge may withdraw instead, leaving the battle
//!   with its damage so far.
//! - Hits and damage follow `engagement::{hit_chance, apply_hit}`.
//! - The battle ends when a side has no active ships, or after [`MAX_ROUNDS`].
mod ai;
#[cfg(test)]
mod tests;

pub use ai::{auto_resolve, finish, play_turn};

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

/// Cells along x, the axis between the two deployment edges.
pub const WIDTH: i8 = 9;
/// Cells along y.
pub const DEPTH: i8 = 9;
/// Cells along z (vertical layers).
pub const HEIGHT: i8 = 5;
pub const MAX_ROUNDS: u32 = 10;

/// A grid cell; valid cells satisfy [`Cell::in_bounds`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Cell {
    pub x: i8,
    pub y: i8,
    pub z: i8,
}

impl Cell {
    /// Chebyshev distance in cells.
    pub fn distance(self, other: Cell) -> u32 {
        let axis = |a: i8, b: i8| u32::from((i16::from(a) - i16::from(b)).unsigned_abs());
        axis(self.x, other.x)
            .max(axis(self.y, other.y))
            .max(axis(self.z, other.z))
    }

    pub fn in_bounds(self) -> bool {
        (0..WIDTH).contains(&self.x)
            && (0..DEPTH).contains(&self.y)
            && (0..HEIGHT).contains(&self.z)
    }
}

/// Cells a ship may move per turn: 1 + engine / 3 (engine 3 → 2, engine 8 → 3).
pub fn move_range(engine: u32) -> u32 {
    1 + engine / 3
}

/// The x column a side deploys on and withdraws from.
pub fn home_edge(side: Side) -> i8 {
    match side {
        Side::Player => 0,
        Side::Hostile => WIDTH - 1,
    }
}

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
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Battle {
    /// Seed drawn from the navigation RNG; recorded in the combat report.
    pub seed: u64,
    rng: u64,
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
        let mut placed = [0_usize; 2];
        let units: Vec<Unit> = setups
            .iter()
            .map(|setup| {
                let rank = &mut placed[usize::from(setup.side == Side::Hostile)];
                let cell = deployment_cell(setup.side, *rank);
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
            units,
            order,
            round: 1,
            turn: 0,
            moved: false,
            fired: false,
            rounds: vec![Vec::new()],
        })
    }

    /// The acting ship, or `None` once the battle is over.
    pub fn active(&self) -> Option<usize> {
        (!self.finished()).then(|| usize::from(self.order[self.turn]))
    }

    pub fn finished(&self) -> bool {
        self.round > MAX_ROUNDS
            || !self.side_active(Side::Player)
            || !self.side_active(Side::Hostile)
    }

    /// Once finished: the side whose enemies were all destroyed, or `None` when
    /// both sides still have ships (round limit or withdrawal).
    pub fn outcome(&self) -> Option<Option<Side>> {
        if !self.finished() {
            return None;
        }
        let alive = |side| {
            self.units
                .iter()
                .any(|unit| unit.side == side && unit.alive())
        };
        Some(match (alive(Side::Player), alive(Side::Hostile)) {
            (true, false) => Some(Side::Player),
            (false, true) => Some(Side::Hostile),
            _ => None,
        })
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
                    if cell != from && cell.in_bounds() && !self.occupied(cell) {
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
        if !cell.in_bounds() || self.occupied(cell) {
            return Err(BattleError::Blocked);
        }
        if cell.distance(self.units[me].cell) > move_range(self.units[me].profile.engine) {
            return Err(BattleError::TooFar);
        }
        self.units[me].cell = cell;
        self.moved = true;
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
        Ok(())
    }

    /// Leaves the battle from the acting ship's own edge; ends its turn.
    pub fn withdraw(&mut self) -> Result<(), BattleError> {
        let me = self.active().ok_or(BattleError::Finished)?;
        if self.units[me].cell.x != home_edge(self.units[me].side) {
            return Err(BattleError::NotAtEdge);
        }
        self.units[me].withdrawn = true;
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

/// Deployment: a side's ships fill its second column (one cell in from the edge,
/// so a damaged ship can step back and withdraw), nearest the centre first.
fn deployment_cell(side: Side, rank: usize) -> Cell {
    let x = match side {
        Side::Player => 1,
        Side::Hostile => WIDTH - 2,
    };
    let (mid_y, mid_z) = (DEPTH / 2, HEIGHT / 2);
    let mut cells: Vec<Cell> = (0..DEPTH)
        .flat_map(|y| (0..HEIGHT).map(move |z| Cell { x, y, z }))
        .collect();
    cells.sort_by_key(|cell| {
        (
            (cell.y - mid_y).abs() * 2 + (cell.z - mid_z).abs(),
            cell.y,
            cell.z,
        )
    });
    cells[rank % cells.len()]
}
