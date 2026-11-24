//! Fleet management view; all mutations go through authoritative commands.
use super::LocalSession;
use crate::map::art::ArtKind;
use crate::theme::icons::{self, Icon, stat};
use crate::theme::{AMBER, CYAN, GREEN};
use game_core::{economy::Commodity, travel::ShipId};
use game_protocol::ClientCommand;

pub(super) struct FleetPanel {
    pub open: bool,
    selection: Vec<ShipId>,
    commodity: Commodity,
    quantity: u32,
    rename: String,
}

impl Default for FleetPanel {
    fn default() -> Self {
        Self {
            open: false,
            selection: Vec::new(),
            commodity: Commodity::Food,
            quantity: 1,
            rename: String::new(),
        }
    }
}

/// A player fleet that can trade right now inside the viewed system.
#[derive(Clone, Debug, PartialEq)]
pub struct TradeOption {
    /// First ship in the fleet that has market access; selecting it trades with it.
    pub ship: ShipId,
    pub fleet: String,
    /// The market's body (the trade port when docked).
    pub body: game_core::BodyId,
    /// True when this is the currently selected fleet.
    pub selected: bool,
}

impl LocalSession {
    /// Fleets with market access in `system`; the selected fleet is listed first.
    pub fn trade_options(&self, system: game_core::SystemId) -> Vec<TradeOption> {
        let mut options: Vec<TradeOption> = self
            .state
            .fleets
            .iter()
            .filter_map(|fleet| {
                fleet.ships.iter().find_map(|id| {
                    let ship = self.state.ship(*id).ok()?;
                    let market = self.economy.current_market(ship).ok()?;
                    (market.id.system == system).then(|| TradeOption {
                        ship: ship.id,
                        fleet: fleet.name.clone(),
                        body: market.id.body,
                        selected: fleet.ships.contains(&self.active_ship),
                    })
                })
            })
            .collect();
        options.sort_by_key(|option| !option.selected);
        options
    }

    pub fn show_fleets(&mut self, ctx: &egui::Context, systems: &[game_core::StarSystem]) {
        let mut open = self.fleet_panel.open;
        crate::theme::metal::window(ctx, "Fleets", &mut open, 620.0, |ui| {
            self.fleets_ui(ui, systems)
        });
        self.fleet_panel.open = open;
    }

