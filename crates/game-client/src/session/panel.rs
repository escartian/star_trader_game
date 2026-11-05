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
            ui.strong("Your ship");
            ui.label(format!(
                "Fuel: {}/{} • tick {}",
                self.state.ship.fuel, self.state.ship.fuel_capacity, self.state.tick
            ));
            match &self.state.ship.location {
                Location::Docked(system) => {
                    ui.label(format!("At {}", name(*system)));
                }
                Location::Travelling(journey) => {
                    ui.label(format!(
                        "{} → {} • {} ticks remaining",
                        name(journey.origin),
                        name(journey.destination),
                        journey.arrival_tick.saturating_sub(self.state.tick)
                    ));
                }
            }
        });
        if let Some(destination) = target {
            match self.authority.handle(ClientCommand::PreviewTravel {
                ship: self.state.ship.id,
                destination,
            }) {
                ServerMessage::TravelPreview(quote) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "To {}: {} fuel • {} ticks (~{} seconds)",
                            name(destination),
                            quote.fuel,
                            quote.duration_ticks,
                            quote.duration_ticks
                        ));
                        if ui.button("Travel").clicked() {
                            let message = self.authority.handle(ClientCommand::Travel {
                                ship: self.state.ship.id,
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
