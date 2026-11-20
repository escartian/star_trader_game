use super::LocalSession;
use crate::theme::icons::{self, Icon};
use game_core::{SystemId, economy::Commodity, travel::Location};

impl LocalSession {
    /// Hides the guide card and remembers that choice; Guide in Session & saves restores it.
    pub fn dismiss_guide(&mut self) {
        self.saves.guide = false;
        let message = match self.persist_preferences() {
            Ok(()) => {
                "Guide hidden. Turn it back on with Guide in Menu, Session & saves.".to_string()
            }
            Err(error) => format!("Guide hidden, but the setting could not be saved: {error}"),
        };
        self.record(message);
    }

    pub fn guide(&mut self, ui: &mut egui::Ui) {
        if !self.saves.guide {
            return;
        }
        let ship = self.selected_ship();
        let fleet = self
            .state
            .fleet_for_ship(ship.id)
            .ok()
            .map(|fleet| fleet.id);
        let hint = if self.state.ships.iter().any(|ship| ship.upgraded) {
            "Goal completed: you purchased the larger cargo hold. Continue exploring trade routes."
        } else if fleet.is_some_and(|fleet| self.state.trade_route(fleet).is_some()) {
            "This fleet runs its trade route on its own: press Advance. It stops only for contacts, or if the route cannot continue."
        } else if matches!(
            ship.location,
            Location::Travelling(_) | Location::InTransit(_)
        ) {
            "This fleet is underway. Press Advance to keep going until it arrives or meets a contact; a contact pauses only this fleet."
        } else if fleet.is_some_and(|fleet| self.state.navigation.order(fleet).is_some()) {
            "Course plotted. Press Advance to set off; a contact along the way needs your decision."
        } else if self.economy.current_market(ship).is_err() {
            "Pick a planet or trade port on the map and plot a course there. Plotting is free; Advance moves fleets and markets together."
        } else if self.economy.credits >= self.economy.rules.hold_upgrade_price {
            "You can afford the hold upgrade. Open Trade (Menu) and buy it to complete the first earnings goal."
        } else if ship.fuel < 20 {
            "Refuel at the current market before planning your next journey."
        } else if ship.cargo[Commodity::Food.index()] > 0 {
            if ship.location == Location::Docked(SystemId(2)) {
                "At Meridian, tap TRADE and sell your food. Compare known prices for your return cargo."
            } else {
                "Select Meridian on the galaxy map, tap its trade port to plot the course; departure and entry are planned for you. Then press Advance. Double-click only inspects a system."
            }
        } else if ship.location == Location::Docked(SystemId(1)) {
            "First delivery: tap TRADE and press Max on food, travel to Meridian, and sell it there with All. Work toward the hold upgrade."
        } else {
            "Compare known markets before buying return cargo. Sell where demand is higher, budget for fuel, and work toward the hold upgrade."
        };
        let mut dismiss = false;
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.strong("Trader introduction / earnings goal");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    dismiss = icons::icon_button(
                        ui,
                        Icon::Cancel,
                        "Hide the guide (turn it back on in Menu, Session & saves)",
                    )
                    .clicked();
                });
            });
            ui.label(hint);
        });
        if dismiss {
            self.dismiss_guide();
        }
    }
}
