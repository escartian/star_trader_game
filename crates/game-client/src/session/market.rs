use super::LocalSession;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::{
    StarSystem,
    economy::{Commodity, MarketId, TradeSide},
    species::Species,
};

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
    /// Where the best-margin column looks for a sale.
    pub(super) scope: super::market_scope::TradeScope,
    pub(super) reach: super::market_scope::ReachCache,
    /// The market window's page.
    page: MarketPage,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MarketPage {
    #[default]
    Trade,
    Shipyard,
    Jobs,
}
impl Default for MarketPanel {
    fn default() -> Self {
        Self {
            open: false,
            quantity: 1,
            scope: Default::default(),
            reach: Default::default(),
            page: MarketPage::Trade,
        }
    }
}

impl LocalSession {
    /// Opens the market window on its Shipyard tab (captures).
    #[cfg(any(test, feature = "dev-tools"))]
    pub fn open_shipyard(&mut self) {
        self.market.open = true;
        self.market.page = MarketPage::Shipyard;
    }

    /// Opens the market window on its Jobs tab (captures).
    #[cfg(any(test, feature = "dev-tools"))]
    pub(super) fn open_market_page_jobs(&mut self) {
        self.market.open = true;
        self.market.page = MarketPage::Jobs;
    }

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
        let residents = market.residents_percent();
        let shares = Species::ALL
            .iter()
            .zip(residents)
            .map(|(species, share)| format!("{} {share}%", species.label()))
            .collect::<Vec<_>>()
            .join(" · ");
        ui.small(format!("Population: {shares}"));
        if self.factions_active() {
            let terms = self.economy.port_terms(&ship);
            let color = terms
                .holder
                .map_or(CYAN, |(_, tier)| super::factions::tier_color(tier));
            ui.label(egui::RichText::new(terms.summary()).color(color).small());
            self.wanted_banner(ui, &ship);
        }
        // A port with hulls for sale has a Shipyard tab; hulls are bought
        // only there, where the ship is docked. Every port has a job board.
        let yard = market.ship_stock.iter().any(|stock| *stock > 0);
        if !yard && self.market.page == MarketPage::Shipyard {
            self.market.page = MarketPage::Trade;
        }
        ui.horizontal(|ui| {
            let mut tab = |ui: &mut egui::Ui, icon, label, page| {
                if icons::labeled_toggle(ui, icon, label, self.market.page == page).clicked() {
                    self.market.page = page;
                }
            };
            tab(ui, Icon::Market, "Trade", MarketPage::Trade);
            if yard {
                tab(ui, Icon::Shipyard, "Shipyard", MarketPage::Shipyard);
            }
            tab(ui, Icon::Log, "Jobs", MarketPage::Jobs);
        });
        ui.separator();
        match self.market.page {
            MarketPage::Trade => {}
            MarketPage::Shipyard => {
                // Hulls join or leave the fleet only while it has no course.
                self.pending_course_ui(ui, systems);
                if let Some(command) = self.shipyard_ui(ui) {
                    self.command(command);
                    ui.ctx().request_repaint();
                }
                return;
            }
            MarketPage::Jobs => {
                self.jobs_board_ui(ui, market.id);
                return;
            }
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
            let (hold, ships) = self.fleet_hold(&ship);
            stat(
                ui,
                Icon::Cargo,
                GREEN,
                &format!("{}/{}", hold.cargo_used(), hold.cargo_capacity),
                &if ships > 1 {
                    format!("Cargo used / capacity of the {ships} fleet ships trading here")
                } else {
                    "Cargo used / capacity".to_owned()
                },
            );
            ui.separator();
            if self.load_order_row(ui) {
                self.persist_load_order();
            }
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
        self.charter_row(ui, &ship, market.id);
        ui.horizontal_wrapped(|ui| {
            self.trade_scope_ui(ui, &ship, systems);
            self.best_trade_line(ui, &market, systems);
        });
        // Orders are priced against the pooled hold of the fleet's ships here.
        let (hold, _) = self.fleet_hold(&ship);
        let mut action = self.trade_table(ui, &hold, &market, systems);
        ui.separator();
        let service = self.fleet_services_row(ui, &ship);
        ui.separator();
        if let Some(command) = self.outfitting_row(ui, &ship) {
            action = Some(command);
        }
        self.known_prices(ui, systems);
        if let Some(command) = action {
            self.command(command);
            ui.ctx().request_repaint();
        }
        if let Some(service) = service {
            self.run_fleet_service(ship.id, service);
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
