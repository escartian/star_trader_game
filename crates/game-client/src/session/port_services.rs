//! The Market window's port services: refuel and hull repair, per ship (in
//! each ship's upgrade row) and for every fleet ship docked here at once.
//! Prices and limits come from `game_core::economy` (see `services.rs`).
use super::LocalSession;
use crate::theme::icons::{self, Icon};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::{
    economy::{Credits, EconomyError},
    fleet_trade::{self, LoadOrder},
    travel::{Ship, ShipId},
};
use game_protocol::ClientCommand;

/// A service applied to every fleet ship at the port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FleetService {
    Refuel,
    Repair,
}

impl LocalSession {
    /// Fleet ships docked at `ship`'s port, `ship` first.
    pub(super) fn ships_here(&self, ship: ShipId) -> Vec<ShipId> {
        let mut here = fleet_trade::partners(&self.state, &self.economy, ship, LoadOrder::Even)
            .unwrap_or_else(|_| vec![ship]);
        here.sort_by_key(|id| *id != ship);
        here
    }

    /// Fuel to fill `ship` as far as credits allow, and what it costs.
    fn refuel_quote(&self, ship: &Ship) -> Result<(u32, Credits), EconomyError> {
        let units = self.economy.max_refuel_units(ship)?;
        let listed = u64::from(units).saturating_mul(self.economy.fuel_price().0);
        Ok((
            units,
            Credits(self.economy.port_terms(ship).service(listed)),
        ))
    }

    /// One ship's fuel gauge, Refuel and Repair buttons.
    pub(super) fn ship_service_buttons(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
    ) -> Option<ClientCommand> {
        let mut action = None;
        let color = if ship.fuel < ship.fuel_capacity {
            AMBER
        } else {
            GREEN
        };
        icons::stat(
            ui,
            Icon::Fuel,
            color,
            &format!("{}/{}", ship.fuel, ship.fuel_capacity),
            "Fuel / tank capacity",
        );
        let (label, blocked) = match self.refuel_quote(ship) {
            Ok((units, cost)) => (format!("Refuel {units} · {cost} cr"), None),
            Err(error) => ("Refuel".to_owned(), Some(blocked_reason(&error))),
        };
        if icons::labeled_action(ui, Icon::Refuel, &label, blocked)
            .on_hover_text("Fill this ship's tanks (as far as your credits allow)")
            .clicked()
            && let Ok((units, _)) = self.refuel_quote(ship)
        {
            action = Some(ClientCommand::Refuel {
                ship: ship.id,
                units,
                expected_revision: self.state.revision,
            });
        }
        let (label, blocked) = match self.economy.repair_quote(ship) {
            Ok(cost) => (format!("Repair {} · {cost} cr", ship.hull_damage), None),
            Err(error) => ("Repair".to_owned(), Some(blocked_reason(&error))),
        };
        if icons::labeled_action(ui, Icon::Hull, &label, blocked)
            .on_hover_text("Restore every lost hull point on this ship")
            .clicked()
        {
            action = Some(ClientCommand::RepairShip {
                ship: ship.id,
                expected_revision: self.state.revision,
            });
        }
        action
    }

    /// Refuel-all and repair-all buttons for the fleet ships at this port.
    pub(super) fn fleet_services_row(
        &self,
        ui: &mut egui::Ui,
        ship: &Ship,
    ) -> Option<FleetService> {
        let ships: Vec<&Ship> = self
            .ships_here(ship.id)
            .into_iter()
            .filter_map(|id| self.state.ship(id).ok())
            .collect();
        let mut pick = None;
        ui.horizontal_wrapped(|ui| {
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                "Services for every fleet ship docked here. Each ship's own row below \
                 refuels or repairs just that ship.",
            );
            let fuel: Vec<_> = ships
                .iter()
                .filter_map(|s| self.refuel_quote(s).ok())
                .collect();
            let (units, cost) = fuel
                .iter()
                .fold((0, 0), |(units, cost), (u, c)| (units + u, cost + c.0));
            let blocked = fuel
                .is_empty()
                .then(|| "No ship here can take fuel.".to_owned());
            if icons::labeled_action(ui, Icon::Refuel, "Refuel fleet", blocked)
                .on_hover_text(format!("Buy {units} fuel for {} cr", Credits(cost)))
                .clicked()
            {
                pick = Some(FleetService::Refuel);
            }
            let quotes: Vec<_> = ships
                .iter()
                .filter_map(|s| self.economy.repair_quote(s).ok())
                .collect();
            let cost: u64 = quotes.iter().map(|c| c.0).sum();
            let blocked = quotes
                .is_empty()
                .then(|| "No ship here is damaged.".to_owned());
            if icons::labeled_action(ui, Icon::Hull, "Repair fleet", blocked)
                .on_hover_text(format!(
                    "Repair every damaged ship for {} cr",
                    Credits(cost)
                ))
                .clicked()
            {
                pick = Some(FleetService::Repair);
            }
        });
        pick
    }

    /// Sends `service` for each fleet ship at `lead`'s port, one command at a
    /// time so each carries the revision the previous one produced.
    pub(super) fn run_fleet_service(&mut self, lead: ShipId, service: FleetService) {
        for id in self.ships_here(lead) {
            let Ok(ship) = self.state.ship(id) else {
                continue;
            };
            let command = match service {
                FleetService::Refuel => {
                    self.refuel_quote(ship)
                        .ok()
                        .map(|(units, _)| ClientCommand::Refuel {
                            ship: id,
                            units,
                            expected_revision: self.state.revision,
                        })
                }
                FleetService::Repair => {
                    self.economy
                        .repair_quote(ship)
                        .ok()
                        .map(|_| ClientCommand::RepairShip {
                            ship: id,
                            expected_revision: self.state.revision,
                        })
                }
            };
            if let Some(command) = command {
                self.command(command);
            }
        }
    }
}

fn blocked_reason(error: &EconomyError) -> String {
    match error {
        EconomyError::FuelFull => "The tanks are full.".to_owned(),
        EconomyError::NoDamage => "No hull damage to repair.".to_owned(),
        error => error.to_string(),
    }
}

#[cfg(test)]
#[path = "port_services_tests.rs"]
mod tests;
