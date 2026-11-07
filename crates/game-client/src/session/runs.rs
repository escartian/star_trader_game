//! Fleet readiness, Civ-style attention and fleet-specific encounter choices.
use super::{
    LocalSession,
    navigation::{location_chip, location_label},
};
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
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
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Execute, GREEN, "Navigation run");
            ui.heading(format!("Run {}", self.state.navigation.run.saturating_add(1)));
            stat(
                ui,
                Icon::Good(Commodity::Medicine),
                CYAN,
                &self.state.navigation.rescues.to_string(),
                "Rescues recorded",
            );
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                "Fleets move in parallel. Longer courses continue across runs; encounters can delay a fleet.",
            );
        });
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
                let (icon, color, status) = if encounter {
                    (Icon::Contact, AMBER, "Contact: decision needed".to_string())
                } else if let Some(order) = &order {
                    let runs = order
                        .remaining_ticks
                        .div_ceil(self.state.movement_rules.ticks_per_navigation_run);
                    let state = if order.started {
                        Icon::Underway
                    } else {
                        Icon::Plot
                    };
                    (state, GREEN, format!("{runs}"))
                } else if held {
                    (Icon::Hold, CYAN, String::new())
                } else if fleet.ships.iter().any(|id| {
                    self.state
                        .ship(*id)
                        .is_ok_and(|ship| !ship.location.stationary())
                }) {
                    (Icon::Underway, GREEN, String::new())
                } else {
                    (Icon::Warning, AMBER, "Needs orders".to_string())
                };
                let tip = match icon {
                    Icon::Underway if order.is_some() => "Underway • runs remaining",
                    Icon::Plot => "Course plotted • runs remaining",
                    Icon::Hold => "Holding this run",
                    Icon::Underway => "Saved journey underway",
                    Icon::Contact => "Encounter needs a decision",
                    _ => "Plot a course or Hold position",
                };
                stat(ui, icon, color, &status, tip);
                if let Some(order) = &order {
                    let (place, name) = location_chip(&order.plan.destination, systems);
                    stat(
                        ui,
                        place,
                        CYAN,
                        &name,
                        &format!(
                            "Destination: {}",
                            location_label(&order.plan.destination, systems)
                        ),
                    );
                }
                if order.as_ref().is_none_or(|order| !order.started)
                    && self.state.fleet_position(fleet.id, fleet.owner).is_ok()
                {
                    if !held
                        && icons::icon_button(ui, Icon::Hold, "Hold position this run").clicked()
                    {
                        action = Some(ClientCommand::HoldFleet {
                            fleet: fleet.id,
                            expected_revision: self.state.revision,
                        });
                    }
                    if order.is_some()
                        && icons::icon_button(ui, Icon::Cancel, "Cancel course").clicked()
                    {
                        action = Some(ClientCommand::CancelCourse {
                            fleet: fleet.id,
                            expected_revision: self.state.revision,
                        });
                    }
                }
            });
        }
        if icons::button(ui, Icon::Execute, "Execute run").clicked() {
            self.request_execute_run();
        }
        if let Some(command) = action {
            self.command(command);
        }
        self.encounter_ui(ui);
        ui.separator();
    }

    /// Fleets paused by an unresolved contact.
    pub fn contact_fleets(&self) -> Vec<game_core::fleet::FleetId> {
        self.state
            .navigation
            .encounters
            .iter()
            .map(|encounter| encounter.fleet)
            .collect()
    }

    /// Decision panel shown beside Execute run while the selected fleet is paused
    /// by a contact, so the choice never hides inside the Navigation window.
    pub(super) fn contact_panel(&mut self, ui: &mut egui::Ui) {
        let Ok(fleet) = self.state.fleet_for_ship(self.active_ship).cloned() else {
            return;
        };
        if !self.contact_fleets().contains(&fleet.id) {
            return;
        }
        ui.separator();
        ui.horizontal(|ui| {
            icons::glyph(ui, Icon::Warning, AMBER, "Decision needed");
            ui.colored_label(AMBER, format!("CONTACT · {}", fleet.name));
        });
        ui.small("Paused until you decide; its course continues on the next run.");
        self.encounter_ui(ui);
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
            ui.horizontal(|ui| {
                icons::glyph(ui, Icon::Contact, AMBER, "Contact");
                ui.strong(encounter.kind.label());
            });
            let mut choice = None;
            let carries = |test: &dyn Fn(&game_core::travel::Ship) -> bool| {
                fleet
                    .ships
                    .iter()
                    .any(|id| self.state.ship(*id).is_ok_and(test))
            };
            let (text, icon, label, option, blocked) = match encounter.kind {
                EncounterKind::DebrisField => (
                    "Your fleet detects recoverable debris. Investigate to collect one ore, or bypass it.",
                    Icon::Good(Commodity::Ore),
                    "Salvage +1 ore",
                    EncounterChoice::Investigate,
                    (!carries(&|ship| ship.cargo_used() < u64::from(ship.cargo_capacity)))
                        .then(|| "One cargo space required.".to_string()),
                ),
                EncounterKind::DistressSignal => (
                    "A stranded crew requests one medicine. Assist to record a rescue, or continue past the signal.",
                    Icon::Good(Commodity::Medicine),
                    "Assist −1 medicine",
                    EncounterChoice::Assist,
                    (!carries(&|ship| ship.cargo[Commodity::Medicine.index()] > 0))
                        .then(|| "No medicine aboard this fleet.".to_string()),
                ),
            };
            ui.label(text);
            ui.horizontal(|ui| {
                if icons::labeled_action(ui, icon, label, blocked).clicked() {
                    choice = Some(option);
                }
                if icons::button(ui, Icon::Bypass, "Bypass").clicked() {
                    choice = Some(EncounterChoice::Bypass);
                }
            });
            if let Some(choice) = choice { self.command(ClientCommand::ResolveEncounter { encounter: encounter.id, choice, expected_revision: self.state.revision }); }
        });
    }
}
