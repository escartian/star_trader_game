//! Local shipyard presentation; authoritative rules enforce funds and inventory.
use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_protocol::ClientCommand;

impl LocalSession {
    pub(super) fn shipyard_ui(&self, ui: &mut egui::Ui) -> Option<ClientCommand> {
        let ship = self.selected_ship();
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Shipyard, GREEN, "Shipyard");
            ui.heading("Shipyard");
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                &format!(
                    "New hulls join {}'s fleet.\nSales require an empty hold. Fuel and hold upgrades are not refunded; hulls sell at catalog specifications.",
                    ship.name
                ),
            );
        });
        let market = match self.economy.current_market(ship) {
            Ok(market) => market,
            Err(error) => {
                ui.label(error.to_string());
                return None;
            }
        };
        stat(
            ui,
            Icon::Funds,
            AMBER,
            &market.credits.to_string(),
            "Shipyard funds available to buy hulls",
        );
        let artwork = crate::map::art::Artwork::preload(ui.ctx());
        for spec in &self.economy.rules.ship_catalog.ships {
            ui.horizontal_wrapped(|ui| {
                let stock = market.ship_stock[spec.kind.index()];
                let (rect, _) =
                    ui.allocate_exact_size(egui::Vec2::splat(36.0), egui::Sense::hover());
                artwork.paint(
                    crate::map::art::ArtKind::from(spec.kind),
                    ui.painter(),
                    rect.center(),
                    36.0,
                );
                ui.strong(spec.kind.label());
                stat(ui, Icon::Credits, AMBER, &spec.price.to_string(), "Price");
                stat(ui, Icon::Funds, CYAN, &stock.to_string(), "In stock");
                stat(
                    ui,
                    Icon::Cargo,
                    GREEN,
                    &spec.cargo_capacity.to_string(),
                    "Cargo hold",
                );
                stat(
                    ui,
                    Icon::Fuel,
                    CYAN,
                    &spec.fuel_capacity.to_string(),
                    "Fuel capacity",
                );
                stat(
                    ui,
                    Icon::Distance,
                    CYAN,
                    &format!("{} ly", spec.range_ly),
                    "Jump range",
                );
                let blocked = if stock == 0 {
                    Some("Out of stock".to_string())
                } else if self.economy.credits < spec.price {
                    Some("Not enough credits".to_string())
                } else if self.state.ships.len() >= 64 {
                    Some("Fleet limit of 64 ships reached".to_string())
                } else {
                    None
                };
                let tip = format!("Buy {} for {} cr", spec.kind.label(), spec.price);
                if icons::action(ui, Icon::Buy, &tip, blocked).clicked() {
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
            |price| format!("Sell {} · {} cr", ship.name, price),
        );
        let blocked = if self.state.ships.len() <= 1 {
            Some("Keep at least one owned ship.".to_string())
        } else {
            quote.as_ref().err().map(ToString::to_string)
        };
        let enabled = blocked.is_none();
        let button = ui
            .add_enabled_ui(enabled, |ui| icons::button(ui, Icon::Sell, &label))
            .inner;
        if button.clicked() {
            action = Some(ClientCommand::SellShip {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
        if let Some(reason) = blocked {
            button.on_disabled_hover_text(reason);
        }
        action
    }
}
