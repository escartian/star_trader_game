//! Local shipyard presentation; authoritative rules enforce funds and inventory.
use super::LocalSession;
use game_protocol::ClientCommand;

impl LocalSession {
    pub(super) fn shipyard_ui(&self, ui: &mut egui::Ui) -> Option<ClientCommand> {
        let ship = self.selected_ship();
        let mut action = None;
        ui.heading(format!("Shipyard • join {}'s fleet", ship.name));
        let market = match self.economy.current_market(ship) {
            Ok(market) => market,
            Err(error) => {
                ui.label(error.to_string());
                return None;
            }
        };
        ui.label(format!("Local shipyard funds: {} cr", market.credits));
        for spec in &self.economy.rules.ship_catalog.ships {
            ui.horizontal_wrapped(|ui| {
                let stock = market.ship_stock[spec.kind.index()];
                ui.label(format!(
                    "{}: {} cr • {} in stock • hold {} • fuel {} • range {} ly",
                    spec.kind.label(),
                    spec.price,
                    stock,
                    spec.cargo_capacity,
                    spec.fuel_capacity,
                    spec.range_ly
                ));
                if ui
                    .add_enabled(
                        stock > 0
                            && self.economy.credits >= spec.price
                            && self.state.ships.len() < 64,
                        egui::Button::new("Buy"),
                    )
                    .clicked()
                {
                    action = Some(ClientCommand::BuyShip {
                        ship: ship.id,
                        kind: spec.kind,
                        expected_revision: self.state.revision,
                    });
                }
            });
        }
        let quote = self.economy.quote_ship_sale(ship);
        let label = quote.as_ref().map_or_else(
            |_| format!("Sell {}", ship.name),
            |price| format!("Sell {} for {} cr", ship.name, price),
        );
        let button = ui.add_enabled(
            quote.is_ok() && self.state.ships.len() > 1,
            egui::Button::new(label),
        );
        if button.clicked() {
            action = Some(ClientCommand::SellShip {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
        if self.state.ships.len() <= 1 {
            button.on_disabled_hover_text("Keep at least one owned ship.");
        } else if let Err(error) = quote {
            button.on_disabled_hover_text(error.to_string());
        }
        ui.label("Sales require an empty hold. Fuel and hold upgrades are not separately refunded; returned hulls are sold at catalog specifications.");
        action
    }
}