    fn fleets_ui(&mut self, ui: &mut egui::Ui, systems: &[game_core::StarSystem]) {
        let revision = self.state.revision;
        let mut action = None;
        let artwork = crate::map::art::Artwork::preload(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            stat(ui, Icon::Ship, GREEN, &self.state.ships.len().to_string(), "Ships owned");
            stat(ui, Icon::Fleets, GREEN, &self.state.fleets.len().to_string(), "Fleets");
            stat(ui, Icon::Credits, AMBER, &self.economy.credits.to_string(), "Credits");
            icons::glyph(
                ui,
                Icon::Info,
                CYAN,
                "Tick ships to split them into a new fleet. Co-located fleets can merge.\nCancel plotted courses before changing fleet membership.",
            );
        });
        for fleet in self.state.fleets.clone() {
            ui.group(|ui| {
                ui.strong(&fleet.name);
                for id in &fleet.ships {
                    let ship = self
                        .state
                        .ship(*id)
                        .expect("validated fleet membership")
                        .clone();
                    ui.horizontal_wrapped(|ui| {
                        let mut checked = self.fleet_panel.selection.contains(id);
                        if ui.checkbox(&mut checked, "").changed() {
                            self.fleet_panel.selection.retain(|selected| selected != id);
                            if checked {
                                self.fleet_panel.selection.push(*id);
                            }
                        }
                        let (rect, _) =
                            ui.allocate_exact_size(egui::Vec2::splat(28.0), egui::Sense::hover());
                        artwork.paint(ArtKind::from(ship.kind), ui.painter(), rect.center(), 28.0);
                        if ui
                            .selectable_label(self.active_ship == *id, &ship.name)
                            .on_hover_text(ship.kind.label())
                            .clicked()
                        {
                            self.select_ship(*id);
                        }
                        stat(
                            ui,
                            Icon::Fuel,
                            CYAN,
                            &format!("{}/{}", ship.fuel, ship.fuel_capacity),
                            "Fuel / capacity",
                        );
                        stat(
                            ui,
                            Icon::Cargo,
                            GREEN,
                            &format!("{}/{}", ship.cargo_used(), ship.cargo_capacity),
                            "Cargo used / capacity",
                        );
                        super::navigation::location_stat(ui, &ship.location, systems);
                        if let Some(spec) = self.economy.rules.ship_catalog.spec(ship.kind) {
                            super::shipyard::combat_chips(
                                ui,
                                &spec.combat,
                                &self.economy.rules.combat_catalog,
                                Some(ship.hull_damage),
                            );
                        }
                    });
                }
                let selected: Vec<_> = fleet
                    .ships
                    .iter()
                    .copied()
                    .filter(|id| self.fleet_panel.selection.contains(id))
                    .collect();
                let can_split = !selected.is_empty() && selected.len() < fleet.ships.len();
                let blocked = (!can_split)
                    .then(|| "Tick some, but not all, ships in this fleet first.".to_string());
                if icons::action(
                    ui,
                    Icon::Split,
                    "Split ticked ships into a new fleet",
                    blocked,
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
                            .fleet_position(fleet.id, fleet.owner)
                            .ok()
                            .zip(self.state.fleet_position(target.id, target.owner).ok())
                            .is_some_and(|(a, b)| a == b)
                        && icons::button(ui, Icon::Merge, &target.name)
                            .on_hover_text(format!("Merge into {}", target.name))
                            .clicked()
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
        if let Ok(fleet) = self.state.fleet_for_ship(ship.id) {
            let id = fleet.id;
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Rename, CYAN, &format!("Rename {}", fleet.name));
                ui.add(
                    egui::TextEdit::singleline(&mut self.fleet_panel.rename).hint_text(&fleet.name),
                );
                if icons::icon_button(ui, Icon::Confirm, "Rename fleet").clicked() {
                    action = Some(ClientCommand::RenameFleet {
                        fleet: id,
                        name: self.fleet_panel.rename.clone(),
                        expected_revision: revision,
                    });
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            icons::glyph(ui, Icon::Shipyard, GREEN, "Shipyard");
            ui.weak("Buy and sell hulls in the market's Shipyard tab, at a port with a shipyard.");
        });
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            icons::glyph(
                ui,
                Icon::Transfer,
                GREEN,
                &format!("Transfer cargo from {}", ship.name),
            );
            ui.strong(&ship.name);
            for good in Commodity::ALL {
                let selected = self.fleet_panel.commodity == good;
                let response = icons::toggle(ui, Icon::Good(good), good.label(), selected);
                if response.clicked() {
                    self.fleet_panel.commodity = good;
                }
            }
            ui.add(
                egui::DragValue::new(&mut self.fleet_panel.quantity)
                    .range(1..=10_000)
                    .prefix("×"),
            );
        });
        for target in &self.state.ships {
            if target.id != ship.id
                && target.location == ship.location
                && icons::button(ui, Icon::Transfer, &target.name)
                    .on_hover_text(format!(
                        "Transfer {} {} to {}",
                        self.fleet_panel.quantity,
                        self.fleet_panel.commodity.label(),
                        target.name
                    ))
                    .clicked()
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
        if !self.notice.is_empty() {
            ui.horizontal_wrapped(|ui| {
                icons::glyph(ui, Icon::Info, CYAN, "Latest event");
                ui.label(&self.notice);
            });
        }
        if let Some(command) = action {
            self.command(command);
            self.fleet_panel.selection.clear();
        }
    }
}
