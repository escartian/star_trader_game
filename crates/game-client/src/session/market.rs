use super::LocalSession;
use game_core::{
    StarSystem,
    economy::{Commodity, TradeSide},
};
use game_protocol::ClientCommand;

pub(super) struct MarketPanel {
    pub open: bool,
    quantity: u32,
    fuel_units: u32,
}
impl Default for MarketPanel {
    fn default() -> Self {
        Self {
            open: false,
            quantity: 1,
            fuel_units: 1,
        }
    }
}

impl LocalSession {
    pub fn show_market(&mut self, ctx: &egui::Context, systems: &[StarSystem]) {
        let mut open = self.market.open;
        egui::Window::new("Local resource market")
            .open(&mut open)
            .vscroll(true)
            .default_width(700.0)
            .show(ctx, |ui| self.market_ui(ui, systems));
        self.market.open = open;
    }

    fn market_ui(&mut self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        let ship = self.selected_ship().clone();
        let market = match self.economy.current_market(&ship) {
            Ok(market) => market.clone(),
            Err(error) => {
                ui.label(error.to_string());
                return;
            }
        };
        let body = systems
            .iter()
            .find(|system| system.id == market.id.system)
            .and_then(|system| system.bodies.iter().find(|body| body.id == market.id.body));
        ui.heading(body.map_or("Unknown market", |body| body.name.as_str()));
        ui.label(format!("Local market funds: {} cr", market.credits));
        if let Some(profile) = body.and_then(|body| body.planet.as_ref()) {
            ui.label(format!(
                "{} • {} • danger: {} • specializes in {}",
                profile.economy.label(),
                profile.biome.label(),
                profile.danger.label(),
                profile.specialization.label()
            ));
        }
        ui.label(format!(
            "Credits: {} • cargo: {}/{}",
            self.economy.credits,
            ship.cargo_used(),
            ship.cargo_capacity
        ));
        ui.horizontal(|ui| {
            ui.label("Order quantity");
            ui.add(egui::DragValue::new(&mut self.market.quantity).range(1..=2000));
        });
        ui.label("Prices in credits. Buy/u and Sell/u show first-unit prices; totals price every unit along the stock curve.");
        ui.label(format!(
            "Quote revision {} • simulation tick {}",
            self.state.revision, self.state.tick
        ));
        let mut action = None;
        egui::ScrollArea::horizontal()
            .id_salt("trade_table")
            .show(ui, |ui| {
                egui::Grid::new("live_market").striped(true).show(ui, |ui| {
                    for label in [
                        "Cargo",
                        "Stock",
                        "Held",
                        "Buy/u",
                        "Sell/u",
                        "Buy total",
                        "Sell total",
                        "Order",
                    ] {
                        ui.strong(label);
                    }
                    ui.end_row();
                    for commodity in Commodity::ALL {
                        let good = &market.goods[commodity.index()];
                        let quote = |side| {
                            self.economy.quote(
                                &ship,
                                commodity,
                                self.market.quantity,
                                side,
                                self.state.revision,
                            )
                        };
                        let buy = quote(TradeSide::Buy);
                        let sell = quote(TradeSide::Sell);
                        ui.label(commodity.label());
                        ui.label(good.stock.to_string());
                        ui.label(ship.cargo[commodity.index()].to_string());
                        ui.label(good.price(TradeSide::Buy, good.stock).to_string());
                        ui.label(good.price(TradeSide::Sell, good.stock).to_string());
                        ui.label(buy.as_ref().map_or("—".into(), |q| q.total.to_string()));
                        ui.label(sell.as_ref().map_or("—".into(), |q| q.total.to_string()));
                        ui.horizontal(|ui| {
                            for (side, label, quote) in [
                                (TradeSide::Buy, "Buy", buy),
                                (TradeSide::Sell, "Sell", sell),
                            ] {
                                let button =
                                    ui.add_enabled(quote.is_ok(), egui::Button::new(label));
                                if button.clicked() {
                                    action = Some(ClientCommand::Trade {
                                        ship: ship.id,
                                        commodity,
                                        quantity: self.market.quantity,
                                        side,
                                        expected_revision: self.state.revision,
                                    });
                                }
                                if let Err(error) = quote {
                                    button.on_disabled_hover_text(error.to_string());
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("Fuel units");
            ui.add(egui::DragValue::new(&mut self.market.fuel_units).range(1..=ship.fuel_capacity));
            let cost = u64::from(self.market.fuel_units) * self.economy.rules.fuel_unit_price.0;
            ui.label(format!(
                "{} cr/unit • total {} cr",
                self.economy.rules.fuel_unit_price,
                game_core::economy::Credits(cost)
            ));
            if ui.button("Refuel").clicked() {
                action = Some(ClientCommand::Refuel {
                    ship: ship.id,
                    units: self.market.fuel_units,
                    expected_revision: self.state.revision,
                });
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "Hold upgrade: +{} cargo for {} cr",
                self.economy.rules.hold_upgrade_capacity, self.economy.rules.hold_upgrade_price
            ));
            if ui
                .add_enabled(!ship.upgraded, egui::Button::new("Upgrade hold"))
                .clicked()
            {
                action = Some(ClientCommand::UpgradeHold {
                    ship: ship.id,
                    expected_revision: self.state.revision,
                });
            }
        });
        if ship.upgraded {
            ui.strong("First earnings goal completed: larger cargo hold purchased.");
        }
        self.known_prices(ui, systems);
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        if let Some(command) = action {
            self.command(command);
            ui.ctx().request_repaint();
        }
    }

    fn known_prices(&self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        egui::CollapsingHeader::new("Known market comparisons").show(ui, |ui| {
            ui.label("Current marginal buy/sell prices. Prices may change while you travel.");
            egui::ScrollArea::horizontal()
                .id_salt("known_prices")
                .show(ui, |ui| {
                    egui::Grid::new("comparison").striped(true).show(ui, |ui| {
                        ui.label("Commodity");
                        for market in &self.economy.markets {
                            ui.strong(
                                systems
                                    .iter()
                                    .find(|s| s.id == market.id.system)
                                    .and_then(|system| {
                                        system.bodies.iter().find(|body| body.id == market.id.body)
                                    })
                                    .map_or("Unknown", |body| body.name.as_str()),
                            );
                        }
                        ui.end_row();
                        for commodity in Commodity::ALL {
                            ui.label(commodity.label());
                            for market in &self.economy.markets {
                                let good = &market.goods[commodity.index()];
                                ui.label(format!(
                                    "{} / {}",
                                    good.price(TradeSide::Buy, good.stock),
                                    good.price(TradeSide::Sell, good.stock)
                                ));
                            }
                            ui.end_row();
                        }
                    });
                });
        });
    }
}
