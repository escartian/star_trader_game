//! Where the trade table looks for the best place to sell: ports the selected
//! ship can reach on the fuel aboard, ports in this star system, or every port
//! in the galaxy. Display only; prices are first-unit quotes before fuel.
use super::LocalSession;
use crate::theme::GREEN;
use game_core::{
    StarSystem,
    economy::{Commodity, Credits, Market, MarketId, TradeSide},
    travel::Ship,
};

/// Which ports the "best margin" column and the best-trade line consider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TradeScope {
    /// Ports a course can reach on the selected ship's fuel aboard.
    #[default]
    InRange,
    /// Every other port in the current star system.
    System,
    /// Every other port in the galaxy.
    Galaxy,
}

impl TradeScope {
    const ALL: [Self; 3] = [Self::InRange, Self::System, Self::Galaxy];

    fn label(self) -> &'static str {
        match self {
            Self::InRange => "In fuel range",
            Self::System => "This system",
            Self::Galaxy => "Whole galaxy",
        }
    }
}

/// Ports within the selected ship's fuel range, worked out once per revision
/// and ship because each one plans a full course.
#[derive(Clone, Debug, Default)]
pub(super) struct ReachCache {
    key: Option<(u64, u64)>,
    markets: Vec<MarketId>,
}

/// The best place to sell one unit of a good bought here.
pub(super) struct BestSale<'a> {
    pub market: &'a Market,
    pub buy: Credits,
    pub sell: Credits,
}

impl LocalSession {
    /// The scope selector; also refreshes the fuel-range cache for `ship`.
    pub(super) fn trade_scope_ui(
        &mut self,
        ui: &mut egui::Ui,
        ship: &Ship,
        systems: &[StarSystem],
    ) {
        let scope = &mut self.market.scope;
        egui::ComboBox::from_id_salt("trade_scope")
            .selected_text(scope.label())
            .show_ui(ui, |ui| {
                for option in TradeScope::ALL {
                    ui.selectable_value(scope, option, option.label());
                }
            })
            .response
            .on_hover_text(
                "Where to look for the best sale: ports a course can reach on this ship's \
                 fuel aboard, ports in this star system, or every port in the galaxy.",
            );
        self.refresh_reach(ship, systems);
    }

    fn refresh_reach(&mut self, ship: &Ship, systems: &[StarSystem]) {
        let key = (self.state.revision, ship.id.0);
        if self.market.reach.key == Some(key) {
            return;
        }
        let here = self
            .economy
            .current_market(ship)
            .ok()
            .map(|market| market.id);
        let markets = self
            .economy
            .markets
            .iter()
            .map(|market| market.id)
            .filter(|id| Some(*id) != here)
            .filter(|id| {
                self.state
                    .plan_destination(ship.owner, ship.id, id.system, Some(id.body), systems)
                    .is_ok()
            })
            .collect();
        self.market.reach = ReachCache {
            key: Some(key),
            markets,
        };
    }

    /// Whether `market` is a sale destination under the chosen scope.
    fn in_scope(&self, here: &Market, market: &Market) -> bool {
        market.id != here.id
            && match self.market.scope {
                TradeScope::InRange => self.market.reach.markets.contains(&market.id),
                TradeScope::System => market.id.system == here.id.system,
                TradeScope::Galaxy => true,
            }
    }

    /// The in-scope port paying most for one unit of `commodity` bought here.
    pub(super) fn best_sale(&self, here: &Market, commodity: Commodity) -> Option<BestSale<'_>> {
        let local = &here.goods[commodity.index()];
        let buy = local.price(TradeSide::Buy, local.stock);
        self.economy
            .markets
            .iter()
            .filter(|market| self.in_scope(here, market))
            .map(|market| {
                let good = &market.goods[commodity.index()];
                BestSale {
                    market,
                    buy,
                    sell: good.price(TradeSide::Sell, good.stock),
                }
            })
            .max_by_key(|sale| sale.sell)
            .filter(|sale| sale.sell > sale.buy)
    }

    /// One line naming the single best buy-here, sell-there trade in scope.
    pub(super) fn best_trade_line(&self, ui: &mut egui::Ui, here: &Market, systems: &[StarSystem]) {
        let best = Commodity::ALL
            .into_iter()
            .filter_map(|good| self.best_sale(here, good).map(|sale| (good, sale)))
            .max_by_key(|(_, sale)| sale.sell.0 - sale.buy.0);
        let scope = self.market.scope.label().to_lowercase();
        match best {
            Some((good, sale)) => {
                ui.colored_label(
                    GREEN,
                    format!(
                        "Best trade ({scope}): buy {} here, sell at {} for +{} per unit",
                        good.label(),
                        port_and_system(systems, sale.market.id),
                        Credits(sale.sell.0 - sale.buy.0)
                    ),
                )
                .on_hover_text(format!(
                    "Buy here at {}; sells for {}.\nFirst-unit prices before fuel; prices change while you travel.",
                    sale.buy, sale.sell
                ));
            }
            None => {
                ui.weak(format!("No profitable trade from here ({scope})."));
            }
        }
    }
}

/// "Mars · Sol": the port (planet or station) and its star system.
pub(super) fn port_and_system(systems: &[StarSystem], market: MarketId) -> String {
    let port = super::market::market_name(systems, market);
    let system = systems
        .iter()
        .find(|system| system.id == market.system)
        .map_or("Unknown", |system| system.name.as_str());
    format!("{port} · {system}")
}

#[cfg(test)]
#[path = "market_scope_tests.rs"]
mod tests;
