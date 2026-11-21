use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::{
    StarSystem,
    economy::{Commodity, MarketId, TradeSide},
};
use game_protocol::ClientCommand;

/// Display name of a market: the body (planet or station) it trades at.
pub(super) fn market_name(systems: &[StarSystem], market: MarketId) -> &str {
    systems
        .iter()
        .find(|system| system.id == market.system)
        .and_then(|system| system.bodies.iter().find(|body| body.id == market.body))
        .map_or("Unknown", |body| body.name.as_str())
}

pub(super) struct MarketPanel {
    pub open: bool,
    /// Units per Buy/Sell order button press.
    pub(super) quantity: u32,
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
        if self.factions_active() {
            let terms = self.economy.port_terms(&ship);
            let color = terms
                .holder
                .map_or(CYAN, |(_, tier)| super::factions::tier_color(tier));
            ui.label(egui::RichText::new(terms.summary()).color(color).small());
            self.wanted_banner(ui, &ship);
        }
        ui.horizontal_wrapped(|ui| {
            stat(ui, Icon::Funds, AMBER, &market.credits.to_string(), "Market funds available to buy your cargo");
            stat(ui, Icon::Credits, AMBER, &self.economy.credits.to_string(), "Your credits");
            let flow = self.economy.market_flow(&market);
            let trend = match flow.net() {
                net if net > 0 => "growing",
                net if net < 0 => "shrinking",
                _ => "steady",
            };
            stat(
                ui,
                Icon::Overview,
                CYAN,
                &format!("Economy {trend}"),
                &format!(
                    "Per tick this port earns {} cr from its own industry and {} cr from \
                     recent trade, and spends {} cr. Trade here makes it richer; quiet \
                     ports fall back to what their industry supports. Prices stand at \
                     {:.1}% of a new game's (inflation).",
                    flow.local_income,
                    flow.trade_income,
                    flow.upkeep,
                    self.economy.price_level_ppm as f64 / 10_000.0
                ),
            );
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
                "Prices in credits for the first unit. Hover an order button for its total:\nbulk orders price every unit along the stock curve. Max fills your hold;\nAll sells everything this market can pay for.",
            );
        });
        if self.unlocked(game_core::progress::Feature::Victory) {
            self.charter_row(ui, &ship, market.id);
        }
        let mut action = self.trade_table(ui, &ship, &market, systems);
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            // Orders never exceed the space left in the tanks.
            let missing = ship.fuel_capacity.saturating_sub(ship.fuel);
            let units = self.market.fuel_units.clamp(1, missing.max(1));
            icons::glyph(ui, Icon::Fuel, CYAN, "Fuel units to buy");
            ui.add_enabled(
                missing > 0,
                egui::DragValue::new(&mut self.market.fuel_units).range(1..=missing.max(1)),
            );
            let terms = self.economy.port_terms(&ship);
            let cost = terms.service(u64::from(units) * self.economy.fuel_price().0);
            stat(
                ui,
                Icon::Credits,
                AMBER,
                &game_core::economy::Credits(cost).to_string(),
                &format!(
                    "{} cr per fuel unit at list price",
                    self.economy.fuel_price()
                ),
            );
            let blocked = (missing == 0).then(|| "The tanks are full.".to_owned());
            if icons::labeled_action(ui, Icon::Refuel, "Refuel", blocked)
                .on_hover_text(format!("Buy {units} fuel ({missing} fits)"))
                .clicked()
            {
                action = Some(ClientCommand::Refuel {
                    ship: ship.id,
                    units,
                    expected_revision: self.state.revision,
                });
            }
            let full = self.economy.max_refuel_units(&ship);
            let reason = match &full {
                Ok(_) => None,
                Err(game_core::economy::EconomyError::FuelFull) => {
                    Some("The tanks are full.".to_owned())
                }
                Err(error) => Some(error.to_string()),
            };
            let response = icons::labeled_action(ui, Icon::Refuel, "Full", reason);
            if let Ok(units) = full {
                let cost =
                    terms.service(u64::from(units).saturating_mul(self.economy.fuel_price().0));
                if response
                    .on_hover_text(format!(
                        "Refuel {units} units for {} cr",
                        game_core::economy::Credits(cost)
                    ))
                    .clicked()
                {
                    action = Some(ClientCommand::Refuel {
                        ship: ship.id,
                        units,
                        expected_revision: self.state.revision,
                    });
                }
            }
            ui.separator();
            let blocked = ship
                .upgraded
                .then(|| "First earnings goal completed: hold already upgraded.".to_string());
            let tip = format!(
                "Upgrade hold: +{} cargo for {} cr",
                self.economy.rules.hold_upgrade_capacity,
                self.economy.hold_upgrade_price()
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
            ui.separator();
            let (blocked, tip) = match self.economy.repair_quote(&ship) {
                Ok(cost) => (
                    None,
                    format!("Repair {} hull points for {cost} cr", ship.hull_damage),
                ),
                Err(error) => (Some(error.to_string()), "Repair hull".to_string()),
            };
            if icons::action(ui, Icon::Hull, &tip, blocked).clicked() {
                action = Some(ClientCommand::RepairShip {
                    ship: ship.id,
                    expected_revision: self.state.revision,
                });
            }
        });
        ui.separator();
        if let Some(command) = self.outfitting_row(ui, &ship) {
            action = Some(command);
        }
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
            if ui.available_width() < 620.0 {
                for commodity in Commodity::ALL {
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            icons::commodity(ui, commodity);
                            ui.strong(commodity.label());
                        });
                        for market in &self.economy.markets {
                            let good = &market.goods[commodity.index()];
                            ui.horizontal_wrapped(|ui| {
                                ui.label(market_name(systems, market.id));
                                ui.weak(format!(
                                    "Buy {} · Sell {} cr",
                                    good.price(TradeSide::Buy, good.stock),
                                    good.price(TradeSide::Sell, good.stock)
                                ));
                            });
                        }
                    });
                }
                return;
            }
            egui::ScrollArea::horizontal()
                .id_salt("known_prices")
                .show(ui, |ui| {
                    egui::Grid::new("comparison").striped(true).show(ui, |ui| {
                        ui.label("");
                        for market in &self.economy.markets {
                            ui.strong(market_name(systems, market.id));
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
