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
            format!(
                "Fleet {} will wait here until you give it a new course.",
                fleet.0
            )
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
        NavigationEvent::BattleTurn { fleet } => {
            format!("Fleet {}: your ship is up in the battle.", fleet.0)
        }
        NavigationEvent::CourseBlocked(fleet) => format!(
            "Fleet {}: a moving body blocks the rest of the course. Plot a new one.",
            fleet.0
        ),
        NavigationEvent::Landed { fleet, .. } => format!("Fleet {} landed.", fleet.0),
        NavigationEvent::LiftedOff { fleet, .. } => {
            format!("Fleet {} lifted off into orbit.", fleet.0)
        }
        NavigationEvent::Mined { fleet, ore: 0, .. } => {
            format!("Fleet {}: holds full, mining stopped.", fleet.0)
        }
        NavigationEvent::Mined { fleet, ore, .. } => format!("Fleet {} mined {ore} ore.", fleet.0),
        NavigationEvent::RouteServiced { fleet, summary } => {
            format!("Fleet {} route · {summary}", fleet.0)
        }
        NavigationEvent::RoutePaused { fleet, reason } => {
            format!("Fleet {} route · {reason}", fleet.0)
        }
        NavigationEvent::RouteEnded { fleet, reason } => format!("Fleet {}: {reason}", fleet.0),
        NavigationEvent::PatrolNote { fleet, note } => format!("Fleet {} patrol · {note}", fleet.0),
        NavigationEvent::PoliceNote(note) => note,
        NavigationEvent::FlagshipChanged(fleet) => format!(
            "Fleet {} is now your flagship. Your other fleets follow their standing orders.",
            fleet.0
        ),
    }
}

impl LocalSession {
    /// The kind of the selected fleet's pending contact (tests).
    #[cfg(test)]
    pub fn pending_contact_kind(&self) -> Option<EncounterKind> {
        let fleet = self.state.fleet_for_ship(self.active_ship).ok()?.id;
        self.state
            .navigation
            .encounters
            .iter()
            .find(|encounter| encounter.fleet == fleet)
            .map(|encounter| encounter.kind)
    }

    pub fn take_focus_request(&mut self) -> bool {
        std::mem::take(&mut self.focus_requested)
    }

    pub fn request_execute_run(&mut self) {
        if let Some(fleet) = self.state.navigation_attention(game_server::LOCAL_PLAYER) {
            self.focus_fleet(fleet);
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
                "This fleet needs a course or an explicit Wait order."
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
                    (Icon::Hold, CYAN, "Waiting".to_string())
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
                    Icon::Hold => "Waiting until you plot a new course",
                    Icon::Underway => "Saved journey underway",
                    Icon::Contact => "Encounter needs a decision",
                    _ => "Plot a course or order this fleet to Wait",
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
                        && icons::icon_button(ui, Icon::Hold, "Wait here until new orders")
                            .clicked()
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
        ui.horizontal(|ui| {
            self.run_controls(ui);
            if self.state.navigation.last_combat.is_some()
                && icons::icon_button(ui, Icon::Weapon, "Open the latest combat report").clicked()
            {
                self.open_combat_report();
            }
        });
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
                ui.strong(if encounter.haven {
                    "Raider Haven assault"
                } else {
                    encounter.kind.label()
                });
            });
            if encounter.kind == EncounterKind::Raiders {
                self.raider_ui(ui, &fleet, &encounter);
                return;
            }
            if encounter.kind == EncounterKind::Trader {
                self.trader_ui(ui, &encounter);
                return;
            }
            if encounter.kind == EncounterKind::Police {
                self.police_ui(ui, &fleet, &encounter);
                return;
            }
            let mut choice = None;
            let carries = |test: &dyn Fn(&game_core::travel::Ship) -> bool| {
                fleet
                    .ships
                    .iter()
                    .any(|id| self.state.ship(*id).is_ok_and(test))
            };
            let offer = super::contacts::offer(
                encounter.kind,
                &super::contacts::Aboard {
                    free_space: carries(&|ship| ship.cargo_used() < u64::from(ship.cargo_capacity)),
                    medicine: carries(&|ship| ship.cargo[Commodity::Medicine.index()] > 0),
                    cargo: carries(&|ship| ship.cargo_used() > 0),
                },
            );
            ui.label(offer.text);
            ui.horizontal(|ui| {
                let blocked = offer.blocked.map(str::to_owned);
                if icons::labeled_action(ui, offer.icon, offer.act, blocked).clicked() {
                    choice = Some(offer.choice);
                }
                if icons::button(ui, Icon::Bypass, offer.pass).clicked() {
                    choice = Some(EncounterChoice::Bypass);
                }
            });
            if let Some(choice) = choice {
                self.command(ClientCommand::ResolveEncounter {
                    encounter: encounter.id,
                    choice,
                    expected_revision: self.state.revision,
                });
            }
        });
    }

    fn raider_ui(
        &mut self,
        ui: &mut egui::Ui,
        fleet: &game_core::fleet::Fleet,
        encounter: &game_core::navigation::Encounter,
    ) {
        // Afterburners on every ship raise the odds.
        let chance = self
            .state
            .evade_chance(fleet, &encounter.hostiles, &self.economy.rules)
            .unwrap_or(0);
        let names: Vec<_> = encounter.hostiles.iter().map(|kind| kind.label()).collect();
        if self
            .state
            .navigation
            .battle
            .as_ref()
            .is_some_and(|open| open.encounter == encounter.id)
        {
            ui.label(format!("Battle in progress against {}.", names.join(", ")));
            return;
        }
        ui.label(if encounter.haven {
            format!(
                "The Haven's garrison stands ready: {}. Fight it turn by turn, let it play out automatically, or call the assault off.",
                names.join(", ")
            )
        } else {
            format!(
                "Hostile raiders close in: {}. Fight them turn by turn, let the fight play out automatically, or try to outrun them; a failed escape starts the battle.",
                names.join(", ")
            )
        });
        let mut choice = None;
        // A captain's first raider contact offers Auto-resolve first.
        let first_fight = self.unlock_card_open(game_core::progress::Feature::Battles);
        ui.horizontal(|ui| {
            let auto = |ui: &mut egui::Ui| {
                icons::button(ui, Icon::Execute, "Auto-resolve")
                    .on_hover_text("Fight the whole battle at once with the AI on both sides.")
                    .clicked()
            };
            if first_fight && auto(ui) {
                choice = Some(EncounterChoice::AutoResolve);
            }
            if icons::button(ui, Icon::Weapon, "Fight")
                .on_hover_text("Command your ships turn by turn on the battle board.")
                .clicked()
            {
                choice = Some(EncounterChoice::Fight);
            }
            if !first_fight && auto(ui) {
                choice = Some(EncounterChoice::AutoResolve);
            }
            let evade = if encounter.haven {
                "Call off".to_owned()
            } else {
                format!("Evade {chance}%")
            };
            if icons::button(ui, Icon::Engine, &evade).clicked() {
                choice = Some(EncounterChoice::Evade);
            }
        });
        if let Some(choice) = choice {
            self.command(ClientCommand::ResolveEncounter {
                encounter: encounter.id,
                choice,
                expected_revision: self.state.revision,
            });
        }
    }
}
