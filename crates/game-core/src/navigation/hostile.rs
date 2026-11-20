//! Raider contacts: hostile group generation, evasion and applying engagements
//! to owned ships. Combat arithmetic itself lives in `combat::engagement`.
use super::{Encounter, EncounterChoice};
use crate::{
    combat::{
        CombatOutcome, CombatReport, CombatantSetup, Side,
        engagement::{self, MAX_COMBATANTS},
    },
    economy::{Commodity, EconomyRules},
    fleet::{Fleet, FleetError, FleetId, FleetMovementError, ShipKind},
    travel::{ShipId, TravelError, TravelState},
};

/// Largest raider group; keeps first contacts survivable for small fleets.
pub const MAX_RAIDERS: usize = 3;
/// Ore recovered per destroyed raider hull, if cargo space allows.
pub const SALVAGE_ORE_PER_RAIDER: u32 = 2;

/// Escape chance: 35% ± 10 points per engine-rating difference between the
/// fleet's slowest ship and the fastest raider, clamped to 10–90%.
pub fn evade_chance_percent(slowest_fleet_engine: u32, fastest_raider_engine: u32) -> u32 {
    let difference = i64::from(slowest_fleet_engine) - i64::from(fastest_raider_engine);
    (35 + 10 * difference).clamp(10, 90) as u32
}

impl TravelState {
    /// One raider per two fleet ships (rounded up, at most [`MAX_RAIDERS`]).
    /// Raiders lead with a patrol hull only when the fleet itself fields one,
    /// so an unarmed trader is never met by a warship.
    pub(super) fn raider_group(&self, fleet: FleetId) -> Result<Vec<ShipKind>, FleetError> {
        let fleet = self
            .fleets
            .iter()
            .find(|record| record.id == fleet)
            .ok_or(FleetError::UnknownFleet)?;
        let count = fleet.ships.len().div_ceil(2).clamp(1, MAX_RAIDERS);
        let armed = fleet.ships.iter().any(|id| {
            self.ship(*id)
                .is_ok_and(|ship| ship.kind == ShipKind::Patrol)
        });
        Ok((0..count)
            .map(|index| match (index, armed) {
                (0, true) => ShipKind::Patrol,
                _ => ShipKind::Scout,
            })
            .collect())
    }

    /// Resolves Fight or Evade; returns the readable outcome line.
    pub(super) fn resolve_raiders(
        &mut self,
        encounter: &Encounter,
        fleet: &Fleet,
        choice: EncounterChoice,
        rules: &EconomyRules,
    ) -> Result<String, FleetMovementError> {
        let catalog = &rules.ship_catalog;
        let engine = |kind| catalog.spec(kind).map_or(1, |spec| spec.combat.engine);
        if choice == EncounterChoice::Evade {
            let slowest = fleet
                .ships
                .iter()
                .map(|id| self.ship(*id).map(|ship| engine(ship.kind)))
                .try_fold(u32::MAX, |low, value| value.map(|value| low.min(value)))?;
            let fastest = encounter.hostiles.iter().map(|kind| engine(*kind)).max();
            let chance = evade_chance_percent(slowest, fastest.unwrap_or(1));
            if (self.navigation.rolls() % 100) < u64::from(chance) {
                return Ok(format!("Evaded the raiders ({chance}% chance)."));
            }
        }
        let setups = self.engagement_setups(encounter, fleet, rules)?;
        let seed = self.navigation.rolls();
        let result = engagement::resolve(&setups, &rules.combat_catalog, seed)
            .map_err(|_| FleetError::InvalidSelection)?;
        let mut report = CombatReport {
            encounter: encounter.id,
            fleet: fleet.id,
            run: encounter.run,
            seed,
            outcome: CombatOutcome::from_winner(result.winner),
            combatants: result.combatants,
            rounds: result.rounds,
            salvage_ore: 0,
            cargo_lost: 0,
            crippled: None,
        };
        self.apply_engagement(fleet, &mut report)?;
        let evaded = if choice == EncounterChoice::Evade {
            "Evasion failed. "
        } else {
            ""
        };
        let summary = format!(
            "{evaded}{} after {} rounds. Review the combat report.",
            report.outcome.label(),
            report.rounds.len()
        );
        self.navigation.last_combat = Some(Box::new(report));
        Ok(summary)
    }

