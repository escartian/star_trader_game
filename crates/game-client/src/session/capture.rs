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
    fn capture_resolve_contacts(&mut self) {
        use game_core::navigation::{EncounterChoice, EncounterKind};
        for encounter in self.state.navigation.encounters.clone() {
            let choice = match encounter.kind {
                EncounterKind::Raiders => EncounterChoice::Fight,
                _ => EncounterChoice::Bypass,
            };
            self.command(ClientCommand::ResolveEncounter {
                encounter: encounter.id,
                choice,
                expected_revision: self.state.revision,
            });
        }
    }

    /// Shuttle between two bodies until the first raider fight, then show its report.
    pub fn capture_combat(&mut self) -> Result<(), String> {
        for leg in 0..400_u64 {
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
