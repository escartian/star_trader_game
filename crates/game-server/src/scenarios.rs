//! Reproducible test scenarios built through ordinary authoritative commands.
//! The only non-command steps are an explicit, documented test budget grant, the
//! encounter-chance override that makes contact tests deterministic, and the
//! fleet-battle fixture's restocked home shipyard and raider group.
use super::Session;
use game_core::{
    BodyId, BodyKind, SystemId,
    economy::Credits,
    fleet::{FleetId, ShipKind},
    navigation::{EncounterChoice, EncounterKind},
    travel::ShipId,
};
use game_protocol::{ClientCommand, ServerMessage};

/// Credits granted so the scenario can afford its extra hulls (hundredths: 25,000.00 cr).
const TEST_BUDGET: Credits = Credits(2_500_000);

impl Session {
    /// Four fleets in different states, for testing multi-fleet management:
    /// - "Home Guard": the starting freighter, docked at the home trade port.
    /// - "Scout Wing": a scout in orbit of the first planet with a market.
    /// - "Survey": a scout in orbit of a moon (no market there).
    /// - "Meridian Convoy": a patrol ship and freighter underway to system 2's port.
    pub fn multi_fleet_scenario(seed: u64) -> Result<Self, String> {
        let mut session = Self::with_seed(seed);
        session.economy.credits = TEST_BUDGET;
        let home = session.state.ships[0]
            .location
            .system()
            .ok_or("Starting ship must begin in a system")?;
        let planet = session.body(home, |kind| {
            kind == BodyKind::RockyPlanet || kind == BodyKind::GasGiant
        })?;
        let moon = session.body(home, |kind| kind == BodyKind::Moon)?;
        let scout = session.buy(ShipKind::Scout)?;
        let survey = session.buy(ShipKind::Scout)?;
        let patrol = session.buy(ShipKind::Patrol)?;
        let freighter = session.buy(ShipKind::Freighter)?;
        session.rename(ShipId(1), "Home Guard")?;
        for (ships, name) in [
            (vec![scout], "Scout Wing"),
            (vec![survey], "Survey"),
            (vec![patrol, freighter], "Meridian Convoy"),
        ] {
            session.split(&ships)?;
            session.rename(ships[0], name)?;
        }
        session.plot(scout, home, Some(planet))?;
        session.plot(survey, home, Some(moon))?;
        session.plot(patrol, SystemId(2), None)?;
        // Run until the local fleets arrive; the interstellar convoy stays underway.
        let chance = session.state.movement_rules.encounter_chance_percent;
        session.state.movement_rules.encounter_chance_percent = 0;
        for _ in 0..16 {
            let local_pending = [scout, survey].iter().any(|ship| {
                session
                    .state
                    .fleet_for_ship(*ship)
                    .is_ok_and(|fleet| session.state.navigation.order(fleet.id).is_some())
            });
            if !local_pending {
                break;
            }
            session.quiet_run()?;
        }
        session.state.movement_rules.encounter_chance_percent = chance;
        Ok(session)
    }

    /// Seven ships against seven raiders, with the battle open on the player's
    /// first turn:
    /// - "Strike Group": the starting freighter, two more freighters, two patrol
    ///   ships and two scouts, bound for the first planet;
    /// - raiders: two patrol ships and five scouts. Play caps raider groups at
    ///   three; this fixture sets the group directly.
    pub fn fleet_battle_scenario(seed: u64) -> Result<Self, String> {
        let mut session = Self::with_seed(seed);
        session.economy.credits = Credits(TEST_BUDGET.0 * 4);
        let yard = session
            .economy
            .current_market(&session.state.ships[0])
            .map_err(|error| error.to_string())?
            .id;
        // Fixture: enough hulls in stock for the whole strike group.
        let yard = session
            .economy
            .market_mut(yard)
            .map_err(|error| error.to_string())?;
        yard.ship_stock = yard.ship_stock.map(|stock| stock.max(4));
        for kind in [
            ShipKind::Freighter,
            ShipKind::Freighter,
            ShipKind::Patrol,
            ShipKind::Patrol,
            ShipKind::Scout,
            ShipKind::Scout,
        ] {
            session.buy(kind)?;
        }
        session.rename(ShipId(1), "Strike Group")?;
        let home = session.state.ships[0]
            .location
            .system()
            .ok_or("Starting ship must begin in a system")?;
        let planet = session.body(home, |kind| {
            kind == BodyKind::RockyPlanet || kind == BodyKind::GasGiant
        })?;
        session.plot(ShipId(1), home, Some(planet))?;
        let chance = session.state.movement_rules.encounter_chance_percent;
        session.set_encounter_chance(100);
        session.quiet_run()?;
        session.state.movement_rules.encounter_chance_percent = chance;
        let encounter = session
            .state
            .navigation
            .encounters
            .first_mut()
            .ok_or("The strike group met no contact")?;
        encounter.kind = EncounterKind::Raiders;
        encounter.hostiles = [ShipKind::Patrol; 2]
            .into_iter()
            .chain([ShipKind::Scout; 5])
            .collect();
        let encounter = encounter.id;
        session.run(ClientCommand::ResolveEncounter {
            encounter,
            choice: EncounterChoice::Fight,
            expected_revision: session.state.revision,
        })?;
        Ok(session)
    }

    /// Test hook: the per-run encounter chance in percent (clamped to 100) for
    /// every later roll, e.g. 0 for uneventful voyages or 100 for a contact.
    pub fn set_encounter_chance(&mut self, percent: u32) {
        self.state.movement_rules.encounter_chance_percent = percent.min(100);
    }

