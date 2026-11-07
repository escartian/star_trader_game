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
            let encounters = self.state.navigation.encounters.clone();
            for encounter in encounters {
                self.command(ClientCommand::ResolveEncounter {
                    encounter: encounter.id,
                    choice: game_core::navigation::EncounterChoice::Bypass,
                    expected_revision: self.state.revision,
                });
            }
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
