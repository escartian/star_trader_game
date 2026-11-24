//! Reproducible capture/test fixtures. Built only for tests and `dev-tools` builds.
//! Each helper runs ordinary authoritative commands in a fresh, unsaved session.
use super::LocalSession;
use game_core::movement::MoveTarget;
use game_protocol::{ClientCommand, ServerMessage};

impl LocalSession {
    /// Hide the Navigation window so a test can prove a flow works without it.
    #[cfg(test)]
    pub fn close_navigation(&mut self) {
        self.navigation.open = false;
    }

    pub fn system_half_extent_km(&self) -> i64 {
        self.state.movement_rules.system_half_extent_km
    }

    pub fn open_saves(&mut self) {
        self.saves.open = true;
    }

    pub fn open_new_session(&mut self) {
        self.saves.new_open = true;
    }

    pub fn open_fleets(&mut self) {
        self.fleet_panel.open = true;
    }
}

impl LocalSession {
    pub fn capture_run_review(&mut self) {
        self.command(ClientCommand::BuyShip {
            ship: self.active_ship,
            kind: game_core::fleet::ShipKind::Scout,
            expected_revision: self.state.revision,
        });
        let second = self.active_ship;
        if let Ok(fleet) = self.state.fleet_for_ship(second) {
            self.command(ClientCommand::SplitFleet {
                fleet: fleet.id,
                ships: vec![second],
                expected_revision: self.state.revision,
            });
        }
        let first = self.state.ships[0].id;
        self.command(ClientCommand::Move {
            ship: first,
            target: game_core::movement::MoveTarget::Body {
                system: game_core::SystemId(1),
                body: game_core::BodyId(1),
            },
            expected_revision: self.state.revision,
        });
        self.request_execute_run();
    }

    /// Plots the selected fleet's route into system 2 and starts advancing along it.
    pub fn capture_advance(&mut self) -> Result<(), String> {
        self.command(ClientCommand::PlotDestination {
            ship: self.active_ship,
            system: game_core::SystemId(2),
            body: None,
            expected_revision: self.state.revision,
        });
        if self.state.navigation.orders.is_empty() {
            return Err(format!("Advance capture could not plot: {}", self.notice));
        }
        self.toggle_advance();
        Ok(())
    }

    /// Starts the selected fleet on a trade route to system 2's trade port.
    pub fn capture_trade_route(&mut self) -> Result<(), String> {
        let port = self
            .economy
            .markets
            .iter()
            .find(|market| market.id.system == game_core::SystemId(2) && market.is_station)
            .ok_or("Trade route capture needs a trade port in system 2")?
            .id;
        self.request_trade_route(port);
        if self.state.navigation.routes.is_empty() {
            return Err(format!("Trade route capture failed: {}", self.notice));
        }
        Ok(())
    }

    pub fn capture_encounter(&mut self) -> Result<(), String> {
        self.command(ClientCommand::Move {
            ship: self.active_ship,
            target: game_core::movement::MoveTarget::Body {
                system: game_core::SystemId(1),
                body: game_core::BodyId(1),
            },
            expected_revision: self.state.revision,
        });
        self.request_execute_run();
        if self.state.navigation.encounters.is_empty() {
            return Err("Use --new --seed 2 for the encounter capture".into());
        }
        self.navigation.open = true;
        Ok(())
    }
}