    fn body(&self, system: SystemId, kind: impl Fn(BodyKind) -> bool) -> Result<BodyId, String> {
        self.systems
            .iter()
            .find(|known| known.id == system)
            .and_then(|known| known.bodies.iter().find(|body| kind(body.kind)))
            .map(|body| body.id)
            .ok_or_else(|| "Home system lacks a body this scenario needs".into())
    }

    fn run(&mut self, command: ClientCommand) -> Result<(), String> {
        match self.handle(command) {
            ServerMessage::CommandRejected(error) => Err(error.to_string()),
            _ => Ok(()),
        }
    }

    fn buy(&mut self, kind: ShipKind) -> Result<ShipId, String> {
        self.run(ClientCommand::BuyShip {
            ship: ShipId(1),
            kind,
            expected_revision: self.state.revision,
        })?;
        self.state
            .ships
            .iter()
            .map(|ship| ship.id)
            .max_by_key(|id| id.0)
            .ok_or_else(|| "Purchase produced no ship".into())
    }

    fn fleet_of(&self, ship: ShipId) -> Result<FleetId, String> {
        self.state
            .fleet_for_ship(ship)
            .map(|fleet| fleet.id)
            .map_err(|error| error.to_string())
    }

    fn split(&mut self, ships: &[ShipId]) -> Result<(), String> {
        let fleet = self.fleet_of(ships[0])?;
        self.run(ClientCommand::SplitFleet {
            fleet,
            ships: ships.to_vec(),
            expected_revision: self.state.revision,
        })
    }

    fn rename(&mut self, ship: ShipId, name: &str) -> Result<(), String> {
        let fleet = self.fleet_of(ship)?;
        self.run(ClientCommand::RenameFleet {
            fleet,
            name: name.into(),
            expected_revision: self.state.revision,
        })
    }

    fn plot(&mut self, ship: ShipId, system: SystemId, body: Option<BodyId>) -> Result<(), String> {
        self.run(ClientCommand::PlotDestination {
            ship,
            system,
            body,
            expected_revision: self.state.revision,
        })
    }

    /// Hold every unassigned fleet, then execute one navigation run.
    fn quiet_run(&mut self) -> Result<(), String> {
        for fleet in self.state.fleets.clone() {
            if self.state.navigation.order(fleet.id).is_none()
                && !self.state.navigation.holds.contains(&fleet.id)
                && self.state.fleet_position(fleet.id, fleet.owner).is_ok()
            {
                self.run(ClientCommand::HoldFleet {
                    fleet: fleet.id,
                    expected_revision: self.state.revision,
                })?;
            }
        }
        self.run(ClientCommand::ExecuteNavigationRun {
            expected_revision: self.state.revision,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::travel::Location;

    #[test]
    fn multi_fleet_scenario_has_four_fleets_in_distinct_states_and_saves() {
        let session = Session::multi_fleet_scenario(1).unwrap();
        let names: Vec<_> = session
            .state
            .fleets
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        for name in ["Home Guard", "Scout Wing", "Survey", "Meridian Convoy"] {
            assert!(names.contains(&name), "{names:?}");
        }
        let location = |name: &str| {
            let fleet = session
                .state
                .fleets
                .iter()
                .find(|f| f.name == name)
                .unwrap();
            session.state.ship(fleet.ships[0]).unwrap().location.clone()
        };
        assert!(matches!(location("Home Guard"), Location::Docked(_)));
        assert!(matches!(location("Scout Wing"), Location::Orbiting { .. }));
        assert!(matches!(location("Survey"), Location::Orbiting { .. }));
        let convoy = session
            .state
            .fleets
            .iter()
            .find(|f| f.name == "Meridian Convoy")
            .unwrap();
        assert_eq!(convoy.ships.len(), 2);
        assert_ne!(location("Meridian Convoy"), location("Home Guard"));
        // Round-trips through the normal save format.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("multi-fleet.json");
        session.save(&path).unwrap();
        let (loaded, _) = Session::load(&path).unwrap();
        assert_eq!(loaded.state.fleets.len(), 4);
    }
}

#[cfg(test)]
mod fleet_battle_tests {
    use super::*;
    use game_core::{combat::Side, navigation::BattleAction};

    #[test]
    fn seven_against_seven_opens_on_a_player_turn_saves_and_fights_to_a_finish() {
        let mut session = Session::fleet_battle_scenario(1).unwrap();
        let open = session
            .state
            .navigation
            .battle
            .clone()
            .expect("battle open");
        let side = |side| {
            open.battle
                .units
                .iter()
                .filter(|unit| unit.side == side)
                .count()
        };
        assert_eq!((side(Side::Player), side(Side::Hostile)), (7, 7));
        let me = open.battle.active().unwrap();
        assert_eq!(open.battle.units[me].side, Side::Player);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fleet-battle.json");
        session.save(&path).unwrap();
        let (loaded, _) = Session::load(&path).unwrap();
        assert_eq!(loaded.state, session.state);
        session
            .run(ClientCommand::BattleAction {
                action: BattleAction::AutoResolve,
                expected_revision: session.state.revision,
            })
            .unwrap();
        let report = session.state.navigation.last_combat.clone().unwrap();
        assert!(session.state.navigation.battle.is_none());
        // Fought, not a ceasefire: ships were lost, whoever left the field.
        assert!(report.combatants.iter().any(|ship| ship.destroyed));
    }
}
