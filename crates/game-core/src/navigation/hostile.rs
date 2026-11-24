//! Raider contacts: hostile group generation, evasion and applying engagements
//! to owned ships. Battles themselves are played in `navigation::battle`. Combat arithmetic itself lives in `combat::engagement`.
use super::{Encounter, EncounterChoice, NavigationError, NavigationEvent};
use crate::{
    combat::{
        CombatOutcome, CombatProfile, CombatReport, CombatantSetup, Side,
        engagement::MAX_COMBATANTS,
        gear::{AFTERBURNER_EVADE_PERCENT, ESCORT_KIND, Gear},
    },
    economy::{Commodity, EconomyRules},
    fleet::{Fleet, FleetError, FleetId, FleetMovementError, ShipKind},
    travel::{ShipId, TravelError, TravelState},
};

/// Largest raider group; keeps first contacts survivable for small fleets.
pub const MAX_RAIDERS: usize = 3;
/// Largest raider group a save may hold. Play never exceeds [`MAX_RAIDERS`],
/// but developer fixtures stage bigger battles (within `MAX_COMBATANTS`).
pub const MAX_SAVED_RAIDERS: usize = 8;
/// Ore recovered per destroyed raider hull, if cargo space allows.
pub const SALVAGE_ORE_PER_RAIDER: u32 = 2;

/// A hired escort's combatant: an undamaged [`ESCORT_KIND`] without gear.
pub(crate) fn escort_setup(side: Side, name: String, profile: CombatProfile) -> CombatantSetup {
    CombatantSetup {
        name,
        side,
        ship: None,
        kind: ESCORT_KIND,
        profile,
        hull_damage: 0,
        gear: Vec::new(),
    }
}

/// Escape chance: 35% ± 10 points per engine-rating difference between the
/// fleet's slowest ship and the fastest raider, clamped to 10–90%.
pub fn evade_chance_percent(slowest_fleet_engine: u32, fastest_raider_engine: u32) -> u32 {
    let difference = i64::from(slowest_fleet_engine) - i64::from(fastest_raider_engine);
    (35 + 10 * difference).clamp(10, 90) as u32
}

impl TravelState {
    /// The fleet's chance to evade `hostiles` ([`evade_chance_percent`]),
    /// plus the afterburner bonus when every ship has afterburners (at most 90%).
    pub fn evade_chance(
        &self,
        fleet: &Fleet,
        hostiles: &[ShipKind],
        rules: &EconomyRules,
    ) -> Result<u32, FleetMovementError> {
        let catalog = &rules.ship_catalog;
        let engine = |kind| catalog.spec(kind).map_or(1, |spec| spec.combat.engine);
        let mut slowest = u32::MAX;
        let mut boosted = true;
        for id in &fleet.ships {
            let ship = self.ship(*id)?;
            slowest = slowest.min(engine(ship.kind));
            boosted &= ship.gear.contains(&Gear::Afterburners);
        }
        let fastest = hostiles.iter().map(|kind| engine(*kind)).max();
        let chance = evade_chance_percent(slowest, fastest.unwrap_or(1));
        let bonus = if boosted {
            AFTERBURNER_EVADE_PERCENT
        } else {
            0
        };
        Ok((chance + bonus).min(90))
    }

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

