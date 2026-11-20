//! One fleet takes your direct orders: the flagship. Every other fleet runs a
//! standing order (Wait, a trade route or a patrol) and settles its own
//! contacts. This card section names the flagship, shows a secondary fleet's
//! order and offers Take command, Wait and Patrol.
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{CYAN, GREEN};
use egui::RichText;
use game_core::{StarSystem, fleet::Fleet};
use game_protocol::ClientCommand;
use game_server::LOCAL_PLAYER;

impl LocalSession {
    pub(super) fn is_flagship(&self, fleet: game_core::fleet::FleetId) -> bool {
        self.state.flagship(LOCAL_PLAYER) == Some(fleet)
    }

    /// The flagship's badge, or a secondary fleet's standing order and controls.
    pub(super) fn standing_orders(
        &mut self,
        ui: &mut egui::Ui,
        fleet: &Fleet,
        systems: &[StarSystem],
    ) {
        if self.is_flagship(fleet.id) {
            ui.label(
                RichText::new("FLAGSHIP · takes your direct orders")
                    .small()
                    .color(GREEN),
            );
            return;
        }
        let name = |id| {
            systems
                .iter()
                .find(|system| system.id == id)
                .map_or("an unknown system", |system| system.name.as_str())
        };
        let order = match (
            self.state.patrol(fleet.id),
            self.state.trade_route(fleet.id),
        ) {
            (Some(patrol), _) => format!("Patrolling {}", name(patrol.system)),
            (None, Some(_)) => "Running its trade route".to_owned(),
            (None, None) if self.state.navigation.order(fleet.id).is_some() => {
                "Finishing its course, then waiting".to_owned()
            }
            (None, None) => "Waiting".to_owned(),
        };
        ui.label(
            RichText::new(format!("Standing order: {order}"))
                .small()
                .color(CYAN),
        )
        .on_hover_text(
            "Secondary fleets run on their own: raider fights auto-resolve, salvage \
                 and distress calls are handled, and Advance never stops for them.",
        );
        let here = self
            .state
            .ship(*fleet.ships.first().unwrap_or(&self.active_ship))
            .ok()
            .and_then(|ship| ship.location.system());
        let mut command = None;
        ui.horizontal_wrapped(|ui| {
            if icons::button(ui, Icon::Focus, "Take command")
                .on_hover_text(
                    "Make this the flagship; the current flagship finishes its course, then waits.",
                )
                .clicked()
            {
                command = Some(ClientCommand::SetFlagship {
                    fleet: fleet.id,
                    expected_revision: self.state.revision,
                });
            }
            // Wait only stops something: a patrol, a route or a course.
            let waiting = self.state.patrol(fleet.id).is_none()
                && self.state.trade_route(fleet.id).is_none()
                && self.state.navigation.order(fleet.id).is_none();
            if !waiting
                && icons::button(ui, Icon::Hold, "Wait")
                    .on_hover_text("Stop the patrol or route and stay put")
                    .clicked()
            {
                command = Some(ClientCommand::HoldFleet {
                    fleet: fleet.id,
                    expected_revision: self.state.revision,
                });
            }
            let mut target = None;
            egui::ComboBox::from_id_salt(("patrol", fleet.id.0))
                .selected_text("Patrol…")
                .show_ui(ui, |ui| {
                    let mut choices: Vec<_> = systems.iter().collect();
                    // The fleet's own system first, then by name.
                    choices.sort_by_key(|system| (Some(system.id) != here, system.name.clone()));
                    for system in choices {
                        if ui.selectable_label(false, &system.name).clicked() {
                            target = Some(system.id);
                        }
                    }
                })
                .response
                .on_hover_text("Guard a system: tour its bodies and fight the raiders that come");
            if let Some(system) = target {
                command = Some(ClientCommand::SetPatrol {
                    fleet: fleet.id,
                    system,
                    expected_revision: self.state.revision,
                });
            }
        });
        if let Some(command) = command {
            self.command(command);
        }
    }
}
