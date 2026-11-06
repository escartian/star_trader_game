use super::LocalSession;
use game_core::{SystemId, economy::Commodity, travel::Location};

impl LocalSession {
    pub fn guide(&self, ui: &mut egui::Ui) {
        if !self.saves.guide {
            return;
        }
        let ship = self.selected_ship();
        let hint = if self.state.ships.iter().any(|ship| ship.upgraded) {
            "Goal completed: you purchased the larger cargo hold. Continue exploring trade routes."
        } else if matches!(
            ship.location,
            Location::Travelling(_) | Location::InTransit(_)
        ) {
            "This fleet is underway. Review its course and any contacts in Navigation, then Execute navigation run to advance all fleets together."
        } else if self.economy.current_market(ship).is_err() {
            "Use Navigation to enter your destination system from deep space, then visit a planet or dock at its trade port. Plotting is free; Execute navigation run advances the world."
        } else if self.economy.credits >= self.economy.rules.hold_upgrade_price {
            "You can afford the hold upgrade. Open Market / refuel and buy it to complete the first earnings goal."
        } else if ship.fuel < 20 {
            "Refuel at the current market before planning your next journey."
        } else if ship.cargo[Commodity::Food.index()] > 0 {
            if ship.location == Location::Docked(SystemId(2)) {
                "At Meridian, open Market / refuel and sell your food. Compare known prices for your return cargo."
            } else {
                "Open Navigation and exit the system, select Meridian and enter it, then inspect its trade port and plot docking to sell. Execute navigation runs to progress each course. Double-click only inspects a system."
            }
        } else if ship.location == Location::Docked(SystemId(1)) {
            "First delivery: open Market / refuel, buy food that fits the selected ship, travel to Meridian, and sell it. Work toward the hold upgrade."
        } else {
            "Compare known markets before buying return cargo. Sell where demand is higher, budget for fuel, and work toward the hold upgrade."
        };
        ui.group(|ui| {
            ui.strong("Trader introduction / earnings goal");
            ui.label(hint);
        });
    }
}