    /// Resolves Fight, Evade or Auto-resolve. A successful escape clears the
    /// encounter; otherwise a battle opens (and, for Auto-resolve, is fought out).
    pub(super) fn resolve_raiders(
        &mut self,
        encounter: &Encounter,
        fleet: &Fleet,
        choice: EncounterChoice,
        rules: &EconomyRules,
    ) -> Result<NavigationEvent, FleetMovementError> {
        if self.navigation.battle.is_some() {
            return Err(NavigationError::BattleUnderway.into());
        }
        let evading = choice == EncounterChoice::Evade;
        if evading && encounter.haven {
            // The player started this assault; calling it off always works.
            self.navigation
                .encounters
                .retain(|record| record.id != encounter.id);
            return Ok(NavigationEvent::EncounterResolved {
                fleet: fleet.id,
                outcome: "Assault on the Raider Haven called off.".into(),
            });
        }
        if evading {
            let chance = self.evade_chance(fleet, &encounter.hostiles, rules)?;
            if (self.navigation.rolls() % 100) < u64::from(chance) {
                self.navigation
                    .encounters
                    .retain(|record| record.id != encounter.id);
                if let Some(band) = encounter.band {
                    self.band_after_contact(band, &[]);
                }
                let foe = match (encounter.police, &encounter.trader) {
                    (Some(police), _) => {
                        self.police_after_contact(police.patrol, &[]);
                        format!("the {} patrol", police.faction.label())
                    }
                    (None, Some(corsair)) => corsair.name.clone(),
                    (None, None) => "the raiders".into(),
                };
                return Ok(NavigationEvent::EncounterResolved {
                    fleet: fleet.id,
                    outcome: format!("Evaded {foe} ({chance}% chance)."),
                });
            }
        }
        let auto = choice == EncounterChoice::AutoResolve;
        self.start_battle(encounter, fleet, evading, auto, rules)
    }

    pub(super) fn engagement_setups(
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
                gear: ship.gear.clone(),
            });
        }
        // A hired escort flies beside the fleet while there is room for it.
        let escorted = self.navigation.escort_runs(fleet.id) > 0;
        let room = setups.len() + encounter.hostiles.len() < MAX_COMBATANTS;
        if escorted && room {
            setups.push(escort_setup(
                Side::Player,
                "Hired escort".into(),
                profile(ESCORT_KIND)?,
            ));
        }
        for (index, kind) in encounter.hostiles.iter().enumerate() {
            // A hailed trader fights under its own name, with its own damage
            // and gear; a second hostile is its hired escort.
            let setup = match (&encounter.trader, index) {
                (Some(trader), 0) => CombatantSetup {
                    name: trader.name.clone(),
                    side: Side::Hostile,
                    ship: None,
                    kind: *kind,
                    profile: profile(*kind)?,
                    hull_damage: trader.hull_damage,
                    gear: trader.gear.clone(),
                },
                (Some(trader), _) => escort_setup(
                    Side::Hostile,
                    format!("{}'s escort", trader.name),
                    profile(*kind)?,
                ),
                (None, _) => CombatantSetup {
                    name: match encounter.police {
                        Some(police) => {
                            format!("{} {} {}", police.faction.label(), kind.label(), index + 1)
                        }
                        None => format!("Raider {} {}", kind.label(), index + 1),
                    },
                    side: Side::Hostile,
                    ship: None,
                    kind: *kind,
                    profile: profile(*kind)?,
                    hull_damage: 0,
                    gear: Vec::new(),
                },
            };
            setups.push(setup);
        }
        if setups.len() > MAX_COMBATANTS {
            return Err(FleetError::ShipLimit.into());
        }
        Ok(setups)
    }

    /// Writes hull damage back, removes destroyed ships (and an emptied fleet
    /// with its course), and loads salvage. Losing the player's last owned
    /// ship ends the campaign (`CampaignEnd::Lost`); its wreck stays on record
    /// at 1 hull point with its cargo lost, so the final state remains readable.
    pub(super) fn apply_engagement(
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
        // The last ship's wreck stays on record (the save needs a ship).
        let wreck = (destroyed.len() >= owned).then(|| destroyed[0]);
        for result in report.combatants.iter_mut() {
            let Some(id) = result.ship else { continue };
            let ship = self.ship_mut(id)?;
            if result.destroyed {
                report.cargo_lost += ship.cargo.iter().sum::<u32>();
            }
            ship.hull_damage = result.final_damage;
            if Some(id) == wreck {
                ship.cargo = [0; 6];
                ship.hull_damage = result.hull - 1;
                report.crippled = Some(id);
            }
        }
        let removed: Vec<ShipId> = destroyed
            .into_iter()
            .filter(|id| Some(*id) != wreck)
            .collect();
        if wreck.is_some() {
            self.campaign
                .ended
                .get_or_insert(crate::campaign::CampaignEnd::Lost {
                    run: self.navigation.run,
                });
        }
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
