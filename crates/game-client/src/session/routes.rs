//! Trade route controls: start a route from the destination card, and show or end
//! a fleet's route on its roster card. The authority runs the route itself.
use super::{LocalSession, market::market_name};
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, GREEN};
use game_core::{BodyId, StarSystem, SystemId, economy::MarketId, fleet::FleetId};
use game_protocol::ClientCommand;

impl LocalSession {
    /// For map-level UI tests, which cannot reach the session state directly.
    #[cfg(test)]
    pub fn trade_route(&self, fleet: FleetId) -> Option<&game_core::routes::TradeRoute> {
        self.state.trade_route(fleet)
    }

    /// "TRADE ROUTE" for a destination with a market: loops the selected fleet
    /// between the market it is at and that destination. Only secondary fleets
    /// run routes; the flagship takes your direct orders, so it has no button.
    pub(super) fn route_button(
        &mut self,
        ui: &mut egui::Ui,
        system: SystemId,
        body: Option<BodyId>,
        systems: &[StarSystem],
    ) {
        let Some(body) = body else {
            return;
        };
        if !self.unlocked(game_core::progress::Feature::TradeRoutes) {
            return;
        }
        let selected = self
            .state
            .fleet_for_ship(self.active_ship)
            .map(|fleet| fleet.id);
        if selected.is_ok_and(|fleet| self.is_flagship(fleet)) {
            return;
        }
        let there = MarketId { system, body };
        if self.economy.market(there).is_err() {
            return;
        }
        let here = self
            .economy
            .current_market(self.selected_ship())
            .ok()
            .map(|market| market.id);
        let blocked = match here {
            None => Some("Start a trade route while the fleet is at a market.".to_string()),
            Some(here) if here == there => {
                Some("Pick a different market as the other end of the route.".to_string())
            }
            Some(_) => None,
        };
        let response = icons::labeled_action(ui, Icon::Transfer, "TRADE ROUTE", blocked);
        let Some(here) = here else {
            return;
        };
        let tip = format!(
            "Loop between {} and {}: at each end, sell, refuel and buy the most profitable cargo for the other end.\nContacts still need your decision; new orders end the route.",
            market_name(systems, here),
            market_name(systems, there)
        );
        if response.on_hover_text(tip).clicked() {
            self.request_trade_route(there);
        }
    }

    /// Starts the selected fleet's route to `destination` and clears the map target.
    pub(super) fn request_trade_route(&mut self, destination: MarketId) {
        let Ok(fleet) = self
            .state
            .fleet_for_ship(self.active_ship)
            .map(|fleet| fleet.id)
        else {
            return;
        };
        self.command(ClientCommand::SetTradeRoute {
            fleet,
            destination,
            expected_revision: self.state.revision,
        });
        self.map_target = None;
    }

    /// The route line on a fleet's roster card, with its End route button.
    pub(super) fn route_line(&mut self, ui: &mut egui::Ui, fleet: FleetId, systems: &[StarSystem]) {
        let Some(route) = self.state.trade_route(fleet).cloned() else {
            return;
        };
        // The route glyph sits between the stops; the UI font has no arrow characters.
        ui.horizontal_wrapped(|ui| {
            ui.small(market_name(systems, route.stops[0]));
            icons::glyph(
                ui,
                Icon::Transfer,
                GREEN,
                "Trade route: loops between these markets",
            );
            ui.small(market_name(systems, route.stops[1]));
        });
        if route.paused {
            ui.colored_label(AMBER, "Paused: nothing profitable to carry.");
            // Resume heads for whichever stop the fleet is not at.
            let here = self
                .state
                .fleets
                .iter()
                .find(|record| record.id == fleet)
                .and_then(|record| record.ships.first())
                .and_then(|id| self.state.ship(*id).ok())
                .and_then(|ship| self.economy.current_market(ship).ok())
                .map(|market| market.id);
            let next = if here == Some(route.stops[1]) {
                route.stops[0]
            } else {
                route.stops[1]
            };
            if icons::button(ui, Icon::Execute, "Resume")
                .on_hover_text("Check prices again and set off if any cargo pays")
                .clicked()
            {
                self.command(ClientCommand::SetTradeRoute {
                    fleet,
                    destination: next,
                    expected_revision: self.state.revision,
                });
            }
        }
        if icons::button(ui, Icon::Cancel, "End route")
            .on_hover_text("The fleet completes its current leg, then waits for orders")
            .clicked()
        {
            self.command(ClientCommand::CancelTradeRoute {
                fleet,
                expected_revision: self.state.revision,
            });
        }
    }
}
