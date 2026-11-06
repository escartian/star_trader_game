use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
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
        crate::theme::metal::window(ctx, "Market", &mut open, 700.0, |ui| {
            self.market_ui(ui, systems)
        });
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
        ui.horizontal_wrapped(|ui| {
            ui.heading(body.map_or("Unknown market", |body| body.name.as_str()));
            if let Some(profile) = body.and_then(|body| body.planet.as_ref()) {
                icons::planet_traits(ui, profile);
            }
        });
        ui.horizontal_wrapped(|ui| {
            stat(ui, Icon::Funds, AMBER, &market.credits.to_string(), "Market funds available to buy your cargo");
            stat(ui, Icon::Credits, AMBER, &self.economy.credits.to_string(), "Your credits");
            stat(
                ui,
                Icon::Cargo,
                GREEN,
                &format!("{}/{}", ship.cargo_used(), ship.cargo_capacity),
                "Cargo used / capacity",
            );
            ui.separator();
            icons::glyph(ui, Icon::Cargo, CYAN, "Order quantity (units per trade)");
            ui.add(egui::DragValue::new(&mut self.market.quantity).range(1..=2000).prefix("×"))
                .on_hover_text("Order quantity");
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                &format!(
                    "Prices in credits. Unit prices are for the first unit; totals price every unit along the stock curve.\nQuote revision {} • tick {}",
                    self.state.revision, self.state.tick
                ),
            );
        });
        let mut action = None;
        egui::ScrollArea::horizontal()
            .id_salt("trade_table")
            .show(ui, |ui| {
                egui::Grid::new("live_market").striped(true).show(ui, |ui| {
                    ui.label("");
                    for (icon, color, text, tip) in [
                        (Icon::Funds, CYAN, "", "Market stock"),
                        (Icon::Cargo, GREEN, "", "Held in your selected ship"),
                        (Icon::Buy, GREEN, "1", "Buy price for one unit"),
                        (Icon::Sell, AMBER, "1", "Sell price for one unit"),
                        (
                            Icon::Buy,
                            GREEN,
                            "Σ",
                            "Total cost to buy the order quantity",
                        ),
                        (
                            Icon::Sell,
                            AMBER,
                            "Σ",
                            "Total paid to sell the order quantity",
                        ),
                    ] {
                        stat(ui, icon, color, text, tip);
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
                        ui.horizontal(|ui| {
                            icons::commodity(ui, commodity);
                            ui.label(commodity.label());
                        });
                        ui.label(good.stock.to_string());
                        ui.label(ship.cargo[commodity.index()].to_string());
                        ui.label(good.price(TradeSide::Buy, good.stock).to_string());
                        ui.label(good.price(TradeSide::Sell, good.stock).to_string());
                        ui.label(buy.as_ref().map_or("—".into(), |q| q.total.to_string()));
                        ui.label(sell.as_ref().map_or("—".into(), |q| q.total.to_string()));
                        ui.horizontal(|ui| {
                            for (side, icon, verb, quote) in [
                                (TradeSide::Buy, Icon::Buy, "Buy", buy),
                                (TradeSide::Sell, Icon::Sell, "Sell", sell),
                            ] {
                                let tip = format!(
                                    "{verb} {} {}",
                                    self.market.quantity,
                                    commodity.label()
                                );
                                let blocked = quote.as_ref().err().map(ToString::to_string);
                                if icons::action(ui, icon, &tip, blocked).clicked() {
                                    action = Some(ClientCommand::Trade {
                                        ship: ship.id,
                                        commodity,
                                        quantity: self.market.quantity,
                                        side,
                                        expected_revision: self.state.revision,
                                    });
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Fuel, CYAN, "Fuel units to buy");
            ui.add(egui::DragValue::new(&mut self.market.fuel_units).range(1..=ship.fuel_capacity));
            let cost = u64::from(self.market.fuel_units) * self.economy.rules.fuel_unit_price.0;
            stat(
                ui,
                Icon::Credits,
                AMBER,
                &game_core::economy::Credits(cost).to_string(),
                &format!("{} cr per fuel unit", self.economy.rules.fuel_unit_price),
            );
            if icons::button(ui, Icon::Refuel, "Refuel").clicked() {
                action = Some(ClientCommand::Refuel {
                    ship: ship.id,
                    units: self.market.fuel_units,
                    expected_revision: self.state.revision,
                });
            }
            ui.separator();
            let blocked = ship
                .upgraded
                .then(|| "First earnings goal completed: hold already upgraded.".to_string());
            let tip = format!(
                "Upgrade hold: +{} cargo for {} cr",
                self.economy.rules.hold_upgrade_capacity, self.economy.rules.hold_upgrade_price
            );
            if icons::action(ui, Icon::Upgrade, &tip, blocked).clicked() {
                action = Some(ClientCommand::UpgradeHold {
                    ship: ship.id,
                    expected_revision: self.state.revision,
                });
            }
            stat(
                ui,
                Icon::Cargo,
                if ship.upgraded { GREEN } else { AMBER },
                &format!("+{}", self.economy.rules.hold_upgrade_capacity),
                &tip,
            );
        });
        self.known_prices(ui, systems);
        if !self.notice.is_empty() {
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Info, CYAN, "Latest event");
                ui.label(&self.notice);
            });
        }
        if let Some(command) = action {
            self.command(command);
            ui.ctx().request_repaint();
        }
    }

    fn known_prices(&self, ui: &mut egui::Ui, systems: &[StarSystem]) {
        egui::CollapsingHeader::new("Price comparison").show(ui, |ui| {
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                "Current buy / sell unit prices at known markets. Prices may change while you travel.",
            );
            egui::ScrollArea::horizontal()
                .id_salt("known_prices")
                .show(ui, |ui| {
                    egui::Grid::new("comparison").striped(true).show(ui, |ui| {
                        ui.label("");
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
                            icons::commodity(ui, commodity);
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
