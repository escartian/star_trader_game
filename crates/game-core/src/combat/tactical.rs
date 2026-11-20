//! Turn-based tactical battles on a hex board with three altitude bands, with
//! HoMM-style ship initiative. Pure and deterministic: the same seed and the
//! same actions replay a battle.
//!
//! - The board is a hexagon of hexes ([`Board`]) that grows with the fleets
//!   (radius 4 for a duel). Each ship stands on one hex in one [`Band`]: Low,
//!   Level or High. One ship per hex, whatever its band.
//! - Range is hex steps plus bands crossed ([`Position::range`]).
//! - Each round, every active ship acts once in initiative order: engine rating,
//!   high first, ties broken by a seeded roll drawn once per battle.
//! - A turn: spend up to [`move_points`] on hex steps and climbs/dives (one
//!   point each) to free hexes, fire once at one chosen enemy with every fitted
//!   weapon that reaches it, then end the turn. A ship on its own side's edge
//!   may withdraw instead, leaving the battle with its damage so far.
//! - Hits and damage follow `engagement::{adjusted_hit_chance, apply_hit}`;
//!   firing down a band gets +10% to hit, firing up one −10% ([`high_ground_percent`]).
//! - The battle is fought to a finish: it ends when a side has no active ships.
//!   It also ends, disengaged, after [`CEASEFIRE_ROUNDS`] whole rounds in which
//!   nobody fired or moved, and [`MAX_ROUNDS`] is a safety backstop normal play
//!   never reaches (every shot can hit and every hit deals at least one point).
//! - Morale (`morale`) drives the AI: badly damaged ships, and every ship of a
//!   broken side, retreat to their edge and withdraw.
mod actions;
mod ai;
mod board;
pub mod morale;
#[cfg(test)]
mod morale_tests;
#[cfg(test)]
mod tests;
mod validation;

pub use ai::{auto_resolve, finish, play_turn};
pub use board::{
    Band, Board, HIGH_GROUND_PERCENT, Hex, MAX_RADIUS, MIN_RADIUS, Position, high_ground_percent,
    move_points,
};

use super::{
    CombatCatalog, CombatProfile,
    engagement::{CombatantResult, CombatantSetup, Engagement, MAX_COMBATANTS, ShotRecord, Side},
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
    pub at: Position,
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
    /// No movement points are left this turn.
    AlreadyMoved,
    AlreadyFired,
    /// The hex is occupied or off the board, or the ship is already there.
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
            Self::AlreadyMoved => "This ship has no movement left this turn.",
            Self::AlreadyFired => "This ship has already fired this turn.",
            Self::Blocked => "That hex is occupied or off the board.",
            Self::TooFar => "That is beyond this ship's remaining movement.",
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
    pub board: Board,
    pub units: Vec<Unit>,
    /// Initiative order: indices into `units`.
    pub order: Vec<u8>,
    /// Current round, starting at 1.
    pub round: u32,
    /// Position in `order` of the acting ship.
    pub turn: usize,
    /// Movement points the acting ship has left this turn.
    pub moves_left: u32,
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
        let board = Board::for_ships(setups.len());
        let mut placed = [0_usize; 2];
        let units: Vec<Unit> = setups
            .iter()
            .map(|setup| {
                let rank = &mut placed[usize::from(setup.side == Side::Hostile)];
                let at = board.deployment(setup.side, *rank);
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
                    at,
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
        let moves_left = move_points(units[usize::from(order[0])].profile.engine);
        Ok(Self {
            seed,
            rng: rng.state(),
            board,
            units,
            order,
            round: 1,
            turn: 0,
            moves_left,
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

    /// Passes to the next active ship, starting a new round after the last one.
    pub fn end_turn(&mut self) -> Result<(), BattleError> {
        if self.finished() {
            return Err(BattleError::Finished);
        }
        self.advance();
        Ok(())
    }

    /// Whether the acting ship has spent any movement this turn.
    pub fn moved(&self) -> bool {
        self.active()
            .is_some_and(|me| self.moves_left < move_points(self.units[me].profile.engine))
    }

    fn advance(&mut self) {
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
            let next = &self.units[usize::from(self.order[self.turn])];
            let (active, engine) = (next.active(), next.profile.engine);
            if active {
                self.moves_left = move_points(engine);
            }
            if self.finished() || active {
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
