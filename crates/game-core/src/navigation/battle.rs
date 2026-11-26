//! Raider battles played turn by turn. While a battle is open it lives in
//! `NavigationState::battle` and its encounter stays pending, so navigation runs
//! wait for it. Raider turns are played by the AI right after each player action:
//! an open battle always waits on a player ship.
use super::{Encounter, EncounterKind, NavigationError, NavigationEvent};
use crate::{
    combat::{
        CombatOutcome, CombatReport, Side,
        gear::ESCORT_KIND,
        tactical::{self, Battle, Hex},
    },
    economy::EconomyRules,
    fleet::{Fleet, FleetError, FleetId, FleetMovementError},
    travel::{PlayerId, TravelError, TravelState},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveBattle {
    pub encounter: u64,
    pub fleet: FleetId,
    /// The fight began after a failed escape attempt.
    pub evasion_failed: bool,
    pub battle: Battle,
}

/// What the acting player ship does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleAction {
    /// Move to a free hex within the remaining movement points (along the
    /// shortest path around asteroids).
    Move(Hex),
    /// Fire every weapon that reaches this unit (an index into `Battle::units`).
    Fire {
        target: u8,
    },
    EndTurn,
    /// Leave the battle from the player's edge.
    Withdraw,
    /// Let the AI play every remaining turn on both sides.
    AutoResolve,
    /// Give up: the raiders seize all cargo in the fleet and let its ships go.
    /// Against a hailed trader it just breaks the attack off.
    Surrender,
}

impl TravelState {
    /// Opens a battle for a raider encounter, then plays raider turns until a
    /// player ship is up (or, with `auto`, plays the whole battle).
    pub(super) fn start_battle(
        &mut self,
        encounter: &Encounter,
        fleet: &Fleet,
        evasion_failed: bool,
        auto: bool,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        if self.navigation.battle.is_some() {
            return Err(NavigationError::BattleUnderway.into());
        }
        let setups = self.engagement_setups(encounter, fleet, rules)?;
        let seed = self.navigation.rolls();
        let battle = Battle::new(&setups, &rules.combat_catalog, seed)
            .map_err(|_| FleetError::InvalidSelection)?;
        self.navigation.battle = Some(Box::new(ActiveBattle {
            encounter: encounter.id,
            fleet: fleet.id,
            evasion_failed,
            battle,
        }));
        if auto {
            self.act(BattleAction::AutoResolve, rules)
        } else {
            self.progress_battle(rules)
        }
    }

