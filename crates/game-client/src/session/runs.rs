//! Fleet readiness, Civ-style attention and fleet-specific encounter choices.
use super::{LocalSession, navigation::location_label};
use game_core::{
    StarSystem,
    economy::Commodity,
    navigation::{EncounterChoice, EncounterKind, NavigationEvent},
};
use game_protocol::ClientCommand;

pub(super) fn event_label(event: NavigationEvent) -> String {
    match event {
        NavigationEvent::CoursePlotted(fleet) => format!(
            "Course plotted for Fleet {}. No time has advanced.",
            fleet.0
        ),
        NavigationEvent::CourseCancelled(fleet) => format!("Cancelled Fleet {}'s course.", fleet.0),
        NavigationEvent::Holding(fleet) => {
            format!("Fleet {} will hold position for this run.", fleet.0)
        }
        NavigationEvent::Arrived(plan) => {
            format!("Fleet {} arrived: {}.", plan.fleet.0, plan.mode.label())
        }
        NavigationEvent::RunExecuted(run) => format!("Navigation run {run} complete."),
        NavigationEvent::EncounterFound(encounter) => format!(
            "Fleet {}: {}. Review the contact in Navigation.",
            encounter.fleet.0,
            encounter.kind.label()
        ),
        NavigationEvent::EncounterResolved { fleet, outcome } => {
            format!("Fleet {}: {outcome}", fleet.0)
        }
    }
}

impl LocalSession {
    pub fn take_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.focus_requested)
    }

    pub fn request_execute_run(&mut self) {
        if let Some(fleet) = self.state.navigation_attention(game_server::LOCAL_PLAYER) {
            if let Some(ship) = self
                .state
                .fleets
                .iter()
                .find(|record| record.id == fleet)
                .and_then(|record| record.ships.first())
                .copied()
            {
                self.select_ship(ship);
            }
            self.focus_requested = true;
            self.navigation.open = true;
            self.navigation.target = None;
            let reason = if self
                .state
                .navigation
                .encounters
                .iter()
                .any(|encounter| encounter.fleet == fleet)
            {
                "Resolve this fleet's encounter first."
            } else {
                "This fleet needs a course or an explicit Hold position order."
            };
            self.record(reason.into());
            return;
        }
        self.command(ClientCommand::ExecuteNavigationRun {
            expected_revision: self.state.revision,
        });
    }

    pub(super) fn run_review(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        ui.heading(format!(
            "Navigation run {}",
            self.state.navigation.run.saturating_add(1)
        ));
        ui.label("Fleets move in parallel. Longer courses continue across runs; encounters can delay a fleet.");
        ui.small(format!(
            "Rescues recorded: {}",
            self.state.navigation.rescues
        ));
        let mut action = None;
        for fleet in self.state.fleets.clone() {
            let order = self.state.navigation.order(fleet.id).cloned();
            let held = self.state.navigation.holds.contains(&fleet.id);
            let encounter = self
                .state
                .navigation
                .encounters
                .iter()
                .any(|encounter| encounter.fleet == fleet.id);
            let selected = fleet.ships.contains(&self.active_ship);
            ui.horizontal_wrapped(|ui| {
                if ui.selectable_label(selected, &fleet.name).clicked()
                    && let Some(ship) = fleet.ships.first()
                {
                    self.select_ship(*ship);
                    self.focus_requested = true;
                }
                if encounter {
                    ui.colored_label(egui::Color32::YELLOW, "Encounter needs a decision");
                } else if let Some(order) = &order {
                    ui.label(format!(
                        "{} • {} runs remaining",
                        if order.started {
                            "Underway"
                        } else {
                            "Course plotted"
                        },
                        order
                            .remaining_ticks
                            .div_ceil(self.state.movement_rules.ticks_per_navigation_run)
                    ));
                } else if held {
                    ui.label("Holding this run");
                } else if fleet.ships.iter().any(|id| {
                    self.state
                        .ship(*id)
                        .is_ok_and(|ship| !ship.location.stationary())
                }) {
                    ui.label("Saved journey underway");
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "Needs orders");
                }
                if order.as_ref().is_none_or(|order| !order.started)
                    && self.state.fleet_position(fleet.id, fleet.owner).is_ok()
                {
                    if !held && ui.button("Hold position").clicked() {
                        action = Some(ClientCommand::HoldFleet {
                            fleet: fleet.id,
                            expected_revision: self.state.revision,
                        });
                    }
                    if order.is_some() && ui.button("Cancel course").clicked() {
                        action = Some(ClientCommand::CancelCourse {
                            fleet: fleet.id,
                            expected_revision: self.state.revision,
                        });
                    }
                }
            });
            if let Some(order) = &order {
                ui.small(format!(
                    "Destination: {}",
                    location_label(&order.plan.destination, systems)
                ));
            }
        }
        if ui.button("Execute navigation run").clicked() {
            self.request_execute_run();
        }
        if let Some(command) = action {
            self.command(command);
        }
        self.encounter_ui(ui);
        ui.separator();
    }

    fn encounter_ui(&mut self, ui: &mut egui::Ui) {
        let Ok(fleet) = self.state.fleet_for_ship(self.active_ship).cloned() else {
            return;
        };
        let Some(encounter) = self
            .state
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.fleet == fleet.id)
            .cloned()
        else {
            return;
        };
        ui.group(|ui| {
            ui.strong(encounter.kind.label());
            let mut choice = None;
            match encounter.kind {
                EncounterKind::DebrisField => {
                    ui.label("Your fleet detects recoverable debris. Investigate to collect one ore, or bypass it.");
                    let room = fleet.ships.iter().any(|id| self.state.ship(*id).is_ok_and(|ship| ship.cargo_used() < u64::from(ship.cargo_capacity)));
                    if ui.add_enabled(room, egui::Button::new("Recover salvage")).clicked() { choice = Some(EncounterChoice::Investigate); }
                    if !room { ui.label("One cargo space required."); }
                }
                EncounterKind::DistressSignal => {
                    ui.label("A stranded crew requests one medicine. Assist to record a rescue, or continue past the signal.");
                    let medicine = fleet.ships.iter().any(|id| self.state.ship(*id).is_ok_and(|ship| ship.cargo[Commodity::Medicine.index()] > 0));
                    if ui.add_enabled(medicine, egui::Button::new("Assist (1 medicine)")).clicked() { choice = Some(EncounterChoice::Assist); }
                    if !medicine { ui.label("No medicine aboard this fleet."); }
                }
            }
            if ui.button("Bypass contact").clicked() { choice = Some(EncounterChoice::Bypass); }
            if let Some(choice) = choice { self.command(ClientCommand::ResolveEncounter { encounter: encounter.id, choice, expected_revision: self.state.revision }); }
        });
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
