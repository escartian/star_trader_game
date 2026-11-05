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
            if self.saves.paused {
                ui.strong("PAUSED");
            }
            if ui.button("Session / saves").clicked() {
                self.saves.open = true;
            }
            ui.label(format!(
                "{} cr • cargo {}/{}",
                self.economy.credits,
                self.economy.cargo_used(),
                self.economy.cargo_capacity
            ));
            if ui.button("Market / refuel").clicked() {
                self.market.open = true;
            }
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
                        "{} -> {} • {} ticks remaining",
                        name(journey.origin),
                        name(journey.destination),
                        journey.arrival_tick.saturating_sub(self.state.tick)
                    ));
                }
            }
        });
        let target = target.filter(|destination| {
            matches!(self.state.ship.location,
            Location::Docked(origin) if origin != *destination)
        });
        if let Some(destination) = target {
            match self.authority.handle(ClientCommand::PreviewTravel {
                ship: self.state.ship.id,
                destination,
            }) {
                ServerMessage::TravelPreview(quote) => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "To {}: {} fuel (~{} cr) • {} ticks (~{:.1} seconds at {}x)",
                            name(destination),
                            quote.fuel,
                            game_core::economy::Credits(
                                u64::from(quote.fuel) * self.economy.rules.fuel_unit_price.0
                            ),
                            quote.duration_ticks,
                            quote.duration_ticks as f64 / f64::from(self.saves.speed),
                            self.saves.speed
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
