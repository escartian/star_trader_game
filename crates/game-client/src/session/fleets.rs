//! Fleet management view; all mutations go through authoritative commands.
use super::LocalSession;
use game_core::{economy::Commodity, travel::ShipId};
use game_protocol::ClientCommand;

pub(super) struct FleetPanel {
    pub open: bool,
    selection: Vec<ShipId>,
    commodity: Commodity,
    quantity: u32,
}

impl Default for FleetPanel {
    fn default() -> Self {
        Self {
            open: false,
            selection: Vec::new(),
            commodity: Commodity::Food,
            quantity: 1,
        }
    }
}

impl LocalSession {
    pub fn open_fleets(&mut self) {
        self.fleet_panel.open = true;
    }

    pub fn show_fleets(&mut self, ctx: &egui::Context, systems: &[game_core::StarSystem]) {
        let mut open = self.fleet_panel.open;
        egui::Window::new("Fleets and shipyard")
            .open(&mut open)
            .default_width(620.0)
            .vscroll(true)
            .show(ctx, |ui| self.fleets_ui(ui, systems));
        self.fleet_panel.open = open;
    }

    fn fleets_ui(&mut self, ui: &mut egui::Ui, systems: &[game_core::StarSystem]) {
        let revision = self.state.revision;
        let mut action = None;
        ui.label(format!(
            "{} ships • {} fleets • {} credits",
            self.state.ships.len(),
            self.state.fleets.len(),
            self.economy.credits
        ));
        ui.label("Travel moves the entire selected fleet. Split it to send ships separately.");
        for fleet in self.state.fleets.clone() {
            ui.group(|ui| {
                ui.strong(&fleet.name);
                for id in &fleet.ships {
                    let ship = self
                        .state
                        .ship(*id)
                        .expect("validated fleet membership")
                        .clone();
                    let system_name = |id| {
                        systems
                            .iter()
                            .find(|system| system.id == id)
                            .map_or("Unknown", |system| system.name.as_str())
                    };
                    let location = match &ship.location {
                        game_core::travel::Location::Docked(id) => {
                            format!("At {}", system_name(*id))
                        }
                        game_core::travel::Location::Travelling(journey) => {
                            format!("Travelling to {}", system_name(journey.destination))
                        }
                    };
                    ui.horizontal_wrapped(|ui| {
                        let mut checked = self.fleet_panel.selection.contains(id);
                        if ui.checkbox(&mut checked, "").changed() {
                            self.fleet_panel.selection.retain(|selected| selected != id);
                            if checked {
                                self.fleet_panel.selection.push(*id);
                            }
                        }
                        if ui
                            .selectable_label(self.active_ship == *id, &ship.name)
                            .clicked()
                        {
                            self.select_ship(*id);
                        }
                        ui.label(format!(
                            "{} • fuel {}/{} • cargo {}/{} • {}",
                            ship.kind.label(),
                            ship.fuel,
                            ship.fuel_capacity,
                            ship.cargo_used(),
                            ship.cargo_capacity,
                            location
                        ));
                    });
                }
                let selected: Vec<_> = fleet
                    .ships
                    .iter()
                    .copied()
                    .filter(|id| self.fleet_panel.selection.contains(id))
                    .collect();
                if ui
                    .add_enabled(
                        !selected.is_empty() && selected.len() < fleet.ships.len(),
                        egui::Button::new("Split checked ships into new fleet"),
                    )
                    .clicked()
                {
                    action = Some(ClientCommand::SplitFleet {
                        fleet: fleet.id,
                        ships: selected,
                        expected_revision: revision,
                    });
                }
                for target in &self.state.fleets {
                    if target.id != fleet.id
                        && self
                            .state
                            .fleet_location(fleet.id, fleet.owner)
                            .ok()
                            .zip(self.state.fleet_location(target.id, target.owner).ok())
                            .is_some_and(|(a, b)| a == b)
                        && ui.button(format!("Merge into {}", target.name)).clicked()
                    {
                        action = Some(ClientCommand::MergeFleets {
                            source: fleet.id,
                            target: target.id,
                            expected_revision: revision,
                        });
                    }
                }
            });
        }
        ui.separator();
        let ship = self.selected_ship().clone();
        ui.heading(format!("Shipyard • join {}'s fleet", ship.name));
        for spec in self.economy.rules.ship_catalog.ships.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.label(format!(
                    "{}: {} cr • hold {} • fuel {} • range {} ly",
                    spec.kind.label(),
                    spec.price,
                    spec.cargo_capacity,
                    spec.fuel_capacity,
                    spec.range_ly
                ));
                if ui.button("Buy").clicked() {
                    action = Some(ClientCommand::BuyShip {
                        ship: ship.id,
                        kind: spec.kind,
                        expected_revision: revision,
                    });
                }
            });
        }
        ui.separator();
        ui.heading(format!("Transfer cargo from {}", ship.name));
        egui::ComboBox::from_id_salt("transfer_good")
            .selected_text(self.fleet_panel.commodity.label())
            .show_ui(ui, |ui| {
                for good in Commodity::ALL {
                    ui.selectable_value(&mut self.fleet_panel.commodity, good, good.label());
                }
            });
        ui.add(egui::DragValue::new(&mut self.fleet_panel.quantity).range(1..=10_000));
        for target in &self.state.ships {
            if target.id != ship.id
                && target.location == ship.location
                && ui.button(format!("Transfer to {}", target.name)).clicked()
            {
                action = Some(ClientCommand::TransferCargo {
                    source: ship.id,
                    target: target.id,
                    commodity: self.fleet_panel.commodity,
                    quantity: self.fleet_panel.quantity,
                    expected_revision: revision,
                });
            }
        }
        ui.label(&self.notice);
        if let Some(command) = action {
            self.command(command);
            self.fleet_panel.selection.clear();
        }
    }
}