impl LocalSession {
    /// Advance just one run for a reproducible in-progress local approach capture.
    pub fn capture_navigation_step(&mut self, target: MoveTarget) -> Result<(), String> {
        for command in [
            ClientCommand::Move {
                ship: self.active_ship,
                target,
                expected_revision: self.state.revision,
            },
            ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision + 1,
            },
        ] {
            let response = self.authority.handle_offline(command);
            if let ServerMessage::CommandRejected(error) = &response {
                return Err(error.to_string());
            }
            self.accept(response);
        }
        Ok(())
    }

    /// Capture fixtures execute ordinary commands in a fresh, unsaved session.
    /// Flies into Meridian until its raider band hunts the fleet (or strikes).
    pub fn capture_hunted(&mut self) -> Result<(), String> {
        self.command(ClientCommand::PlotDestination {
            ship: self.active_ship,
            system: game_core::SystemId(2),
            body: Some(game_core::BodyId(2)),
            expected_revision: self.state.revision,
        });
        for _ in 0..40 {
            let hunted = self
                .state
                .navigation
                .bands
                .iter()
                .any(|band| band.hunting.is_some());
            let struck = self
                .state
                .navigation
                .encounters
                .iter()
                .any(|contact| contact.band.is_some());
            if hunted || struck {
                return Ok(());
            }
            self.capture_resolve_contacts();
            self.command(ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision,
            });
        }
        Err(format!("No band hunted the fleet: {}", self.notice))
    }

    /// Lands the active fleet, orders it to wait and executes `runs` runs.
    pub fn capture_landing(&mut self, runs: usize) -> Result<(), String> {
        let fleet = self
            .state
            .fleet_for_ship(self.active_ship)
            .map_err(|error| error.to_string())?
            .id;
        for command in [
            ClientCommand::Land {
                fleet,
                expected_revision: self.state.revision,
            },
            ClientCommand::HoldFleet {
                fleet,
                expected_revision: self.state.revision + 1,
            },
        ] {
            let response = self.authority.handle_offline(command);
            if let ServerMessage::CommandRejected(error) = &response {
                return Err(error.to_string());
            }
            self.accept(response);
        }
        self.capture_wait_runs(runs);
        Ok(())
    }

    pub fn capture_movement(&mut self, target: MoveTarget) -> Result<(), String> {
        let response = self.authority.handle_offline(ClientCommand::Move {
            ship: self.active_ship,
            target,
            expected_revision: self.state.revision,
        });
        if let ServerMessage::CommandRejected(error) = &response {
            return Err(error.to_string());
        }
        self.accept(response);
        self.capture_follow_courses()
    }

    /// Executes runs (fighting raiders, bypassing other contacts, holding idle
    /// fleets) until no course is left.
    fn capture_follow_courses(&mut self) -> Result<(), String> {
        for _ in 0..128 {
            if self.state.navigation.orders.is_empty() {
                return Ok(());
            }
            let fleets = self.state.fleets.clone();
            for fleet in fleets {
                if self.state.navigation.order(fleet.id).is_none() {
                    self.command(ClientCommand::HoldFleet {
                        fleet: fleet.id,
                        expected_revision: self.state.revision,
                    });
                }
            }
            self.capture_resolve_contacts();
            self.command(ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision,
            });
        }
        Err("Capture course did not finish within 128 runs".into())
    }
}

impl LocalSession {
    pub fn capture_wait_runs(&mut self, count: usize) {
        for _ in 0..count {
            for fleet in self.state.fleets.clone() {
                self.command(ClientCommand::HoldFleet {
                    fleet: fleet.id,
                    expected_revision: self.state.revision,
                });
            }
            self.request_execute_run();
        }
    }
}

impl LocalSession {
    /// Fight raiders and bypass every other contact so capture courses continue.
    pub(super) fn capture_resolve_contacts(&mut self) {
        use game_core::navigation::{EncounterChoice, EncounterKind};
        for encounter in self.state.navigation.encounters.clone() {
            let choice = match encounter.kind {
                EncounterKind::Raiders => EncounterChoice::AutoResolve,
                _ => EncounterChoice::Bypass,
            };
            self.command(ClientCommand::ResolveEncounter {
                encounter: encounter.id,
                choice,
                expected_revision: self.state.revision,
            });
        }
    }

    /// Chases NPC traders from market to market until the flagship can hail
    /// one, then hails it, so the trader's contact card shows.
    pub fn capture_hail(&mut self) -> Result<(), String> {
        use game_core::travel::Location;
        let player = game_server::LOCAL_PLAYER;
        for _ in 0..40 {
            let systems = self.authority.systems().to_vec();
            let reach = game_core::piracy::in_reach(&self.state, &self.economy, player, &systems);
            if let (Some(trader), Some(fleet)) = (reach.first(), self.state.flagship(player)) {
                self.command(ClientCommand::HailTrader {
                    fleet,
                    trader: *trader,
                    expected_revision: self.state.revision,
                });
                let hailed = self.state.navigation.encounters.iter();
                return match hailed.clone().any(|contact| contact.trader.is_some()) {
                    true => Ok(()),
                    false => Err(format!("The hail failed: {}", self.notice)),
                };
            }
            let parked =
                self.economy
                    .traders
                    .iter()
                    .find_map(|npc| match npc.state.ships[0].location {
                        Location::Orbiting { system, body } => Some((system, body)),
                        _ => None,
                    });
            let plotted = parked.is_some_and(|(system, body)| {
                let response = self
                    .authority
                    .handle_offline(ClientCommand::PlotDestination {
                        ship: self.active_ship,
                        system,
                        body: Some(body),
                        expected_revision: self.state.revision,
                    });
                let accepted = !matches!(response, ServerMessage::CommandRejected(_));
                self.accept(response);
                accepted
            });
            if plotted {
                self.capture_follow_courses()?;
            } else {
                self.capture_wait_runs(1);
            }
            self.capture_resolve_contacts();
        }
        Err("No trader came within hailing range in 40 tries".into())
    }

    /// Flies the active fleet to the Raider Haven's body; returns its system.
    pub fn capture_haven(&mut self) -> Result<game_core::SystemId, String> {
        let haven = self
            .state
            .navigation
            .haven
            .clone()
            .ok_or("No Raider Haven")?;
        let response = self
            .authority
            .handle_offline(ClientCommand::PlotDestination {
                ship: self.active_ship,
                system: haven.system,
                body: Some(haven.body),
                expected_revision: self.state.revision,
            });
        if let ServerMessage::CommandRejected(error) = &response {
            return Err(error.to_string());
        }
        self.accept(response);
        self.capture_follow_courses()?;
        // A band may strike on arrival; settle it so the Assault prompt shows.
        self.capture_resolve_contacts();
        self.combat_report = super::combat_report::CombatReportPanel::seen(&self.state);
        Ok(haven.system)
    }