    /// Applies one player action to the open battle.
    pub fn battle_action(
        &mut self,
        player: PlayerId,
        action: BattleAction,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let open = self
            .navigation
            .battle
            .as_ref()
            .ok_or(NavigationError::NoBattle)?;
        let owner = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == open.fleet)
            .ok_or(FleetError::UnknownFleet)?
            .owner;
        if owner != player {
            return Err(FleetError::NotOwner.into());
        }
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(TravelError::TimeOverflow)?;
        let event = self.act(action, rules)?;
        self.revision = revision;
        Ok(event)
    }

    fn act(
        &mut self,
        action: BattleAction,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let weapons = &rules.combat_catalog;
        let open = self
            .navigation
            .battle
            .as_mut()
            .ok_or(NavigationError::NoBattle)?;
        let battle = &mut open.battle;
        match action {
            BattleAction::Move(to) => battle.move_to(to),
            BattleAction::Fire { target } => battle.fire(usize::from(target), weapons),
            BattleAction::EndTurn => battle.end_turn(),
            BattleAction::Withdraw => battle.withdraw(),
            BattleAction::AutoResolve => tactical::finish(battle, weapons),
            BattleAction::Surrender => battle.surrender(),
        }
        .map_err(NavigationError::Battle)?;
        self.progress_battle(rules)
    }

    /// Plays raider turns until a player ship is up; concludes a finished battle.
    fn progress_battle(
        &mut self,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let open = self
            .navigation
            .battle
            .as_mut()
            .ok_or(NavigationError::NoBattle)?;
        let battle = &mut open.battle;
        while let Some(unit) = battle.active()
            && battle.units[unit].side == Side::Hostile
        {
            tactical::play_turn(battle, &rules.combat_catalog).map_err(NavigationError::Battle)?;
        }
        if !battle.finished() {
            return Ok(NavigationEvent::BattleTurn { fleet: open.fleet });
        }
        self.conclude_battle(rules)
    }

    /// Writes a finished battle back to the fleet, files the combat report and
    /// clears the encounter.
    fn conclude_battle(
        &mut self,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        let open = self
            .navigation
            .battle
            .take()
            .ok_or(NavigationError::NoBattle)?;
        let encounter = self
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.id == open.encounter)
            .cloned()
            .ok_or(NavigationError::UnknownEncounter)?;
        let fleet = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == open.fleet)
            .cloned()
            .ok_or(FleetError::UnknownFleet)?;
        let result = open.battle.engagement();
        // Bounties are for raiders; an attacked trader or the police pay none.
        let bounty = match (&encounter.trader, encounter.police) {
            (None, None) => raider_bounty(&result.combatants, rules)?,
            _ => crate::economy::Credits(0),
        };
        let mut report = CombatReport {
            encounter: encounter.id,
            fleet: fleet.id,
            run: encounter.run,
            seed: open.battle.seed,
            outcome: if open.battle.surrendered {
                CombatOutcome::Surrendered
            } else {
                CombatOutcome::from_winner(result.winner)
            },
            combatants: result.combatants,
            rounds: result.rounds,
            salvage_ore: 0,
            cargo_lost: 0,
            crippled: None,
            bounty,
            trader: encounter.trader.as_ref().map(|trader| trader.id),
            police: encounter.police.map(|police| police.faction),
            plunder: 0,
            shakedown: encounter.kind == EncounterKind::Shakedown,
            rescue: encounter.rescue,
            rescue_reward: crate::economy::Credits(0),
        };
        self.apply_engagement(&fleet, &mut report)?;
        if report.outcome == CombatOutcome::Victory
            && let Some(survivors) = self.fleets.iter_mut().find(|record| record.id == fleet.id)
        {
            survivors.captain.gain(5);
        }
        // Raiders, a corsair (or the police, impounding it) take every cargo
        // unit still aboard a fleet that gives up; a trader the player
        // attacked just breaks away.
        if open.battle.surrendered && encounter.kind != EncounterKind::Trader {
            for id in &fleet.ships {
                if let Ok(ship) = self.ship_mut(*id) {
                    report.cargo_lost += ship.cargo.iter().sum::<u32>();
                    ship.cargo = [0; 6];
                }
            }
        }
        let evaded = if open.evasion_failed {
            "Evasion failed. "
        } else {
            ""
        };
        let outcome = format!(
            "{evaded}{} after {} rounds. Review the combat report.",
            report.outcome.label(),
            report.rounds.len()
        );
        let destroyed: Vec<_> = report
            .side(Side::Hostile)
            .filter(|ship| ship.destroyed)
            .map(|ship| ship.kind)
            .collect();
        if let Some(band) = encounter.band {
            self.band_after_contact(band, &destroyed);
        }
        if let Some(police) = encounter.police {
            self.police_after_contact(police.patrol, &destroyed);
        }
        // A destroyed escort ends its contract.
        if report
            .side(Side::Player)
            .any(|unit| unit.ship.is_none() && unit.destroyed)
        {
            self.navigation
                .escorts
                .retain(|escort| escort.fleet != fleet.id);
        }
        if encounter.haven {
            self.haven_after_contact(&destroyed);
        }
        self.navigation.last_combat = Some(Box::new(report));
        self.navigation
            .encounters
            .retain(|record| record.id != encounter.id);
        Ok(NavigationEvent::EncounterResolved {
            fleet: fleet.id,
            outcome,
        })
    }

    /// Save check: an open battle belongs to a pending raider (or hailed
    /// trader) encounter, fields exactly that fleet's ships and the contact's
    /// hostiles with their catalog
    /// profiles, and is in a state legal play can reach.
    pub fn validate_battle(&self, rules: &EconomyRules) -> Result<(), String> {
        let Some(open) = &self.navigation.battle else {
            return Ok(());
        };
        let invalid = || "Invalid saved battle".to_owned();
        open.battle.validate(&rules.combat_catalog)?;
        let encounter = self
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.id == open.encounter)
            .filter(|encounter| {
                matches!(
                    encounter.kind,
                    EncounterKind::Raiders
                        | EncounterKind::Trader
                        | EncounterKind::Police
                        | EncounterKind::Shakedown
                        | EncounterKind::TraderDistress
                )
            })
            .filter(|encounter| encounter.fleet == open.fleet)
            .ok_or_else(invalid)?;
        let fleet = self
            .fleets
            .iter()
            .find(|fleet| fleet.id == open.fleet)
            .ok_or_else(invalid)?;
        let units = &open.battle.units;
        let waiting = open.battle.active().map(|unit| units[unit].side);
        if waiting != Some(Side::Player) {
            return Err(invalid());
        }
        let mut ships: Vec<_> = units.iter().filter_map(|unit| unit.ship).collect();
        let mut expected = fleet.ships.clone();
        ships.sort_unstable_by_key(|id| id.0);
        expected.sort_unstable_by_key(|id| id.0);
        let mut raiders: Vec<_> = units
            .iter()
            .filter(|unit| unit.side == Side::Hostile)
            .map(|unit| unit.kind)
            .collect();
        let mut contact = encounter.hostiles.clone();
        raiders.sort_unstable_by_key(|kind| kind.label());
        contact.sort_unstable_by_key(|kind| kind.label());
        if ships != expected || raiders != contact {
            return Err(invalid());
        }
        let escorted = self.navigation.escort_runs(fleet.id) > 0;
        let escorts = units
            .iter()
            .filter(|unit| unit.side == Side::Player && unit.ship.is_none())
            .count();
        if escorts > usize::from(escorted) {
            return Err(invalid());
        }
        // The first hostile of a hail is the trader; anything else unowned
        // (raiders, escorts) starts undamaged and without gear.
        let trader_unit = encounter
            .trader
            .as_ref()
            .and_then(|_| units.iter().position(|unit| unit.side == Side::Hostile));
        for (index, unit) in units.iter().enumerate() {
            let spec = rules.ship_catalog.spec(unit.kind).ok_or_else(invalid)?;
            let mut expected_profile = spec.combat.clone();
            if unit.ship.is_some() {
                fleet.captain.combat_profile(&mut expected_profile);
            }
            let fits = match (unit.ship, &encounter.trader) {
                (Some(id), _) => {
                    let ship = self.ship(id).map_err(|_| invalid())?;
                    ship.kind == unit.kind
                        && ship.hull_damage == unit.starting_damage
                        && ship.gear == unit.gear
                }
                (None, Some(trader)) if Some(index) == trader_unit => {
                    unit.starting_damage == trader.hull_damage && unit.gear == trader.gear
                }
                (None, _) => {
                    let escort = unit.side == Side::Player;
                    unit.starting_damage == 0
                        && unit.gear.is_empty()
                        && (!escort || unit.kind == ESCORT_KIND)
                }
            };
            if expected_profile != unit.profile || !fits {
                return Err(invalid());
            }
        }
        Ok(())
    }
}

/// Bounty for every raider destroyed; raiders that fled pay nothing.
pub(crate) fn raider_bounty(
    combatants: &[crate::combat::engagement::CombatantResult],
    rules: &EconomyRules,
) -> Result<crate::economy::Credits, FleetMovementError> {
    combatants
        .iter()
        .filter(|ship| ship.side == Side::Hostile && ship.destroyed)
        .try_fold(0_u64, |total, ship| {
            total.checked_add(rules.raider_bounties.of(ship.kind).0)
        })
        .map(crate::economy::Credits)
        .ok_or(TravelError::TimeOverflow.into())
}
