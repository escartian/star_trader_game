use super::LocalSession;
use game_core::{SystemId, travel::Location};
use game_protocol::{ClientCommand, ServerMessage};

impl LocalSession {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        target: Option<SystemId>,
        systems: &[game_core::StarSystem],
    ) {
        let name = |id: SystemId| {
            systems
                .iter()
                .find(|s| s.id == id)
                .map_or("Unknown", |s| s.name.as_str())
        };
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
            ui.label(format!(
                "{} cr • cargo {}/{}",
                self.economy.credits,
                self.selected_ship().cargo_used(),
                self.selected_ship().cargo_capacity
            ));
            if ui.button("Market / refuel").clicked() {
                self.market.open = true;
            }
            ui.label(format!(
                "Fuel: {}/{} • tick {}",
                self.selected_ship().fuel,
                self.selected_ship().fuel_capacity,
                self.state.tick
            ));
            match &self.selected_ship().location {
                Location::Docked(system) => {
                    ui.label(format!("At {}", name(*system)));
                }
                Location::Travelling(journey) => {
                    ui.label(format!(
                        "{} -> {} • {} ticks remaining",
                        name(journey.origin),
                        name(journey.destination),
                        journey.arrival_tick.saturating_sub(self.state.tick)
                    ));
                }
            }
        });
        if matches!(self.selected_ship().location, Location::Travelling(_))
            && ui.button("Continue journey").clicked()
        {
            self.command(ClientCommand::ContinueTravel {
                ship: self.selected_ship().id,
                expected_revision: self.state.revision,
            });
            ui.ctx().request_repaint();
        }
        let target = target.filter(|destination| {
            matches!(self.selected_ship().location,
            Location::Docked(origin) if origin != *destination)
        });
        if let Some(destination) = target {
            match self.authority.handle(ClientCommand::PreviewTravel {
                ship: self.selected_ship().id,
                destination,
            }) {
                ServerMessage::TravelPreview(quote) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "To {}: {} fuel per ship (~{} cr each) • {} simulation ticks for the fleet",
                            name(destination),
                            quote.fuel,
                            game_core::economy::Credits(
                                u64::from(quote.fuel) * self.economy.rules.fuel_unit_price.0
                            ),
                            quote.duration_ticks
                        ));
                        if ui.button("Travel").clicked() {
                            let message = self.authority.handle_offline(ClientCommand::Travel {
                                ship: self.selected_ship().id,
                                destination,
                                expected_revision: quote.revision,
                            });
                            self.accept(message);
                            ui.ctx().request_repaint();
                        }
                    });
                }
                ServerMessage::CommandRejected(error) => {
                    ui.label(error.to_string());
                }
                _ => {}
            }
        }
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
    }
}