    /// The guide card a new captain sees when the shipyard opens.
    pub fn capture_unlock(&mut self) {
        self.announce_unlock(game_core::progress::Feature::Fleets);
    }

    /// The Victory panel over the map.
    pub fn capture_victory(&mut self) {
        self.victory.open = true;
    }

    /// Lays out the four factions (sworn to the Federation), opening the
    /// panel unless only the map's territory rings are wanted.
    pub fn capture_factions(&mut self, panel: bool) -> Result<(), String> {
        let setup = game_core::factions::FactionSetup {
            allegiance: Some(game_core::factions::Faction::Federation),
            ..Default::default()
        };
        self.authority.set_up_factions(0, &setup)?;
        self.command(ClientCommand::GetSession);
        self.factions.open = panel;
        Ok(())
    }

    /// Shuttle between two bodies until the first raider fight, then show its report.
    pub fn capture_combat(&mut self) -> Result<(), String> {
        self.capture_raiders(false)
    }

    /// Shuttle until raiders appear, then fight them turn by turn.
    pub fn capture_battle(&mut self) -> Result<(), String> {
        self.capture_raiders(true)
    }

    /// The battle after the acting ship steps toward the raiders and aims at
    /// the nearest one in sight (when one is), centred on it and zoomed in.
    pub fn capture_battle_close(&mut self) -> Result<(), String> {
        use game_core::navigation::BattleAction;
        self.capture_raiders(true)?;
        let battle = &self
            .state
            .navigation
            .battle
            .as_ref()
            .ok_or("No battle")?
            .battle;
        let me = battle.active().ok_or("No acting ship")?;
        let toward = battle
            .units
            .iter()
            .find(|unit| unit.side != battle.units[me].side);
        let step = toward.and_then(|enemy| {
            battle
                .reachable()
                .into_iter()
                .min_by_key(|hex| (hex.distance(enemy.at), hex.q, hex.r))
        });
        if let Some(step) = step {
            self.battle_command(BattleAction::Move(step));
        }
        self.battle_close_up(2.0);
        let battle = &self
            .state
            .navigation
            .battle
            .as_ref()
            .ok_or("No battle")?
            .battle;
        let targets = battle.targets(me, &self.economy.rules.combat_catalog);
        // Sync first: a turn the view has not seen yet would clear the aim.
        self.battle.view.sync(battle);
        self.battle.view.aimed = targets.first().copied();
        Ok(())
    }

    fn capture_raiders(&mut self, open_battle: bool) -> Result<(), String> {
        use game_core::navigation::{EncounterChoice, EncounterKind};
        if open_battle && self.state.navigation.battle.is_some() {
            return Ok(());
        }
        for leg in 0..400_u64 {
            if open_battle
                && let Some(encounter) = self
                    .state
                    .navigation
                    .encounters
                    .iter()
                    .find(|encounter| encounter.kind == EncounterKind::Raiders)
                    .cloned()
            {
                self.command(ClientCommand::ResolveEncounter {
                    encounter: encounter.id,
                    choice: EncounterChoice::Fight,
                    expected_revision: self.state.revision,
                });
                return match self.state.navigation.battle {
                    Some(_) => Ok(()),
                    None => Err(format!("The battle did not open: {}", self.notice)),
                };
            }
            if self.state.navigation.last_combat.is_some() {
                self.combat_report.open = true;
                return Ok(());
            }
            let fleet = self
                .state
                .fleet_for_ship(self.active_ship)
                .map_err(|e| e.to_string())?
                .id;
            if self.state.navigation.order(fleet).is_none() {
                let ship = self.selected_ship();
                let missing = ship.fuel_capacity - ship.fuel;
                if missing > 0 && self.economy.current_market(ship).is_ok() {
                    self.command(ClientCommand::Refuel {
                        ship: self.active_ship,
                        units: missing,
                        expected_revision: self.state.revision,
                    });
                }
                self.command(ClientCommand::PlotDestination {
                    ship: self.active_ship,
                    system: game_core::SystemId(1),
                    // Station (3) and inner world (1) both have markets for fuel.
                    body: Some(game_core::BodyId(if leg % 2 == 0 { 1 } else { 3 })),
                    expected_revision: self.state.revision,
                });
            }
            self.capture_resolve_contacts();
            self.command(ClientCommand::ExecuteNavigationRun {
                expected_revision: self.state.revision,
            });
        }
        Err(format!(
            "No raider contact within 400 runs: {}",
            self.notice
        ))
    }
}
