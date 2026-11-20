//! The live trade table: prices, the best known margin elsewhere and order buttons.
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
                            "Best known margin: buy one unit here and sell it at another market.\nFirst-unit prices before fuel; prices change while you travel.",
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
                        ui.label(good.price(TradeSide::Buy, good.stock).to_string());
                        ui.label(good.price(TradeSide::Sell, good.stock).to_string());
                        self.margin_cell(ui, market, commodity, systems);
                        ui.horizontal(|ui| {
                            for side in [TradeSide::Buy, TradeSide::Sell] {
                                if let Some(choice) = self.order_button(ui, ship, commodity, side) {
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

    /// Buys or sells the chosen order quantity; the tooltip prices the whole order.
    fn order_button(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
        commodity: Commodity,
        side: TradeSide,
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
        icons::action(ui, icon, &tip, blocked)
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

    /// Best known margin for buying one unit here and selling it at another market.
    fn margin_cell(
        &self,
        ui: &mut egui::Ui,
        here: &Market,
        commodity: Commodity,
        systems: &[StarSystem],
    ) {
        let local = &here.goods[commodity.index()];
        let buy = local.price(TradeSide::Buy, local.stock);
        let best = self
            .economy
            .markets
            .iter()
            .filter(|market| market.id != here.id)
            .map(|market| {
                let good = &market.goods[commodity.index()];
                (market, good.price(TradeSide::Sell, good.stock))
            })
            .max_by_key(|(_, sell)| *sell);
        match best {
            Some((market, sell)) if sell > buy => {
                let name = super::market::market_name(systems, market.id);
                // Another system is named by its system; a market nearby by its body.
                let place = if market.id.system == here.id.system {
                    name
                } else {
                    systems
                        .iter()
                        .find(|system| system.id == market.id.system)
                        .map_or("Unknown", |system| system.name.as_str())
                };
                ui.colored_label(GREEN, format!("+{} {place}", Credits(sell.0 - buy.0)))
                    .on_hover_text(format!(
                        "Buy here at {buy}; sells for {sell} at {name}.\nFirst-unit prices before fuel; prices change while you travel."
                    ));
            }
            _ => {
                ui.weak("—")
                    .on_hover_text("No known market pays more than this buy price");
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
