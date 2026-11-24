//! The live trade table: prices, the best margin at a port in scope and order buttons.
//! "Max" and "All" size an order to what the economy's quote accepts right now; the
//! authority still validates every trade it receives.
use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::{
    StarSystem,
    economy::{Commodity, Credits, Market, TradeSide},
    travel::Ship,
};
use game_protocol::ClientCommand;

impl LocalSession {
    /// Draws the trade table for `market`; returns the order the player chose.
    pub(super) fn trade_table(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
        market: &Market,
        systems: &[StarSystem],
    ) -> Option<ClientCommand> {
        if ui.available_width() < 620.0 {
            return self.trade_cards(ui, ship, market, systems);
        }
        let mut order = None;
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
                            Icon::Credits,
                            GREEN,
                            "+",
                            "Best margin: buy one unit here and sell it at the best port in the chosen scope.\nFirst-unit prices before fuel; prices change while you travel.",
                        ),
                    ] {
                        stat(ui, icon, color, text, tip);
                    }
                    ui.end_row();
                    for commodity in Commodity::ALL {
                        let good = &market.goods[commodity.index()];
                        ui.horizontal(|ui| {
                            icons::commodity(ui, commodity);
                            ui.label(commodity.label());
                        });
                        ui.label(good.stock.to_string());
                        ui.label(ship.cargo[commodity.index()].to_string());
                        let [buy, sell] = self.your_prices(ship, good);
                        ui.label(buy.to_string());
                        ui.label(sell.to_string());
                        self.margin_cell(ui, market, commodity, systems);
                        ui.horizontal(|ui| {
                            for side in [TradeSide::Buy, TradeSide::Sell] {
                                if let Some(choice) =
                                    self.order_button(ui, ship, commodity, side, false)
                                {
                                    order = Some(choice);
                                }
                            }
                            ui.separator();
                            for side in [TradeSide::Buy, TradeSide::Sell] {
                                if let Some(choice) = self.max_button(ui, ship, commodity, side) {
                                    order = Some(choice);
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        order
    }

    /// A narrow market keeps each quote and its actions in one vertical card.
    fn trade_cards(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
        market: &Market,
        systems: &[StarSystem],
    ) -> Option<ClientCommand> {
        let mut order = None;
        for commodity in Commodity::ALL {
            let good = &market.goods[commodity.index()];
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal_wrapped(|ui| {
                    icons::commodity(ui, commodity);
                    ui.strong(commodity.label());
                    ui.label(format!(
                        "Stock {} · Held {}",
                        good.stock,
                        ship.cargo[commodity.index()]
                    ));
                });
                ui.horizontal_wrapped(|ui| {
                    let [buy, sell] = self.your_prices(ship, good);
                    ui.label(format!("Buy {buy} cr"));
                    ui.label(format!("Sell {sell} cr"));
                });
                self.margin_cell(ui, market, commodity, systems);
                ui.horizontal_wrapped(|ui| {
                    for side in [TradeSide::Buy, TradeSide::Sell] {
                        if let Some(choice) = self.order_button(ui, ship, commodity, side, true) {
                            order = Some(choice);
                        }
                    }
                    for side in [TradeSide::Buy, TradeSide::Sell] {
                        if let Some(choice) = self.max_button(ui, ship, commodity, side) {
                            order = Some(choice);
                        }
                    }
                });
            });
        }
        order
    }

    /// Buys or sells the chosen order quantity; the tooltip prices the whole order.
    fn order_button(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
        commodity: Commodity,
        side: TradeSide,
        labeled: bool,
    ) -> Option<ClientCommand> {
        let quantity = self.market.quantity;
        let (icon, verb) = side_label(side);
        let quote = self
            .economy
            .quote(ship, commodity, quantity, side, self.state.revision);
        let tip = match &quote {
            Ok(quote) => format!(
                "{verb} {quantity} {} for {} cr",
                commodity.label(),
                quote.total
            ),
            Err(_) => format!("{verb} {quantity} {}", commodity.label()),
        };
        let blocked = quote.as_ref().err().map(ToString::to_string);
        let response = if labeled {
            icons::labeled_action(ui, icon, &format!("{verb} {quantity}"), blocked)
                .on_hover_text(&tip)
        } else {
            icons::action(ui, icon, &tip, blocked)
        };
        response
            .clicked()
            .then(|| self.trade_order(ship, commodity, quantity, side))
    }

    /// "Max" buys as much as cargo space, stock and credits allow; "All" sells as
    /// much as is held and the market can pay for and store.
    fn max_button(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
        commodity: Commodity,
        side: TradeSide,
    ) -> Option<ClientCommand> {
        let quantity = self.economy.max_trade_quantity(ship, commodity, side);
        let (icon, verb) = side_label(side);
        let label = match side {
            TradeSide::Buy => "Max",
            TradeSide::Sell => "All",
        };
        // With nothing tradable, a one-unit quote explains why.
        let quote = self
            .economy
            .quote(ship, commodity, quantity.max(1), side, self.state.revision);
        let response = icons::labeled_action(
            ui,
            icon,
            label,
            quote.as_ref().err().map(ToString::to_string),
        );
        let response = match &quote {
            Ok(quote) => response.on_hover_text(format!(
                "{verb} {quantity} {} for {} cr",
                commodity.label(),
                quote.total
            )),
            Err(_) => response,
        };
        (response.clicked() && quantity > 0)
            .then(|| self.trade_order(ship, commodity, quantity, side))
    }

    /// First-unit buy and sell prices on your terms at this port (`factions`).
    fn your_prices(&self, ship: &Ship, good: &game_core::economy::MarketGood) -> [Credits; 2] {
        let terms = self.economy.port_terms(ship);
        [
            Credits(terms.buying(good.price(TradeSide::Buy, good.stock).0)),
            Credits(terms.selling(good.price(TradeSide::Sell, good.stock).0)),
        ]
    }

    /// Best margin for buying one unit here and selling it at a port in scope.
    fn margin_cell(
        &self,
        ui: &mut egui::Ui,
        here: &Market,
        commodity: Commodity,
        systems: &[StarSystem],
    ) {
        match self.best_sale(here, commodity, systems) {
            Some(sale) => {
                let place = super::market_scope::port_and_system(systems, sale.market.id);
                ui.colored_label(GREEN, format!("+{} {place}", Credits(sale.sell.0 - sale.buy.0)))
                    .on_hover_text(format!(
                        "Buy here at {}; sells for {} at {place}.\nFirst-unit prices before fuel; prices change while you travel.",
                        sale.buy, sale.sell
                    ));
            }
            None => {
                ui.weak("—")
                    .on_hover_text("No port in the chosen scope pays more than this buy price");
            }
        }
    }

    fn trade_order(
        &self,
        ship: &Ship,
        commodity: Commodity,
        quantity: u32,
        side: TradeSide,
    ) -> ClientCommand {
        ClientCommand::Trade {
            ship: ship.id,
            commodity,
            quantity,
            side,
            expected_revision: self.state.revision,
        }
    }
}

fn side_label(side: TradeSide) -> (Icon, &'static str) {
    match side {
        TradeSide::Buy => (Icon::Buy, "Buy"),
        TradeSide::Sell => (Icon::Sell, "Sell"),
    }
}
