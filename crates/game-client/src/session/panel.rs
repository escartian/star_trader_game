use super::{LocalSession, navigation::location_label};
use game_core::{SystemId, travel::Location};
use game_protocol::ClientCommand;

impl LocalSession {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        target: Option<SystemId>,
        systems: &[game_core::StarSystem],
    ) {
        ui.horizontal_wrapped(|ui| {
            let ships = self.state.ships.clone();
            egui::ComboBox::from_id_salt("active_ship")
                .selected_text(&self.selected_ship().name)
                .show_ui(ui, |ui| {
                    for ship in ships {
                        if ui
                            .selectable_label(ship.id == self.active_ship, &ship.name)
                            .clicked()
                        {
                            self.select_ship(ship.id);
                        }
                    }
                });
            if ui.button("Fleets / shipyard").clicked() {
                self.fleet_panel.open = true;
            }
            if ui.button("Session / saves").clicked() {
                self.saves.open = true;
            }
            if ui.button("Navigation").clicked() {
                self.open_navigation();
            }
            if ui.button("Market / refuel").clicked() {
                self.market.open = true;
            }
            ui.label(format!(
                "{} cr • cargo {}/{} • fuel {}/{} • tick {}",
                self.economy.credits,
                self.selected_ship().cargo_used(),
                self.selected_ship().cargo_capacity,
                self.selected_ship().fuel,
                self.selected_ship().fuel_capacity,
                self.state.tick
            ));
            ui.label(location_label(&self.selected_ship().location, systems));
        });
        if matches!(self.selected_ship().location, Location::Travelling(_))
            && ui.button("Continue journey").clicked()
        {
            self.command(ClientCommand::ContinueTravel {
                ship: self.active_ship,
                expected_revision: self.state.revision,
            });
        }
        if matches!(self.selected_ship().location, Location::DeepSpace(_))
            && let Some(system) = target
            && ui.button("Preview entry into selected system").clicked()
        {
            self.request_movement(game_core::movement::MoveTarget::EnterSystem(system));
        }
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
    }
}