    fn engagement_setups(
        &self,
        encounter: &Encounter,
        fleet: &Fleet,
        rules: &EconomyRules,
    ) -> Result<Vec<CombatantSetup>, FleetMovementError> {
        let profile = |kind| {
            rules
                .ship_catalog
                .spec(kind)
                .map(|spec| spec.combat.clone())
                .ok_or(FleetError::UnknownClass)
        };
        let mut setups = Vec::new();
        for id in &fleet.ships {
            let ship = self.ship(*id)?;
            setups.push(CombatantSetup {
                name: ship.name.clone(),
                side: Side::Player,
                ship: Some(ship.id),
                kind: ship.kind,
                profile: profile(ship.kind)?,
                hull_damage: ship.hull_damage,
            });
        }
        for (index, kind) in encounter.hostiles.iter().enumerate() {
            setups.push(CombatantSetup {
                name: format!("Raider {} {}", kind.label(), index + 1),
                side: Side::Hostile,
                ship: None,
                kind: *kind,
                profile: profile(*kind)?,
                hull_damage: 0,
            });
        }
        if setups.len() > MAX_COMBATANTS {
            return Err(FleetError::ShipLimit.into());
        }
        Ok(setups)
    }

    /// Writes hull damage back, removes destroyed ships (and an emptied fleet
    /// with its course), and loads salvage. The player's last owned ship is
    /// never removed: it survives crippled at 1 hull point with its cargo lost.
    fn apply_engagement(
        &mut self,
        fleet: &Fleet,
        report: &mut CombatReport,
    ) -> Result<(), FleetMovementError> {
        let owned = self
            .ships
            .iter()
            .filter(|ship| ship.owner == fleet.owner)
            .count();
        let destroyed: Vec<ShipId> = report
            .side(Side::Player)
            .filter(|ship| ship.destroyed)
            .filter_map(|ship| ship.ship)
            .collect();
        let spared = (destroyed.len() >= owned).then(|| destroyed[0]);
        for result in report.combatants.iter_mut() {
            let Some(id) = result.ship else { continue };
            let ship = self.ship_mut(id)?;
            if result.destroyed {
                report.cargo_lost += ship.cargo.iter().sum::<u32>();
            }
            if Some(id) == spared {
                ship.cargo = [0; 6];
                result.destroyed = false;
                result.final_damage = result.hull - 1;
                report.crippled = Some(id);
            }
            ship.hull_damage = result.final_damage;
        }
        let removed: Vec<ShipId> = destroyed
            .into_iter()
            .filter(|id| Some(*id) != spared)
            .collect();
        self.ships.retain(|ship| !removed.contains(&ship.id));
        for record in &mut self.fleets {
            record.ships.retain(|id| !removed.contains(id));
        }
        self.fleets.retain(|record| !record.ships.is_empty());
        if !self.fleets.iter().any(|record| record.id == fleet.id) {
            self.navigation
                .orders
                .retain(|order| order.plan.fleet != fleet.id);
        }
        // A destroyed fleet's trade route goes with it.
        self.prune_standing_orders();
        if report.outcome == CombatOutcome::Victory {
            let wrecks = report
                .side(Side::Hostile)
                .filter(|ship| ship.destroyed)
                .count() as u32;
            let mut salvage = wrecks
                .checked_mul(SALVAGE_ORE_PER_RAIDER)
                .ok_or(TravelError::TimeOverflow)?;
            for id in &fleet.ships {
                let Ok(ship) = self.ship_mut(*id) else {
                    continue;
                };
                let space = u64::from(ship.cargo_capacity).saturating_sub(ship.cargo_used());
                let loaded = salvage.min(space.min(u64::from(u32::MAX)) as u32);
                ship.cargo[Commodity::Ore.index()] += loaded;
                salvage -= loaded;
                report.salvage_ore += loaded;
            }
        }
        Ok(())
    }
}
